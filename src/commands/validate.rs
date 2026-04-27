use std::fs;
use std::path::{Path, PathBuf};

use clap::Args;
use colored::*;
use jsonschema::JSONSchema;

use crate::error::Error;

const SKILL_SCHEMA_V0: &str = include_str!("../schemas/skill_v1.json");

#[derive(Args, Debug)]
pub struct ValidateArgs {
    /// Path to .libskills/ directory or skill.json file
    #[arg(default_value = ".libskills")]
    pub path: String,
}

pub fn run(args: ValidateArgs) -> Result<(), Error> {
    let skill_path = resolve_skill_json(&args.path)?;

    let content = fs::read_to_string(&skill_path)?;
    let instance: serde_json::Value = serde_json::from_str(&content)?;
    let schema: serde_json::Value = serde_json::from_str(SKILL_SCHEMA_V0)
        .map_err(|e| Error::Schema(format!("Failed to parse embedded schema: {}", e)))?;

    let compiled = JSONSchema::compile(&schema)
        .map_err(|e| Error::Schema(format!("Failed to compile schema: {}", e)))?;

    let mut errors = Vec::new();
    let validation = compiled.validate(&instance);
    if let Err(validation_errors) = validation {
        for error in validation_errors {
            errors.push(format!("  {} {}", "✗".red(), error));
        }
    }

    if errors.is_empty() {
        println!("{}", "✓ skill.json is valid".green().bold());
    } else {
        println!("{} ({} errors)", "✗ skill.json validation failed".red().bold(), errors.len());
        for e in &errors {
            println!("{}", e);
        }
    }

    // Check that referenced files exist
    let base_dir = skill_path.parent().unwrap_or(Path::new("."));
    if let Some(files) = instance.get("files") {
        for priority in &["P0", "P1", "P2", "P3"] {
            if let Some(file_list) = files.get(priority) {
                if let Some(arr) = file_list.as_array() {
                    for f in arr {
                        if let Some(filename) = f.as_str() {
                            let file_path = base_dir.join(filename);
                            if !file_path.exists() {
                                println!("  {} {} (referenced but missing)", "✗".yellow(), filename);
                            }
                        }
                    }
                }
            }
        }
    }

    // Check required P0 files
    if let Some(files) = instance.get("files") {
        if let Some(p0) = files.get("P0") {
            if let Some(arr) = p0.as_array() {
                let found_overview = arr.iter().any(|f| f.as_str().map_or(false, |s| s.contains("overview")));
                let found_pitfalls = arr.iter().any(|f| f.as_str().map_or(false, |s| s.contains("pitfalls")));
                let found_safety = arr.iter().any(|f| f.as_str().map_or(false, |s| s.contains("safety")));
                if !found_overview {
                    println!("  {} P0: overview.md not listed in files.P0", "✗".red());
                }
                if !found_pitfalls {
                    println!("  {} P0: pitfalls.md not listed in files.P0", "✗".red());
                }
                if !found_safety {
                    println!("  {} P0: safety.md not listed in files.P0", "✗".red());
                }
            }
        }
    }

    if errors.is_empty() {
        println!();
        println!("{}", "✓ Validation passed".green().bold());
    } else {
        return Err(Error::Validation("skill.json has validation errors".into()));
    }

    Ok(())
}

fn resolve_skill_json(path_str: &str) -> Result<PathBuf, Error> {
    let path = Path::new(path_str);
    if path.is_file() {
        Ok(path.to_path_buf())
    } else if path.is_dir() {
        let skill_json = path.join("skill.json");
        if skill_json.exists() {
            Ok(skill_json)
        } else {
            Err(Error::Validation(format!(
                "No skill.json found in '{}'. Is this a .libskills/ directory?",
                path.display()
            )))
        }
    } else {
        Err(Error::Validation(format!(
            "Path '{}' does not exist",
            path.display()
        )))
    }
}
