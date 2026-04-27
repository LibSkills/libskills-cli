use std::fs;
use std::path::PathBuf;

use clap::Args;
use colored::*;

use crate::cache::Cache;
use crate::error::Error;

#[derive(Args, Debug)]
pub struct GetArgs {
    /// Skill key (e.g., "cpp/gabime/spdlog")
    pub key: String,

    /// Path to the registry directory containing skill files
    #[arg(short, long)]
    registry: Option<String>,
}

pub fn run(args: GetArgs) -> Result<(), Error> {
    let cache = Cache::new(Cache::default_location());
    cache.ensure_dirs()?;

    let registry_path = resolve_registry_path(args.registry)?;
    let source_dir = registry_path.join("skills").join(&args.key);

    if !source_dir.exists() {
        return Err(Error::Schema(format!(
            "Skill '{}' not found in registry at '{}'.",
            args.key, registry_path.display()
        )));
    }

    let dest_dir = cache.skill_dir(&args.key);
    if dest_dir.exists() {
        fs::remove_dir_all(&dest_dir)?;
    }
    fs::create_dir_all(&dest_dir)?;

    copy_dir_recursive(&source_dir, &dest_dir)?;

    let file_count = count_files(&dest_dir);
    println!("{} Skill '{}' downloaded", "✓".green().bold(), args.key);
    println!("  {} files → {}", file_count, dest_dir.display());
    Ok(())
}

fn resolve_registry_path(user_path: Option<String>) -> Result<std::path::PathBuf, Error> {
    if let Some(path) = user_path {
        let pb = PathBuf::from(&path);
        if pb.exists() {
            return Ok(pb);
        }
        return Err(Error::Schema(format!("Registry path '{}' not found", path)));
    }

    // Auto-detect sibling libskills-registry
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

    let cwd = std::env::current_dir().unwrap_or_default();
    for ancestor in cwd.ancestors().take(6) {
        let candidate = ancestor.join("libskills-registry");
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    Err(Error::Schema(
        "Could not find libskills-registry. Use --registry <path> to specify it.".into()
    ))
}

fn copy_dir_recursive(src: &std::path::Path, dst: &std::path::Path) -> Result<(), Error> {
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let dst_path = dst.join(entry.file_name());

        if file_type.is_dir() {
            fs::create_dir_all(&dst_path)?;
            copy_dir_recursive(&entry.path(), &dst_path)?;
        } else {
            fs::copy(entry.path(), &dst_path)?;
        }
    }
    Ok(())
}

fn count_files(dir: &std::path::Path) -> usize {
    let mut count = 0;
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            if entry.file_type().map_or(false, |t| t.is_file()) {
                count += 1;
            }
            if entry.file_type().map_or(false, |t| t.is_dir()) {
                count += count_files(&entry.path());
            }
        }
    }
    count
}
