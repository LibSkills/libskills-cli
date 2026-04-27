use std::fs;

use clap::Args;
use colored::*;

use crate::cache::Cache;
use crate::error::Error;
use crate::index::ContentIndex;

#[derive(Args, Debug)]
pub struct FindArgs {
    /// Natural language query (e.g., "fast async logger for C++")
    pub query: Vec<String>,

    /// Maximum number of results to return
    #[arg(short, long, default_value = "10")]
    pub limit: usize,

    /// Minimum score threshold (0.0 to 1.0)
    #[arg(short = 't', long, default_value = "0.0")]
    pub threshold: f64,

    /// Force rebuild of the content index
    #[arg(long)]
    pub rebuild: bool,

    /// Path to the registry directory containing skill files
    #[arg(short, long)]
    registry: Option<String>,
}

pub fn run(args: FindArgs) -> Result<(), Error> {
    let cache = Cache::new(Cache::default_location());
    let query = args.query.join(" ");

    if query.is_empty() {
        println!("Usage: libskills find <query>");
        println!("Example: libskills find 'fast async logger for C++'");
        return Ok(());
    }

    let registry_path = resolve_registry_path(args.registry)?;

    // Load or build content index
    let mut index = if args.rebuild {
        ContentIndex::empty()
    } else {
        ContentIndex::load_from_cache(&cache)?
    };

    if index.doc_count == 0 || args.rebuild {
        println!("{} Building content index...", "…".dimmed());
        index.build(&registry_path)?;
        index.save_to_cache(&cache)?;
        println!("{} Indexed {} skill(s)", "✓".green(), index.doc_count);
    }

    // Search
    let results = index.search(&query, args.limit);

    if results.is_empty() {
        println!("No skills found for '{}'", query);
        return Ok(());
    }

    // Normalize scores relative to top result
    let max_score = results.first().map(|r| r.score).unwrap_or(1.0);
    let filtered: Vec<_> = results.iter().filter(|r| r.score >= args.threshold).collect();

    println!();
    println!("{} results for '{}':", filtered.len(), query.cyan());
    println!();

    for (i, result) in filtered.iter().enumerate() {
        let normalized = if max_score > 0.0 { result.score / max_score } else { 0.0 };
        let score_percent = (normalized * 100.0).min(100.0);
        let score_bar = match score_percent {
            s if s >= 80.0 => format!("{:.0}%", s).green(),
            s if s >= 40.0 => format!("{:.0}%", s).yellow(),
            s => format!("{:.0}%", s).dimmed(),
        };

        // Try to load skill metadata from cache or registry
        let skill_dir = cache.skill_dir(&result.key);
        let skill_json_path = if skill_dir.exists() {
            skill_dir.join("skill.json")
        } else {
            registry_path.join("skills").join(&result.key).join("skill.json")
        };

        if let Ok(content) = fs::read_to_string(&skill_json_path) {
            if let Ok(skill) = serde_json::from_str::<serde_json::Value>(&content) {
                let name = skill["name"].as_str().unwrap_or(&result.key);
                let language = skill["language"].as_str().unwrap_or("?");
                let tier = skill["tier"].as_str().unwrap_or("?");
                let trust = skill["trust_score"].as_i64().map_or(String::new(), |t| format!(" trust:{}", t));
                let version = skill["version"].as_str().unwrap_or("?");
                let mut tags = String::new();
                if let Some(tags_arr) = skill["tags"].as_array() {
                    let tag_list: Vec<&str> = tags_arr.iter().filter_map(|t| t.as_str()).collect();
                    if !tag_list.is_empty() {
                        tags = format!(" [{}]", tag_list.join(", ").blue());
                    }
                }

                let rank = format!("#{}", i + 1).bold();
                println!("{} {} — {} {}", rank, name.yellow(), score_bar, tags);
                println!("  {}  {} v{} {} [{}]",
                    " ".repeat(3),
                    language.dimmed(),
                    version,
                    tier,
                    trust.trim(),
                );
                println!("  {}  {}", " ".repeat(3), result.key.dimmed());
                println!();
            }
        }
    }

    Ok(())
}

fn resolve_registry_path(user_path: Option<String>) -> Result<std::path::PathBuf, Error> {
    if let Some(path) = user_path {
        let pb = std::path::PathBuf::from(&path);
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
