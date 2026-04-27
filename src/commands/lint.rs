use std::fs;
use std::path::{Path, PathBuf};

use clap::Args;
use colored::*;

use crate::error::Error;

#[derive(Args, Debug)]
pub struct LintArgs {
    /// Path to .libskills/ directory or skill.json file
    #[arg(default_value = ".libskills")]
    pub path: String,

    /// Automatically fix issues where possible
    #[arg(short, long)]
    pub fix: bool,
}

struct LintResult {
    errors: Vec<String>,
    warnings: Vec<String>,
}

pub fn run(args: LintArgs) -> Result<(), Error> {
    let skill_path = resolve_skill_json(&args.path)?;
    let base_dir = skill_path.parent().unwrap_or(Path::new("."));

    let content = fs::read_to_string(&skill_path)?;
    let instance: serde_json::Value = serde_json::from_str(&content)?;

    let mut result = LintResult {
        errors: Vec::new(),
        warnings: Vec::new(),
    };

    check_token_limits(base_dir, &instance, &mut result);
    check_required_files(base_dir, &instance, &mut result);
    check_pitfalls_entries(base_dir, &instance, &mut result);
    check_safety_entries(base_dir, &instance, &mut result);
    check_examples(base_dir, &instance, &mut result);
    check_metadata(&instance, &mut result);

    if args.fix && (!result.errors.is_empty() || !result.warnings.is_empty()) {
        println!("{}", "LibSkills Lint — Fix Mode".bold());
        println!("  Path: {}", base_dir.display());
        println!();
        if !result.errors.is_empty() {
            println!("  Fixing errors...");
        }
        let fixed = apply_fixes(base_dir, &instance, &result)?;
        println!();
        if fixed > 0 {
            println!("  {} {} issue(s) fixed. Re-run lint to verify.", "✓".green(), fixed);
        }
        return Ok(());
    }

    // Print results
    println!("{}", "LibSkills Lint".bold());
    println!("  Path: {}", base_dir.display());
    println!();

    if result.errors.is_empty() && result.warnings.is_empty() {
        println!("{}", "✓ No issues found".green().bold());
        if let Some(completeness) = instance.get("completeness").and_then(|c| c.as_u64()) {
            println!("  Completeness score: {}/100", completeness);
        }
    } else {
        for error in &result.errors {
            println!("  {} {}", "ERROR".red().bold(), error);
        }
        for warning in &result.warnings {
            println!("  {} {}", "WARN".yellow().bold(), warning);
        }
        println!();
        if result.errors.is_empty() {
            println!("{} ({} warnings)", "✓ No errors".green().bold(), result.warnings.len());
        } else {
            println!("{} ({} errors, {} warnings)", "✗ Lint failed".red().bold(), result.errors.len(), result.warnings.len());
        }
    }

    if !result.errors.is_empty() {
        return Err(Error::Lint("lint found errors".into()));
    }

    Ok(())
}

fn check_token_limits(base_dir: &Path, instance: &serde_json::Value, result: &mut LintResult) {
    let mut files_to_check: Vec<String> = Vec::new();
    if let Some(files) = instance.get("files") {
        for priority in &["P0", "P1", "P2", "P3"] {
            if let Some(list) = files.get(priority).and_then(|l| l.as_array()) {
                for f in list {
                    if let Some(name) = f.as_str() {
                        if name.ends_with(".md") {
                            files_to_check.push(name.to_string());
                        }
                    }
                }
            }
        }
    }

    for filename in &files_to_check {
        let file_path = base_dir.join(filename);
        if let Ok(content) = fs::read_to_string(&file_path) {
            let tokens = estimate_tokens(&content);
            if tokens < 300 {
                result.warnings.push(format!(
                    "{} is very short (~{} tokens). Minimum recommended: 500 tokens.",
                    filename, tokens
                ));
            } else if tokens < 500 {
                result.warnings.push(format!(
                    "{} is below recommended minimum (~{} tokens, target: 500-1500).",
                    filename, tokens
                ));
            } else if tokens > 2000 {
                result.warnings.push(format!(
                    "{} exceeds recommended maximum (~{} tokens, target: 500-1500).",
                    filename, tokens
                ));
            }
        }
    }
}

fn check_required_files(base_dir: &Path, instance: &serde_json::Value, result: &mut LintResult) {
    let p0_files = instance.get("files")
        .and_then(|f| f.get("P0"))
        .and_then(|f| f.as_array());

    if let Some(p0) = p0_files {
        for f in p0 {
            if let Some(name) = f.as_str() {
                if !base_dir.join(name).exists() {
                    result.errors.push(format!("Required P0 file '{}' is missing", name));
                }
            }
        }
    }
}

fn check_pitfalls_entries(base_dir: &Path, instance: &serde_json::Value, result: &mut LintResult) {
    let pitfalls_path = find_file_in_instance(base_dir, instance, "pitfalls.md");
    if let Some(path) = pitfalls_path {
        if let Ok(content) = fs::read_to_string(&path) {
            let entries = count_markdown_sections(&content, 3);
            if entries < 3 {
                result.errors.push(format!(
                    "pitfalls.md has only {} section(s). Minimum required: 3.",
                    entries
                ));
            }
        }
    }
}

fn check_safety_entries(base_dir: &Path, instance: &serde_json::Value, result: &mut LintResult) {
    let safety_path = find_file_in_instance(base_dir, instance, "safety.md");
    if let Some(path) = safety_path {
        if let Ok(content) = fs::read_to_string(&path) {
            let entries = count_markdown_sections(&content, 3);
            if entries < 2 {
                result.errors.push(format!(
                    "safety.md has only {} section(s). Minimum required: 2.",
                    entries
                ));
            }
        }
    }
}

fn check_examples(base_dir: &Path, _instance: &serde_json::Value, result: &mut LintResult) {
    let examples = base_dir.join("examples");
    if examples.exists() {
        let count = std::fs::read_dir(&examples)
            .map(|entries| entries.filter_map(|e| e.ok()).count())
            .unwrap_or(0);
        if count == 0 {
            result.errors.push("examples/ directory is empty. At least 1 example required.".into());
        }
    } else {
        result.errors.push("examples/ directory is missing. At least 1 example required.".into());
    }
}

fn check_metadata(instance: &serde_json::Value, result: &mut LintResult) {
    if let Some(tags) = instance.get("tags").and_then(|t| t.as_array()) {
        if tags.is_empty() {
            result.errors.push("tags field is empty. At least 1 tag required.".into());
        }
    } else {
        result.errors.push("tags field is missing.".into());
    }

    if let Some(risk) = instance.get("risk_level").and_then(|r| r.as_str()) {
        if !["high", "medium", "low"].contains(&risk) {
            result.errors.push(format!(
                "risk_level must be 'high', 'medium', or 'low'. Got '{}'.",
                risk
            ));
        }
    }

    if let Some(version) = instance.get("skill_version").and_then(|v| v.as_str()) {
        if !version.chars().all(|c| c.is_ascii_digit() || c == '.') {
            result.warnings.push(format!(
                "skill_version '{}' may not be valid semver.",
                version
            ));
        }
    }

    if instance.get("repo_skill").and_then(|r| r.as_bool()).is_none() {
        result.warnings.push("repo_skill field is missing. Should be true for repo-hosted skills.".into());
    }

    if instance.get("skill_type").is_none() {
        result.warnings.push("skill_type field is missing. Should be one of: library, framework, sdk, ...".into());
    }
}

fn find_file_in_instance(base_dir: &Path, instance: &serde_json::Value, name: &str) -> Option<PathBuf> {
    if let Some(files) = instance.get("files") {
        for priority in &["P0", "P1", "P2", "P3"] {
            if let Some(list) = files.get(priority).and_then(|l| l.as_array()) {
                for f in list {
                    if f.as_str().map_or(false, |s| s.contains(name)) {
                        let full_path = base_dir.join(f.as_str().unwrap());
                        if full_path.exists() {
                            return Some(full_path);
                        }
                    }
                }
            }
        }
    }
    // Fallback: check directly
    let fallback = base_dir.join(name);
    if fallback.exists() {
        return Some(fallback);
    }
    None
}

fn count_markdown_sections(content: &str, level: usize) -> usize {
    let prefix = "#".repeat(level) + " ";
    content.lines().filter(|line| line.starts_with(&prefix)).count()
}

fn estimate_tokens(text: &str) -> usize {
    let word_count = text.split_whitespace().count();
    let char_count = text.chars().count();
    let word_estimate = word_count * 13 / 10; // ~1.3 tokens per word
    let char_estimate = char_count / 4; // ~4 chars per token
    (word_estimate + char_estimate) / 2
}

fn resolve_skill_json(path_str: &str) -> Result<std::path::PathBuf, Error> {
    let path = Path::new(path_str);
    if path.is_file() {
        Ok(path.to_path_buf())
    } else if path.is_dir() {
        let skill_json = path.join("skill.json");
        if skill_json.exists() {
            Ok(skill_json)
        } else {
            Err(Error::Lint(format!(
                "No skill.json found in '{}'. Is this a .libskills/ directory?",
                path.display()
            )))
        }
    } else {
        Err(Error::Lint(format!(
            "Path '{}' does not exist",
            path.display()
        )))
    }
}

fn apply_fixes(base_dir: &Path, instance: &serde_json::Value, _result: &LintResult) -> Result<usize, Error> {
    let mut fixed = 0usize;
    let example_ext = detect_example_ext(instance);

    // Fix: ensure pitfalls.md has at least 3 entries
    fixed += fix_markdown_sections(base_dir, instance, "pitfalls.md", 3, "### Do NOT ...\n\n```\n// BAD:\n\n// GOOD:\n```\n")?;

    // Fix: ensure safety.md has at least 2 entries
    fixed += fix_markdown_sections(base_dir, instance, "safety.md", 2, "### NEVER ...\n\n- ...\n")?;

    // Fix: ensure examples/ directory exists
    let examples_dir = base_dir.join("examples");
    if !examples_dir.exists() {
        fs::create_dir_all(&examples_dir)?;
        fixed += 1;
    }

    // Fix: ensure at least 1 example file
    let has_example = examples_dir.exists() && std::fs::read_dir(&examples_dir)
        .map(|mut e| e.next().is_some())
        .unwrap_or(false);
    if !has_example {
        let example_file = examples_dir.join(format!("basic.{}", example_ext));
        let content = match example_ext {
            "cpp" => "// TODO: add working example\n",
            "rs" => "// TODO: add working example\n",
            "py" => "# TODO: add working example\n",
            "go" => "// TODO: add working example\n",
            "js" => "// TODO: add working example\n",
            _ => "# TODO: add working example\n",
        };
        fs::write(&example_file, content)?;
        println!("  {} {}", "create".green(), example_file.display());
        fixed += 1;
    }

    // Fix: ensure P0 required files exist
    let p0_required = ["overview.md", "pitfalls.md", "safety.md"];
    for filename in &p0_required {
        let file_path = base_dir.join(filename);
        if !file_path.exists() {
            let lang = instance.get("language")
                .and_then(|l| l.as_str())
                .unwrap_or("python");
            let name = instance.get("name")
                .and_then(|n| n.as_str())
                .unwrap_or("Library");
            let content = p0_placeholder(filename, name, lang);
            fs::write(&file_path, content)?;
            println!("  {} {}", "create".green(), file_path.display());
            fixed += 1;
        }
    }

    Ok(fixed)
}

fn fix_markdown_sections(base_dir: &Path, instance: &serde_json::Value, filename: &str, required: usize, template: &str) -> Result<usize, Error> {
    let file_path = find_file_in_instance(base_dir, instance, filename)
        .or_else(|| {
            let fallback = base_dir.join(filename);
            if fallback.exists() { Some(fallback) } else { None }
        });

    let file_path = match file_path {
        Some(p) => p,
        None => {
            // File doesn't exist — create it
            let lang = instance.get("language").and_then(|l| l.as_str()).unwrap_or("python");
            let name = instance.get("name").and_then(|n| n.as_str()).unwrap_or("Library");
            let content = p0_placeholder(filename, name, lang);
            let p = base_dir.join(filename);
            fs::write(&p, &content)?;
            println!("  {} {}", "create".green(), p.display());
            return Ok(1);
        }
    };

    let content = fs::read_to_string(&file_path)?;
    let current = count_markdown_sections(&content, 3);
    if current >= required {
        return Ok(0);
    }

    let needed = required - current;
    let mut append = String::new();
    for i in 0..needed {
        append.push_str(&template.replace("...", &format!("[TODO: add section {}]", current + i + 1)));
        append.push('\n');
    }

    let mut new_content = content;
    new_content.push('\n');
    new_content.push_str(&append);
    fs::write(&file_path, &new_content)?;

    println!("  {} {} (added {} section{})", "fix".cyan(), filename, needed, if needed > 1 { "s" } else { "" });
    Ok(1)
}

fn p0_placeholder(filename: &str, name: &str, _lang: &str) -> String {
    let header = match filename {
        "overview.md" => format!("# {} — Overview\n\n**{}** is a ...\n\n## When to Use\n\n- ...\n\n## When NOT to Use\n\n- ...\n", name, name),
        "pitfalls.md" => format!("# {} — Pitfalls\n\nCommon mistakes that cause crashes, data loss, or silent misbehavior.\n\n### Do NOT ...\n\n```\n// BAD:\n\n// GOOD:\n```\n\n### Do NOT ...\n\n### Do NOT ...\n", name),
        "safety.md" => format!("# {} — Safety\n\nRed lines — conditions that must NEVER occur.\n\n### NEVER ...\n\n- ...\n\n### NEVER ...\n\n- ...\n", name),
        _ => format!("# {} — {}\n\n...\n", name, filename.trim_end_matches(".md")),
    };
    header
}

fn detect_example_ext(instance: &serde_json::Value) -> &'static str {
    if let Some(files) = instance.get("files") {
        if let Some(p3) = files.get("P3").and_then(|p| p.as_array()) {
            for f in p3 {
                if let Some(name) = f.as_str() {
                    if let Some(ext) = std::path::Path::new(name).extension() {
                        return match ext.to_str().unwrap_or("") {
                            "cpp" | "cc" | "cxx" => "cpp",
                            "rs" => "rs",
                            "py" => "py",
                            "go" => "go",
                            "js" => "js",
                            _ => "txt",
                        };
                    }
                }
            }
        }
    }
    // Fallback: guess from language
    if let Some(lang) = instance.get("language").and_then(|l| l.as_str()) {
        return match lang {
            "cpp" => "cpp",
            "rust" => "rs",
            "python" => "py",
            "go" => "go",
            "js" => "js",
            _ => "txt",
        };
    }
    "txt"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count_markdown_sections_level3() {
        let content = "### Section A\n\n### Section B\n\n### Section C\n";
        assert_eq!(count_markdown_sections(content, 3), 3);
    }

    #[test]
    fn test_count_markdown_sections_ignores_level2() {
        let content = "## Level 2\n\n### Section A\n\n### Section B\n";
        assert_eq!(count_markdown_sections(content, 3), 2);
    }

    #[test]
    fn test_count_markdown_sections_empty() {
        assert_eq!(count_markdown_sections("no headers here", 3), 0);
    }

    #[test]
    fn test_estimate_tokens_text() {
        // ~50 chars of English text ≈ 12-15 tokens
        let tokens = estimate_tokens("The quick brown fox jumps over the lazy dog");
        assert!(tokens > 5 && tokens < 30, "Got {}", tokens);
    }

    #[test]
    fn test_estimate_tokens_code_block() {
        let text = "```\nfn main() { println!(\"hello\"); }\n```\nSome text after code";
        let tokens = estimate_tokens(text);
        assert!(tokens > 3, "Got {}", tokens);
    }
}
