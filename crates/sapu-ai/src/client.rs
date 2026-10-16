//! Runs tasks against the configured provider with anonymisation, preview
//! and caching.

use std::path::PathBuf;
use std::sync::Mutex;

use crate::anonymize::Anonymizer;
use crate::cache::AnswerCache;
use crate::config::AiConfig;
use crate::provider::{Provider, from_config};
use crate::tasks::Task;
use crate::{AiError, Result};

/// Exactly what would leave the device for a request.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Preview {
    pub provider: String,
    pub model: String,
    pub system: String,
    pub user: String,
}

pub struct AiClient {
    provider: Box<dyn Provider>,
    anonymizer: Anonymizer,
    cache: Mutex<AnswerCache>,
    cache_path: Option<PathBuf>,
}

impl AiClient {
    /// Builds from config; `api_key` from the keychain when needed.
    pub fn new(cfg: &AiConfig, api_key: Option<String>) -> Result<Self> {
        if !cfg.is_enabled() {
            return Err(AiError::Disabled);
        }
        let provider = from_config(cfg, api_key)?;
        let cache_path = AnswerCache::default_path();
        let cache = cache_path
            .as_deref()
            .map(AnswerCache::load)
            .unwrap_or_default();
        Ok(Self {
            provider,
            anonymizer: Anonymizer::new(cfg.anonymize),
            cache: Mutex::new(cache),
            cache_path,
        })
    }

    /// For tests and embedding: explicit provider, no cache file.
    pub fn with_provider(provider: Box<dyn Provider>, anonymizer: Anonymizer) -> Self {
        Self {
            provider,
            anonymizer,
            cache: Mutex::new(AnswerCache::default()),
            cache_path: None,
        }
    }

    pub fn anonymizer(&self) -> &Anonymizer {
        &self.anonymizer
    }

    pub fn model(&self) -> String {
        self.provider.model().to_string()
    }

    fn sanitized<T: Task>(&self, input: &T::Input) -> Result<serde_json::Value> {
        let mut v = serde_json::to_value(input)?;
        sanitize(&mut v, &self.anonymizer);
        Ok(v)
    }

    /// The payload that `run` would send, without sending it.
    pub fn preview<T: Task>(&self, input: &T::Input) -> Result<Preview> {
        let v = self.sanitized::<T>(input)?;
        Ok(Preview {
            provider: self.provider.name(),
            model: self.provider.model().to_string(),
            system: T::system_prompt().to_string(),
            user: T::user_prompt(&v),
        })
    }

    pub fn run<T: Task>(&self, input: &T::Input) -> Result<T::Output> {
        let v = self.sanitized::<T>(input)?;
        let key = AnswerCache::key(T::NAME, self.provider.model(), &v);
        if let Ok(c) = self.cache.lock()
            && let Some(hit) = c.get(&key)
            && let Ok(out) = serde_json::from_value::<T::Output>(hit.clone())
        {
            return Ok(out);
        }
        tracing::info!(task = T::NAME, provider = %self.provider.name(), "ai request");
        let answer = self
            .provider
            .complete(T::system_prompt(), &T::user_prompt(&v))?;
        let out = T::parse(&answer)?;
        if let Ok(mut c) = self.cache.lock() {
            c.put(key, serde_json::to_value(&out)?);
            if let Some(p) = &self.cache_path
                && let Err(e) = c.save(p)
            {
                tracing::warn!(error = %e, "could not save ai cache");
            }
        }
        Ok(out)
    }

    /// Quick connectivity check: a trivial completion.
    pub fn ping(&self) -> Result<String> {
        let answer = self.provider.complete(
            "Reply with the JSON {\"ok\": true} and nothing else.",
            "{\"ping\": 1}",
        )?;
        let _ = crate::tasks::extract_json(&answer)?;
        Ok(self.provider.model().to_string())
    }
}

/// Anonymise every string in the input tree.
fn sanitize(v: &mut serde_json::Value, a: &Anonymizer) {
    match v {
        serde_json::Value::String(s) => *s = a.text(s),
        serde_json::Value::Array(items) => items.iter_mut().for_each(|i| sanitize(i, a)),
        serde_json::Value::Object(map) => map.values_mut().for_each(|i| sanitize(i, a)),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::ProviderError;
    use crate::tasks::whatis::{WhatIs, WhatIsInput};
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct Fake {
        calls: AtomicUsize,
        answer: &'static str,
    }

    impl Provider for Fake {
        fn name(&self) -> String {
            "fake".into()
        }
        fn model(&self) -> &str {
            "fake-1"
        }
        fn complete(&self, _s: &str, user: &str) -> std::result::Result<String, ProviderError> {
            self.calls.fetch_add(1, Ordering::Relaxed);
            assert!(
                !user.contains("/Users/budi"),
                "home must be anonymised: {user}"
            );
            Ok(self.answer.into())
        }
    }

    fn input() -> WhatIsInput {
        WhatIsInput {
            language: "id".into(),
            os: "macos".into(),
            path: "/Users/budi/Library/Caches/com.spotify.client".into(),
            is_dir: true,
            bytes: 1 << 30,
            file_count: 1200,
            top_extensions: vec![("bin".into(), 1100)],
            newest_age_days: Some(2),
            oldest_age_days: Some(400),
        }
    }

    #[test]
    fn anonymises_previews_and_caches() {
        let fake = Box::leak(Box::new(Fake {
            calls: AtomicUsize::new(0),
            answer: r#"{"summary":"Cache Spotify","likely_owner":"Spotify","removable":"yes","consequence":"unduh ulang","confidence":0.8,"reason":"Caches"}"#,
        }));
        let fake_ref: &'static Fake = fake;
        struct Shim(&'static Fake);
        impl Provider for Shim {
            fn name(&self) -> String {
                self.0.name()
            }
            fn model(&self) -> &str {
                self.0.model()
            }
            fn complete(&self, s: &str, u: &str) -> std::result::Result<String, ProviderError> {
                self.0.complete(s, u)
            }
        }
        let client = AiClient::with_provider(
            Box::new(Shim(fake_ref)),
            Anonymizer::with(Some("/Users/budi".into()), Some("budi".into()), true),
        );
        let p = client.preview::<WhatIs>(&input()).expect("preview");
        assert!(p.user.contains("~/Library/Caches"));
        assert!(!p.user.contains("budi"));

        let a = client.run::<WhatIs>(&input()).expect("run");
        assert_eq!(a.summary, "Cache Spotify");
        let _ = client.run::<WhatIs>(&input()).expect("run again");
        assert_eq!(
            fake_ref.calls.load(Ordering::Relaxed),
            1,
            "second call served from cache"
        );
    }
}
