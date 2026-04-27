use std::fs;

use clap::Args;
use colored::*;

use crate::cache::Cache;
use crate::error::Error;

#[derive(Args, Debug)]
pub struct SearchArgs {
    /// Search keyword (matches name, tags, summary)
    pub keyword: String,
}

#[derive(serde::Deserialize)]
struct IndexSkill {
    key: String,
    name: String,
    language: String,
    tier: String,
    #[allow(dead_code)]
    group: Option<String>,
    #[serde(default)]
    version: Option<String>,
    #[serde(default)]
    trust_score: Option<i64>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    summary: Option<String>,
    #[serde(default)]
    risk_level: Option<String>,
}

#[derive(serde::Deserialize)]
struct RegistryIndex {
    #[serde(default)]
    skills: Vec<IndexSkill>,
}

pub fn run(args: SearchArgs) -> Result<(), Error> {
    let cache = Cache::new(Cache::default_location());
    let index_path = cache.index_path();

    if !index_path.exists() {
        println!("{}", "No local index found. Run 'libskills update' first.".yellow());
        return Ok(());
    }

    let content = fs::read_to_string(&index_path)?;
    let index: RegistryIndex = serde_json::from_str(&content)?;

    let keyword = args.keyword.to_lowercase();
    let mut results: Vec<(&IndexSkill, usize)> = index.skills.iter()
        .map(|skill| {
            let mut score = 0usize;

            let name_lower = skill.name.to_lowercase();
            if name_lower == keyword {
                score += 100;
            } else if name_lower.contains(&keyword) {
                score += 50;
            }

            for tag in &skill.tags {
                if tag.to_lowercase().contains(&keyword) {
                    score += 30;
                }
            }

            if let Some(ref summary) = skill.summary {
                if summary.to_lowercase().contains(&keyword) {
                    score += 20;
                }
            }

            if skill.key.to_lowercase().contains(&keyword) {
                score += 10;
            }

            (skill, score)
        })
        .filter(|(_, score)| *score > 0)
        .collect();

    results.sort_by(|a, b| b.1.cmp(&a.1));

    if results.is_empty() {
        println!("No skills found for '{}'", args.keyword);
        println!("Try 'libskills update' to refresh the index.");
        return Ok(());
    }

    println!("{} results for '{}'", results.len(), args.keyword);
    println!();

    for (skill, _score) in &results {
        let trust = skill.trust_score.map_or(String::new(), |t| format!(" trust:{}", t));
        let version = skill.version.as_deref().unwrap_or("?");
        let risk = skill.risk_level.as_deref().unwrap_or("?");
        let tags_display = if skill.tags.is_empty() {
            String::new()
        } else {
            format!(" [{}]", skill.tags.join(", "))
        };

        if skill.tier == "tier1" {
            println!("{:<40} {:<8} {} v{}{}",
                skill.name.bold(),
                skill.language.dimmed(),
                skill.tier.clone().green(),
                version,
                trust.dimmed(),
            );
        } else {
            println!("{:<40} {:<8} {} v{}{}",
                skill.name.bold(),
                skill.language.dimmed(),
                skill.tier.clone().yellow(),
                version,
                trust.dimmed(),
            );
        }
        println!("  {}", skill.key.dimmed());
        if let Some(ref summary) = skill.summary {
            println!("  {}{}", summary, tags_display.dimmed());
        }
        println!("  risk:{}", risk);
        println!();
    }

    Ok(())
}
