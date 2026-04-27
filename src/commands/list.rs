use std::fs;

use clap::Args;
use colored::*;

use crate::cache::Cache;
use crate::error::Error;

#[derive(Args, Debug)]
pub struct ListArgs {
    /// Show detailed information for each cached skill
    #[arg(short, long)]
    verbose: bool,
}

pub fn run(args: ListArgs) -> Result<(), Error> {
    let cache = Cache::new(Cache::default_location());

    let skills = cache.list_cached()?;

    if skills.is_empty() {
        println!("{}", "No skills in local cache.".dimmed());
        println!("Run 'libskills get <key>' to download a skill.");
        println!("Use 'libskills search <keyword>' to find skills.");
        return Ok(());
    }

    println!("{} skill(s) in cache:", skills.len());
    println!();

    for key in &skills {
        let dir = cache.skill_dir(key);
        let skill_json = dir.join("skill.json");

        if let Ok(content) = fs::read_to_string(&skill_json) {
            if let Ok(skill) = serde_json::from_str::<serde_json::Value>(&content) {
                let name = skill["name"].as_str().unwrap_or(key);
                let version = skill["version"].as_str().unwrap_or("?");
                let tier = skill["tier"].as_str().unwrap_or("?");
                let language = skill["language"].as_str().unwrap_or("?");
                let trust = skill["trust_score"].as_i64().map_or(String::new(), |t| format!(" trust:{}", t));
                let risk = skill["risk_level"].as_str().unwrap_or("?");

                println!("  {} {} v{} [{}, {}]{} risk:{}",
                    name.bold(),
                    language.dimmed(),
                    version,
                    tier,
                    trust.trim(),
                    trust.dimmed(),
                    risk,
                );

                if args.verbose {
                    if let Some(tags) = skill["tags"].as_array() {
                        let tag_list: Vec<&str> = tags.iter().filter_map(|t| t.as_str()).collect();
                        println!("    Tags: {}", tag_list.join(", ").blue());
                    }
                    if let Some(_summary) = skill.get("summary").and_then(|s| s.as_str()) {
                        // summary is in index, not in skill.json — skip if absent
                    }

                    let file_count = dir.join("overview.md").exists() as usize
                        + dir.join("pitfalls.md").exists() as usize
                        + dir.join("safety.md").exists() as usize
                        + dir.join("lifecycle.md").exists() as usize
                        + dir.join("threading.md").exists() as usize
                        + dir.join("best-practices.md").exists() as usize
                        + dir.join("performance.md").exists() as usize;
                    println!("    {} knowledge files cached", file_count);
                }
            }
        }
    }

    Ok(())
}
