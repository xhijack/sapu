//! Chat providers. One method: system + user prompt in, text out.

use std::time::Duration;

use crate::config::{AiConfig, AiMode, CloudKind};

#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("cannot reach {url}: {message}")]
    Unreachable { url: String, message: String },
    #[error("provider answered {status}: {message}")]
    Status { status: u16, message: String },
    #[error("no model configured and none installed")]
    NoModel,
    #[error("API key missing for {0}")]
    NoApiKey(String),
    #[error("malformed response: {0}")]
    Malformed(String),
}

pub trait Provider: Send + Sync {
    fn name(&self) -> String;
    fn model(&self) -> &str;
    fn complete(&self, system: &str, user: &str) -> Result<String, ProviderError>;
}

fn agent(timeout: Duration) -> ureq::Agent {
    ureq::Agent::new_with_config(
        ureq::Agent::config_builder()
            .timeout_global(Some(timeout))
            // 4xx/5xx come back as responses so the body's error text can
            // be shown ("invalid x-api-key", "model not found").
            .http_status_as_error(false)
            .build(),
    )
}

/// Turns a response into JSON, or into a `Status` error carrying the
/// provider's own explanation.
fn into_json(
    mut resp: ureq::http::Response<ureq::Body>,
) -> Result<serde_json::Value, ProviderError> {
    let status = resp.status().as_u16();
    if status >= 400 {
        let text = resp.body_mut().read_to_string().unwrap_or_default();
        let message = serde_json::from_str::<serde_json::Value>(&text)
            .ok()
            .and_then(|v| {
                v["error"]["message"]
                    .as_str()
                    .or_else(|| v["error"].as_str())
                    .or_else(|| v["message"].as_str())
                    .map(str::to_string)
            })
            .unwrap_or_else(|| text.chars().take(200).collect());
        return Err(ProviderError::Status { status, message });
    }
    resp.body_mut()
        .read_json::<serde_json::Value>()
        .map_err(|e| ProviderError::Malformed(e.to_string()))
}

fn post_json(
    agent: &ureq::Agent,
    url: &str,
    headers: &[(&str, &str)],
    body: &serde_json::Value,
) -> Result<serde_json::Value, ProviderError> {
    let mut req = agent.post(url);
    for (k, v) in headers {
        req = req.header(*k, *v);
    }
    match req.send_json(body) {
        Ok(resp) => into_json(resp),
        Err(e) => Err(ProviderError::Unreachable {
            url: url.to_string(),
            message: e.to_string(),
        }),
    }
}

fn get_json(
    agent: &ureq::Agent,
    url: &str,
    headers: &[(&str, &str)],
) -> Result<serde_json::Value, ProviderError> {
    let mut req = agent.get(url);
    for (k, v) in headers {
        req = req.header(*k, *v);
    }
    match req.call() {
        Ok(resp) => into_json(resp),
        Err(e) => Err(ProviderError::Unreachable {
            url: url.to_string(),
            message: e.to_string(),
        }),
    }
}

/// Ollama's native API.
pub struct Ollama {
    base: String,
    model: String,
    agent: ureq::Agent,
}

impl Ollama {
    pub fn new(base: &str, model: &str, timeout: Duration) -> Self {
        Self {
            base: base.trim_end_matches('/').to_string(),
            model: model.to_string(),
            agent: agent(timeout),
        }
    }

    /// Installed models, newest first as Ollama reports them.
    pub fn list_models(base: &str, timeout: Duration) -> Result<Vec<String>, ProviderError> {
        let v = get_json(
            &agent(timeout),
            &format!("{}/api/tags", base.trim_end_matches('/')),
            &[],
        )?;
        Ok(v["models"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|m| m["name"].as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default())
    }
}

impl Provider for Ollama {
    fn name(&self) -> String {
        "ollama".into()
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn complete(&self, system: &str, user: &str) -> Result<String, ProviderError> {
        let body = serde_json::json!({
            "model": self.model,
            "stream": false,
            "format": "json",
            "options": {"temperature": 0.2},
            "messages": [
                {"role": "system", "content": system},
                {"role": "user", "content": user}
            ]
        });
        let v = post_json(&self.agent, &format!("{}/api/chat", self.base), &[], &body)?;
        v["message"]["content"]
            .as_str()
            .map(str::to_string)
            .ok_or_else(|| ProviderError::Malformed("missing message.content".into()))
    }
}

/// Joins a provider base with an OpenAI-style path. Bases that already end
/// in a version segment (`…/v1`, Gemini's `…/v1beta/openai`) are used as
/// is; bare hosts get `/v1`.
fn openai_url(base: &str, path: &str) -> String {
    let base = base.trim_end_matches('/');
    let versioned = base.ends_with("/v1") || base.contains("/openai") || base.ends_with("/v1beta");
    if versioned {
        format!("{base}{path}")
    } else {
        format!("{base}/v1{path}")
    }
}

fn ids_of(v: &serde_json::Value) -> Vec<String> {
    let mut ids: Vec<String> = v["data"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|m| m["id"].as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    ids.sort();
    ids
}

/// Any `/v1/chat/completions` server (OpenAI, Groq, OpenRouter, LM Studio, ...).
pub struct OpenAiCompatible {
    base: String,
    model: String,
    api_key: Option<String>,
    agent: ureq::Agent,
}

impl OpenAiCompatible {
    pub fn new(base: &str, model: &str, api_key: Option<String>, timeout: Duration) -> Self {
        Self {
            base: base.trim_end_matches('/').to_string(),
            model: model.to_string(),
            api_key,
            agent: agent(timeout),
        }
    }

    /// Model ids the server offers (`GET /v1/models`), sorted.
    pub fn list_models(
        base: &str,
        api_key: Option<&str>,
        timeout: Duration,
    ) -> Result<Vec<String>, ProviderError> {
        let auth = api_key.map(|k| format!("Bearer {k}"));
        let headers: Vec<(&str, &str)> = auth
            .as_deref()
            .map(|a| vec![("Authorization", a)])
            .unwrap_or_default();
        let v = get_json(&agent(timeout), &openai_url(base, "/models"), &headers)?;
        Ok(ids_of(&v))
    }
}

impl Provider for OpenAiCompatible {
    fn name(&self) -> String {
        "openai_compatible".into()
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn complete(&self, system: &str, user: &str) -> Result<String, ProviderError> {
        // No sampling parameters: newer models (gpt-5, claude-*-5) reject
        // `temperature`, and the JSON-only prompt is what keeps answers tight.
        let body = serde_json::json!({
            "model": self.model,
            "response_format": {"type": "json_object"},
            "messages": [
                {"role": "system", "content": system},
                {"role": "user", "content": user}
            ]
        });
        let auth = self.api_key.as_ref().map(|k| format!("Bearer {k}"));
        let headers: Vec<(&str, &str)> = auth
            .as_deref()
            .map(|a| vec![("Authorization", a)])
            .unwrap_or_default();
        let v = post_json(
            &self.agent,
            &openai_url(&self.base, "/chat/completions"),
            &headers,
            &body,
        )?;
        v["choices"][0]["message"]["content"]
            .as_str()
            .map(str::to_string)
            .ok_or_else(|| ProviderError::Malformed("missing choices[0].message.content".into()))
    }
}

/// Anthropic Messages API.
pub struct Anthropic {
    base: String,
    model: String,
    api_key: String,
    agent: ureq::Agent,
}

impl Anthropic {
    pub fn new(base: &str, model: &str, api_key: String, timeout: Duration) -> Self {
        Self {
            base: base.trim_end_matches('/').to_string(),
            model: model.to_string(),
            api_key,
            agent: agent(timeout),
        }
    }

    /// Model ids the account can use (`GET /v1/models`), sorted.
    pub fn list_models(
        base: &str,
        api_key: &str,
        timeout: Duration,
    ) -> Result<Vec<String>, ProviderError> {
        let v = get_json(
            &agent(timeout),
            &format!("{}/v1/models?limit=100", base.trim_end_matches('/')),
            &[("x-api-key", api_key), ("anthropic-version", "2023-06-01")],
        )?;
        Ok(ids_of(&v))
    }
}

impl Provider for Anthropic {
    fn name(&self) -> String {
        "anthropic".into()
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn complete(&self, system: &str, user: &str) -> Result<String, ProviderError> {
        let body = serde_json::json!({
            "model": self.model,
            "max_tokens": 1024,
            "system": system,
            "messages": [{"role": "user", "content": user}]
        });
        let v = post_json(
            &self.agent,
            &format!("{}/v1/messages", self.base),
            &[
                ("x-api-key", self.api_key.as_str()),
                ("anthropic-version", "2023-06-01"),
            ],
            &body,
        )?;
        v["content"][0]["text"]
            .as_str()
            .map(str::to_string)
            .ok_or_else(|| ProviderError::Malformed("missing content[0].text".into()))
    }
}

pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(60);

/// Build the provider the configuration points at. `api_key` comes from the keychain.
pub fn from_config(
    cfg: &AiConfig,
    api_key: Option<String>,
) -> Result<Box<dyn Provider>, ProviderError> {
    match cfg.mode {
        AiMode::Off => Err(ProviderError::NoModel),
        AiMode::Local => {
            let model = if cfg.local_model.is_empty() {
                Ollama::list_models(&cfg.local_url, Duration::from_secs(5))?
                    .into_iter()
                    .next()
                    .ok_or(ProviderError::NoModel)?
            } else {
                cfg.local_model.clone()
            };
            Ok(Box::new(Ollama::new(
                &cfg.local_url,
                &model,
                DEFAULT_TIMEOUT,
            )))
        }
        AiMode::Cloud => {
            if cfg.cloud_model.is_empty() {
                return Err(ProviderError::NoModel);
            }
            match cfg.cloud_kind {
                CloudKind::OpenAiCompatible => Ok(Box::new(OpenAiCompatible::new(
                    &cfg.cloud_url,
                    &cfg.cloud_model,
                    api_key,
                    DEFAULT_TIMEOUT,
                ))),
                CloudKind::Anthropic => {
                    let key = api_key.ok_or_else(|| ProviderError::NoApiKey("anthropic".into()))?;
                    Ok(Box::new(Anthropic::new(
                        &cfg.cloud_url,
                        &cfg.cloud_model,
                        key,
                        DEFAULT_TIMEOUT,
                    )))
                }
            }
        }
    }
}

#[cfg(test)]
pub(crate) mod test_server {
    use std::io::{Read, Write};
    use std::net::TcpListener;

    /// One-shot HTTP server answering `body` with 200; returns (base url, captured request).
    pub fn serve(body: &'static str) -> (String, std::sync::mpsc::Receiver<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr");
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let (mut s, _) = listener.accept().expect("accept");
            // Read until the body is complete (Content-Length or chunked
            // terminator), or until the client goes quiet.
            s.set_read_timeout(Some(std::time::Duration::from_millis(400)))
                .expect("timeout");
            let mut buf = vec![0u8; 65536];
            let mut read = 0;
            loop {
                let n = match s.read(&mut buf[read..]) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => n,
                };
                read += n;
                let text = String::from_utf8_lossy(&buf[..read]).into_owned();
                if let Some(idx) = text.find("\r\n\r\n") {
                    let len: Option<usize> = text.lines().find_map(|l| {
                        l.to_ascii_lowercase()
                            .strip_prefix("content-length:")
                            .map(|v| v.trim().parse().unwrap_or(0))
                    });
                    let done = match len {
                        Some(len) => read >= idx + 4 + len,
                        None => text.ends_with("0\r\n\r\n"),
                    };
                    if done {
                        break;
                    }
                }
            }
            let _ = tx.send(String::from_utf8_lossy(&buf[..read]).into_owned());
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            let _ = s.write_all(resp.as_bytes());
        });
        (format!("http://{addr}"), rx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ollama_chat_and_tags() {
        let (url, rx) =
            test_server::serve(r#"{"message":{"role":"assistant","content":"{\"ok\":true}"}}"#);
        let p = Ollama::new(&url, "llama3", Duration::from_secs(5));
        assert_eq!(p.complete("sys", "usr").expect("complete"), "{\"ok\":true}");
        let req = rx.recv().expect("req");
        assert!(req.starts_with("POST /api/chat"));
        let compact: String = req.chars().filter(|c| !c.is_whitespace()).collect();
        assert!(
            compact.contains("\"format\":\"json\""),
            "captured request: {req}"
        );

        let (url, _rx) =
            test_server::serve(r#"{"models":[{"name":"qwen2.5:3b"},{"name":"llama3.2"}]}"#);
        assert_eq!(
            Ollama::list_models(&url, Duration::from_secs(5)).expect("tags"),
            vec!["qwen2.5:3b", "llama3.2"]
        );
    }

    #[test]
    fn openai_compatible_sends_bearer() {
        let (url, rx) = test_server::serve(r#"{"choices":[{"message":{"content":"{\"a\":1}"}}]}"#);
        let p = OpenAiCompatible::new(
            &url,
            "gpt-x",
            Some("sk-test".into()),
            Duration::from_secs(5),
        );
        assert_eq!(p.complete("s", "u").expect("complete"), "{\"a\":1}");
        let req = rx.recv().expect("req");
        assert!(req.starts_with("POST /v1/chat/completions"));
        assert!(
            req.to_ascii_lowercase()
                .contains("authorization: bearer sk-test")
        );
    }

    #[test]
    fn openai_url_respects_versioned_bases() {
        assert_eq!(
            openai_url("https://api.openai.com", "/models"),
            "https://api.openai.com/v1/models"
        );
        assert_eq!(
            openai_url("https://api.groq.com/openai/v1/", "/models"),
            "https://api.groq.com/openai/v1/models"
        );
        assert_eq!(
            openai_url(
                "https://generativelanguage.googleapis.com/v1beta/openai",
                "/chat/completions"
            ),
            "https://generativelanguage.googleapis.com/v1beta/openai/chat/completions"
        );
    }

    #[test]
    fn anthropic_headers_and_parse() {
        let (url, rx) = test_server::serve(r#"{"content":[{"type":"text","text":"{\"b\":2}"}]}"#);
        let p = Anthropic::new(&url, "claude-x", "key1".into(), Duration::from_secs(5));
        assert_eq!(p.complete("s", "u").expect("complete"), "{\"b\":2}");
        let req = rx.recv().expect("req");
        assert!(req.starts_with("POST /v1/messages"));
        assert!(req.to_ascii_lowercase().contains("x-api-key: key1"));
        assert!(req.contains("anthropic-version"));
    }

    #[test]
    fn unreachable_is_reported() {
        let p = Ollama::new("http://127.0.0.1:1", "m", Duration::from_secs(2));
        assert!(matches!(
            p.complete("s", "u"),
            Err(ProviderError::Unreachable { .. })
        ));
        let cfg = AiConfig::default();
        assert!(matches!(
            from_config(&cfg, None),
            Err(ProviderError::NoModel)
        ));
    }
}
