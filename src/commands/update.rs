use std::fs;
use std::path::PathBuf;

use clap::Args;
use colored::*;

use crate::cache::Cache;
use crate::error::Error;
use crate::index::ContentIndex;

#[derive(Args, Debug)]
pub struct UpdateArgs {
    /// Path or URL to the registry. Defaults to the sibling libskills-registry directory.
    #[arg(short, long)]
    registry: Option<String>,
}

pub fn run(args: UpdateArgs) -> Result<(), Error> {
    let cache = Cache::new(Cache::default_location());
    cache.ensure_dirs()?;

    let registry_path = resolve_registry_path(args.registry)?;
    let index_path = registry_path.join("index.json");

    if !index_path.exists() {
        return Err(Error::Schema(format!(
            "Registry index not found at '{}'. Use --registry to specify the registry path.",
            index_path.display()
        )));
    }

    let index_content = fs::read_to_string(&index_path)?;
    let index: serde_json::Value = serde_json::from_str(&index_content)?;
    let skill_count = index.get("skills")
        .and_then(|s| s.as_array())
        .map(|a| a.len())
        .unwrap_or(0);

    fs::write(cache.index_path(), &index_content)?;

    println!("{}", "✓ Registry index updated".green().bold());
    println!("  Source: {}", index_path.display());
    println!("  Skills indexed: {}", skill_count);
    println!("  Local cache: {}", cache.index_path().display());

    // Build content index for semantic search
    let mut content_index = ContentIndex::empty();
    content_index.build(&registry_path)?;
    content_index.save_to_cache(&cache)?;

    if content_index.doc_count > 0 {
        println!("  Content index: {} skill(s) ready for 'libskills find'", content_index.doc_count);
    }

    Ok(())
}

fn resolve_registry_path(user_path: Option<String>) -> Result<PathBuf, Error> {
    if let Some(path) = user_path {
        let pb = PathBuf::from(&path);
        if pb.exists() {
            return Ok(pb);
        }
        return Err(Error::Schema(format!("Registry path '{}' not found", path)));
    }

    // Auto-detect: look for sibling libskills-registry directory
    let exe_path = std::env::current_exe().unwrap_or_default();
    let mut search = exe_path.clone();

    for _ in 0..6 {
        search = match search.parent() {
            Some(p) => p.to_path_buf(),
            None => break,
        };
        let candidate = search.join("libskills-registry");
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    // Fallback: relative to cwd
    let cwd = std::env::current_dir().unwrap_or_default();
    for ancestor in cwd.ancestors().take(6) {
        let candidate = ancestor.join("libskills-registry");
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    Err(Error::Schema(
        "Could not find libskills-registry directory. Use --registry <path> to specify it.".into()
    ))
}
