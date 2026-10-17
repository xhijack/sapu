//! AI commands. The app is the only bridge between core data and the
//! `sapu-ai` crate: it extracts metadata, never contents, and shows the
//! payload on request. Nothing here can remove files.

use std::path::PathBuf;
use std::sync::Arc;

use sapu_ai::config::{AiConfig, AiMode, CloudKind};
use sapu_ai::tasks::artefact::{Artefact, ArtefactAnswer, ArtefactInput, ArtefactSignal};
use sapu_ai::tasks::report::{Report, ReportAnswer, ReportInput};
use sapu_ai::tasks::search::{Search, SearchFilter, SearchInput};
use sapu_ai::tasks::whatis::{WhatIs, WhatIsAnswer, WhatIsInput};
use sapu_ai::{AiClient, AiError, Preview};
use sapu_core::scanner::{AgeFilter, Category, FileEntry, FileFilter};
use sapu_core::settings::Settings;
use tauri::State;

use crate::{CmdResult, CommandError};

impl From<AiError> for CommandError {
    fn from(e: AiError) -> Self {
        let code = match &e {
            AiError::Disabled => "ai_disabled",
            AiError::Provider(_) => "ai_provider",
            AiError::BadAnswer(_) => "ai_bad_answer",
            AiError::Keychain(_) => "ai_keychain",
            AiError::Io(_) => "io",
            AiError::Serde(_) => "serde",
        };
        CommandError::new(code, e.to_string())
    }
}

fn config_path() -> CmdResult<PathBuf> {
    AiConfig::default_path().ok_or_else(|| CommandError::new("io", "no config dir"))
}

fn key_name(cfg: &AiConfig) -> &'static str {
    match cfg.cloud_kind {
        CloudKind::OpenAiCompatible => "openai_compatible",
        CloudKind::Anthropic => "anthropic",
    }
}

pub(crate) fn client() -> CmdResult<AiClient> {
    let cfg = AiConfig::load(&config_path()?);
    let key = if cfg.mode == AiMode::Cloud {
        sapu_ai::keys::get_api_key(key_name(&cfg))?
    } else {
        None
    };
    Ok(AiClient::new(&cfg, key)?)
}

pub(crate) fn language() -> String {
    Settings::default_path()
        .and_then(|p| Settings::load(&p))
        .ok()
        .and_then(|s| s.locale)
        .unwrap_or_else(|| "id".into())
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AiStatus {
    enabled: bool,
    config: AiConfig,
    has_key: bool,
    /// Last four characters of the stored key, so the user can tell which
    /// key is in the keychain without ever seeing it whole.
    key_hint: Option<String>,
}

#[tauri::command]
pub fn ai_status() -> CmdResult<AiStatus> {
    let config = AiConfig::load(&config_path()?);
    let key = sapu_ai::keys::get_api_key(key_name(&config)).unwrap_or(None);
    let key_hint = key.as_deref().map(|k| {
        let tail: String = k
            .chars()
            .rev()
            .take(4)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        format!("…{tail}")
    });
    Ok(AiStatus {
        enabled: config.is_enabled(),
        config,
        has_key: key.is_some(),
        key_hint,
    })
}

#[tauri::command]
pub fn ai_config_set(config: AiConfig) -> CmdResult<()> {
    config
        .save(&config_path()?)
        .map_err(|e| CommandError::new("io", e.to_string()))?;
    // Keep the core setting in sync so the wizard/settings reflect the mode.
    if let Ok(p) = Settings::default_path()
        && let Ok(mut s) = Settings::load(&p)
    {
        s.ai_mode = match config.mode {
            AiMode::Off => sapu_core::settings::AiMode::Off,
            AiMode::Local => sapu_core::settings::AiMode::Local,
            AiMode::Cloud => sapu_core::settings::AiMode::Cloud,
        };
        let _ = s.save(&p);
    }
    Ok(())
}

#[tauri::command]
pub fn ai_set_api_key(key: String) -> CmdResult<()> {
    let cfg = AiConfig::load(&config_path()?);
    if key.trim().is_empty() {
        sapu_ai::keys::delete_api_key(key_name(&cfg))?;
    } else {
        sapu_ai::keys::set_api_key(key_name(&cfg), key.trim())?;
    }
    Ok(())
}

/// Models the configured provider offers: Ollama's installed list for
/// local mode, the provider's `/models` endpoint for cloud mode.
#[tauri::command]
pub async fn ai_models() -> CmdResult<Vec<String>> {
    let cfg = AiConfig::load(&config_path()?);
    let key = if cfg.mode == AiMode::Cloud {
        sapu_ai::keys::get_api_key(key_name(&cfg))?
    } else {
        None
    };
    tauri::async_runtime::spawn_blocking(move || {
        use sapu_ai::provider::{Anthropic, Ollama, OpenAiCompatible};
        let t = std::time::Duration::from_secs(10);
        let r = match (cfg.mode, cfg.cloud_kind) {
            (AiMode::Cloud, CloudKind::Anthropic) => {
                let key = key.ok_or_else(|| CommandError::new("ai_provider", "API key missing"))?;
                Anthropic::list_models(&cfg.cloud_url, &key, t)
            }
            (AiMode::Cloud, CloudKind::OpenAiCompatible) => {
                OpenAiCompatible::list_models(&cfg.cloud_url, key.as_deref(), t)
            }
            _ => Ollama::list_models(&cfg.local_url, t),
        };
        r.map_err(|e| CommandError::new("ai_provider", e.to_string()))
    })
    .await
    .map_err(|e| CommandError::new("io", e.to_string()))?
}

/// Connectivity check; returns the model that answered.
#[tauri::command]
pub async fn ai_test() -> CmdResult<String> {
    tauri::async_runtime::spawn_blocking(|| Ok(client()?.ping()?))
        .await
        .map_err(|e| CommandError::new("io", e.to_string()))?
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AiResult<T> {
    answer: Option<T>,
    preview: Preview,
}

/// A1. `dry_run` returns only the payload that would be sent.
#[tauri::command]
pub async fn ai_whatis(path: PathBuf, dry_run: bool) -> CmdResult<AiResult<WhatIsAnswer>> {
    tauri::async_runtime::spawn_blocking(move || {
        let facts = sapu_core::scanner::describe(&path, 20_000)
            .map_err(|e| CommandError::new("io", e.to_string()))?;
        let input = WhatIsInput {
            language: language(),
            os: std::env::consts::OS.into(),
            path: facts.path.to_string_lossy().into_owned(),
            is_dir: facts.is_dir,
            bytes: facts.bytes,
            file_count: facts.file_count,
            top_extensions: facts.top_extensions,
            newest_age_days: facts.newest_age_days,
            oldest_age_days: facts.oldest_age_days,
        };
        let c = client()?;
        let preview = c.preview::<WhatIs>(&input)?;
        let answer = if dry_run {
            None
        } else {
            Some(c.run::<WhatIs>(&input)?)
        };
        Ok(AiResult { answer, preview })
    })
    .await
    .map_err(|e| CommandError::new("io", e.to_string()))?
}

/// A7 for one project of the last clean evaluation.
#[tauri::command]
pub async fn ai_artefact(
    session: State<'_, Arc<crate::clean::CleanSession>>,
    root: PathBuf,
    dry_run: bool,
) -> CmdResult<AiResult<ArtefactAnswer>> {
    let project = session
        .project(&root)
        .ok_or_else(|| CommandError::new("no_evaluation", "project not in the last evaluation"))?;
    tauri::async_runtime::spawn_blocking(move || {
        let frameworks: Vec<String> = project
            .artefacts
            .iter()
            .filter_map(|a| match a.kind.as_str() {
                ".next" => Some("Next.js"),
                ".nuxt" | ".output" => Some("Nuxt"),
                ".svelte-kit" => Some("SvelteKit"),
                ".angular" => Some("Angular"),
                "storybook-static" => Some("Storybook"),
                ".turbo" => Some("Turborepo"),
                ".dart_tool" => Some("Dart/Flutter"),
                _ => None,
            })
            .map(str::to_string)
            .collect();
        let input = ArtefactInput {
            language: language(),
            project_name: project.name.clone(),
            ecosystem: format!("{:?}", project.ecosystem).to_lowercase(),
            frameworks,
            scripts: project
                .rebuild
                .iter()
                .filter_map(|c| c.split_whitespace().last())
                .map(str::to_string)
                .collect(),
            idle_days: project.idle_days,
            rebuild: project.rebuild.clone(),
            artefacts: project
                .artefacts
                .iter()
                .map(|a| ArtefactSignal {
                    kind: a.kind.clone(),
                    bytes: a.bytes,
                    gitignored: a.gitignored,
                    tracked: a.tracked,
                })
                .collect(),
        };
        let c = client()?;
        let preview = c.preview::<Artefact>(&input)?;
        let answer = if dry_run {
            None
        } else {
            Some(c.run::<Artefact>(&input)?)
        };
        Ok(AiResult { answer, preview })
    })
    .await
    .map_err(|e| CommandError::new("io", e.to_string()))?
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AiSearchResult {
    filter: SearchFilter,
    files: Vec<FileEntry>,
    preview: Preview,
}

/// A2: sentence → validated filter → deterministic query over the last scan.
#[tauri::command]
pub async fn ai_search(
    session: State<'_, Arc<crate::scan::ScanSession>>,
    sentence: String,
    limit: usize,
) -> CmdResult<AiSearchResult> {
    let result = session.result_arc()?;
    tauri::async_runtime::spawn_blocking(move || {
        let input = SearchInput {
            language: language(),
            sentence: sentence.chars().take(300).collect(),
            categories: Category::ALL
                .iter()
                .map(|c| format!("{c:?}").to_lowercase())
                .collect(),
        };
        let c = client()?;
        let preview = c.preview::<Search>(&input)?;
        let filter = c.run::<Search>(&input)?;
        let category = filter.category.as_deref().and_then(|name| {
            Category::ALL
                .iter()
                .copied()
                .find(|c| format!("{c:?}").eq_ignore_ascii_case(name))
        });
        let core_filter = FileFilter {
            min_bytes: filter.min_bytes.unwrap_or(0),
            category,
            age: AgeFilter::Any,
            min_age_days: filter.min_age_days,
            max_age_days: filter.max_age_days,
            extensions: filter.extensions.clone(),
            name_contains: filter.name_contains.clone(),
        };
        let files = result.top_files(&core_filter, limit.min(500));
        Ok(AiSearchResult {
            filter,
            files,
            preview,
        })
    })
    .await
    .map_err(|e| CommandError::new("io", e.to_string()))?
}

/// A3: narrative summary of the last scan.
#[tauri::command]
pub async fn ai_report(
    session: State<'_, Arc<crate::scan::ScanSession>>,
    dry_run: bool,
) -> CmdResult<AiResult<ReportAnswer>> {
    let result = session.result_arc()?;
    tauri::async_runtime::spawn_blocking(move || {
        let input = ReportInput {
            language: language(),
            root: result.root.to_string_lossy().into_owned(),
            total_bytes: result.total_bytes,
            disk_total_bytes: result.disk_total_bytes,
            categories: result
                .categories()
                .into_iter()
                .filter(|c| c.bytes > 0)
                .map(|c| (format!("{:?}", c.category).to_lowercase(), c.bytes))
                .collect(),
            top_dirs: result
                .top_dirs(12)
                .into_iter()
                .map(|d| (d.path.to_string_lossy().into_owned(), d.bytes))
                .collect(),
            delta_bytes_since_last: result.diff.as_ref().map(|d| d.delta_bytes),
        };
        let c = client()?;
        let preview = c.preview::<Report>(&input)?;
        let answer = if dry_run {
            None
        } else {
            Some(c.run::<Report>(&input)?)
        };
        Ok(AiResult { answer, preview })
    })
    .await
    .map_err(|e| CommandError::new("io", e.to_string()))?
}

/// A4: keep a confirmed A1 answer as a local (yellow) rule.
#[tauri::command]
pub fn ai_save_rule(
    path: PathBuf,
    name_id: String,
    name_en: String,
    description_id: String,
    description_en: String,
) -> CmdResult<PathBuf> {
    let dir = sapu_core::rules::user_rules_dir()
        .ok_or_else(|| CommandError::new("io", "no config dir"))?;
    let slug: String = path
        .file_name()
        .map(|n| n.to_string_lossy().to_lowercase())
        .unwrap_or_default()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string();
    let id = format!(
        "ai-{}-{}",
        if slug.is_empty() {
            "rule".to_string()
        } else {
            slug
        },
        &blake3::hash(path.to_string_lossy().as_bytes()).to_hex()[..6]
    );
    sapu_core::rules::save_user_rule(
        &dir,
        &id,
        (&name_id, &name_en),
        (&description_id, &description_en),
        &path,
    )
    .map_err(|e| CommandError::new("io", e.to_string()))
}
