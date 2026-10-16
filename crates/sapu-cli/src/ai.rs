//! `sapu ai`: status, "what is this?", and natural-language search.

use std::path::PathBuf;

use anyhow::Context;
use clap::{Args, Subcommand};
use sapu_ai::AiClient;
use sapu_ai::config::{AiConfig, AiMode, CloudKind};
use sapu_ai::tasks::search::{Search, SearchInput};
use sapu_ai::tasks::whatis::{WhatIs, WhatIsInput};
use sapu_core::progress::NoProgress;
use sapu_core::scanner::{AgeFilter, Category, FileFilter, ScanControl, ScanOptions, scan};

use crate::human_bytes;

#[derive(Args)]
pub struct AiArgs {
    #[command(subcommand)]
    action: AiAction,
    /// Only print the payload that would be sent.
    #[arg(long, global = true)]
    dry_run: bool,
}

#[derive(Subcommand)]
enum AiAction {
    /// Show the AI configuration.
    Status,
    /// Explain what a folder or file probably is.
    Whatis { path: PathBuf },
    /// Find files matching a sentence, within a folder.
    Search { sentence: String, root: PathBuf },
}

fn client() -> anyhow::Result<AiClient> {
    let cfg = AiConfig::load(&AiConfig::default_path().context("config dir")?);
    let key = if cfg.mode == AiMode::Cloud {
        let name = match cfg.cloud_kind {
            CloudKind::OpenAiCompatible => "openai_compatible",
            CloudKind::Anthropic => "anthropic",
        };
        sapu_ai::keys::get_api_key(name)?
    } else {
        None
    };
    Ok(AiClient::new(&cfg, key)?)
}

pub fn run(args: AiArgs) -> anyhow::Result<()> {
    match args.action {
        AiAction::Status => {
            let cfg = AiConfig::load(&AiConfig::default_path().context("config dir")?);
            println!("{}", serde_json::to_string_pretty(&cfg)?);
            if cfg.is_enabled() {
                match client().and_then(|c| Ok(c.ping()?)) {
                    Ok(model) => println!("provider ok, model {model}"),
                    Err(e) => println!("provider error: {e}"),
                }
            }
        }
        AiAction::Whatis { path } => {
            let facts = sapu_core::scanner::describe(&path, 20_000).context("describe")?;
            let input = WhatIsInput {
                language: "en".into(),
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
            if args.dry_run {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&c.preview::<WhatIs>(&input)?)?
                );
                return Ok(());
            }
            let a = c.run::<WhatIs>(&input)?;
            println!("{}", serde_json::to_string_pretty(&a)?);
        }
        AiAction::Search { sentence, root } => {
            let input = SearchInput {
                language: "en".into(),
                sentence,
                categories: Category::ALL
                    .iter()
                    .map(|c| format!("{c:?}").to_lowercase())
                    .collect(),
            };
            let c = client()?;
            if args.dry_run {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&c.preview::<Search>(&input)?)?
                );
                return Ok(());
            }
            let f = c.run::<Search>(&input)?;
            eprintln!("filter: {}", serde_json::to_string(&f)?);
            let result = scan(
                &ScanOptions::new(&root, None),
                &ScanControl::new(),
                &mut NoProgress,
            )
            .context("scan")?;
            let category = f.category.as_deref().and_then(|n| {
                Category::ALL
                    .iter()
                    .copied()
                    .find(|c| format!("{c:?}").eq_ignore_ascii_case(n))
            });
            let files = result.top_files(
                &FileFilter {
                    min_bytes: f.min_bytes.unwrap_or(0),
                    category,
                    age: AgeFilter::Any,
                    min_age_days: f.min_age_days,
                    max_age_days: f.max_age_days,
                    extensions: f.extensions.clone(),
                    name_contains: f.name_contains.clone(),
                },
                50,
            );
            for e in files {
                println!("{:>9}  {}", human_bytes(e.size), e.path.display());
            }
        }
    }
    Ok(())
}
