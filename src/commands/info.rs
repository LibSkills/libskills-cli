use std::fs;

use clap::Args;
use colored::*;

use crate::cache::Cache;
use crate::error::Error;

#[derive(Args, Debug)]
pub struct InfoArgs {
    /// Skill key (e.g., "cpp/gabime/spdlog")
    pub key: String,
}

pub fn run(args: InfoArgs) -> Result<(), Error> {
    let cache = Cache::new(Cache::default_location());
    let skill_dir = cache.skill_dir(&args.key);

    let skill_json_path = skill_dir.join("skill.json");
    if !skill_json_path.exists() {
        println!("{} Skill '{}' is not in the local cache.", "✗".red(), args.key);
        println!("Run 'libskills get {}' to download it first.", args.key);
        return Ok(());
    }

    let content = fs::read_to_string(&skill_json_path)?;
    let skill: serde_json::Value = serde_json::from_str(&content)?;

    println!("{}", "━".repeat(50));
    print_field(&skill, "Name", "name");
    print_field(&skill, "Repo", "repo");
    print_field(&skill, "Language", "language");
    print_field(&skill, "Tier", "tier");
    print_field(&skill, "Group", "group");
    print_field(&skill, "Skill Type", "skill_type");
    print_field(&skill, "Library Version", "version");
    print_field(&skill, "Skill Version", "skill_version");
    print_field(&skill, "Schema", "schema");

    if let Some(ts) = skill.get("trust_score").and_then(|v| v.as_i64()) {
        let label = match ts {
            90..=100 => format!("{}/100", ts).green(),
            50..=89 => format!("{}/100", ts).yellow(),
            _ => format!("{}/100", ts).red(),
        };
        println!("  {:<20} {}", "Trust Score:".bold(), label);
    }

    print_field(&skill, "Risk Level", "risk_level");
    print_field(&skill, "Updated", "updated_at");
    print_field(&skill, "Completeness", "completeness");

    if let Some(tags) = skill.get("tags").and_then(|t| t.as_array()) {
        let tags_str: Vec<String> = tags.iter()
            .filter_map(|t| t.as_str().map(String::from))
            .collect();
        println!("  {:<20} {}", "Tags:".bold(), tags_str.join(", ").blue());
    }

    // Dependencies
    if let Some(deps) = skill.get("dependencies") {
        if let Some(req) = deps.get("required").and_then(|r| r.as_array()) {
            if !req.is_empty() {
                let req_str: Vec<String> = req.iter()
                    .filter_map(|r| r.as_str().map(String::from))
                    .collect();
                println!("  {:<20} {}", "Required Deps:".bold(), req_str.join(", "));
            }
        }
    }

    // Files by priority
    if let Some(files) = skill.get("files") {
        println!();
        for priority in &["P0", "P1", "P2", "P3"] {
            if let Some(list) = files.get(priority).and_then(|l| l.as_array()) {
                if !list.is_empty() {
                    println!("  {} files:", priority.bold());
                    for f in list {
                        if let Some(name) = f.as_str() {
                            let exists = if skill_dir.join(name).exists() {
                                "✓".green()
                            } else {
                                "✗".red()
                            };
                            println!("    {} {}", exists, name);
                        }
                    }
                }
            }
        }
    }

    // Read order
    if let Some(order) = skill.get("read_order").and_then(|o| o.as_array()) {
        let order_str: Vec<String> = order.iter()
            .filter_map(|o| o.as_str().map(String::from))
            .collect();
        println!();
        println!("  {:<20} {}", "Read Order:".bold(), order_str.join(" → ").cyan());
    }

    println!("{}", "━".repeat(50));
    Ok(())
}

fn print_field(skill: &serde_json::Value, label: &str, field: &str) {
    if let Some(value) = skill.get(field) {
        let display = match value {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::Bool(b) => b.to_string(),
            _ => return,
        };
        println!("  {:<20} {}", format!("{}:", label).bold(), display);
    }
}
