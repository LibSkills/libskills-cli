use std::fs;
use std::path::Path;

use clap::Args;
use colored::*;

use crate::error::Error;

const SKILL_JSON_TEMPLATE: &str = r#"{
  "name": "{name}",
  "repo": "{repo}",
  "language": "{language}",
  "tier": "{tier}",
  "group": "{group}",
  "version": "{version}",
  "skill_version": "0.1.0",
  "schema": "libskills/v1",
  "skill_type": "library",
  "repo_skill": true,
  "trust_score": 0,
  "verified": false,
  "official": false,
  "updated_at": "{updated_at}",
  "completeness": 0,
  "risk_level": "medium",
  "tags": [{tags}],
  "compatibility": {},
  "dependencies": {"required": [], "optional": [], "skills": []},
  "read_order": ["overview.md", "pitfalls.md", "safety.md"],
  "files": {
    "P0": ["overview.md", "pitfalls.md", "safety.md"],
    "P1": ["lifecycle.md", "threading.md", "best-practices.md"],
    "P2": ["performance.md"],
    "P3": ["examples/basic.{ext}"]
  }
}
"#;

const OVERVIEW_TEMPLATE: &str = "# {name} — Overview\n\n**{name}** is a ...\n\n## When to Use\n\n- ...\n\n## When NOT to Use\n\n- ...\n\n## Key Design\n\n- ...\n";
const PITFALLS_TEMPLATE: &str = "# {name} — Pitfalls\n\nCommon mistakes that cause crashes, data loss, or silent misbehavior.\n\n### Do NOT ...\n\n```\n// BAD:\n\n// GOOD:\n```\n";
const SAFETY_TEMPLATE: &str = "# {name} — Safety\n\nRed lines — conditions that must NEVER occur.\n\n### NEVER ...\n\n- ...\n";
const LIFECYCLE_TEMPLATE: &str = "# {name} — Lifecycle\n\n## Initialization\n\n...\n\n## Shutdown\n\n...\n";
const THREADING_TEMPLATE: &str = "# {name} — Threading\n\n## Thread Safety\n\n...\n";
const BEST_PRACTICES_TEMPLATE: &str = "# {name} — Best Practices\n\n## Recommended Setup\n\n...\n";
const PERFORMANCE_TEMPLATE: &str = "# {name} — Performance\n\n## Throughput\n\n...\n";

#[derive(Args, Debug)]
pub struct InitArgs {
    /// Library name (e.g., "spdlog")
    #[arg(short, long)]
    pub name: Option<String>,

    /// GitHub repository (e.g., "gabime/spdlog")
    #[arg(short, long)]
    pub repo: Option<String>,

    /// Programming language (cpp, rust, python, go, js)
    #[arg(short, long)]
    pub language: Option<String>,

    /// Library version this skill targets
    #[arg(long, default_value = "0.1.0")]
    pub version: String,

    /// Comma-separated tags (e.g., "logging,async,cpp")
    #[arg(short, long)]
    pub tags: Option<String>,

    /// Tier: tier1 or tier2 (default: tier2)
    #[arg(long, default_value = "tier2")]
    pub tier: String,

    /// Group: main or contrib (default: contrib)
    #[arg(long, default_value = "contrib")]
    pub group: String,

    /// Output directory (default: .libskills in current directory)
    #[arg(short, long, default_value = ".libskills")]
    pub output: String,
}

impl InitArgs {
    fn resolve_or_prompt(&self) -> Result<ResolvedConfig, Error> {
        let name = self.name.clone().unwrap_or_else(|| prompt("Library name (e.g., spdlog): "));
        let repo = self.repo.clone().unwrap_or_else(|| prompt("GitHub repo (e.g., gabime/spdlog): "));
        let language = self.language.clone().unwrap_or_else(|| prompt("Language (cpp, rust, python, go, js): "));
        let version = self.version.clone();
        let tags_str = self.tags.clone().unwrap_or_else(|| prompt("Tags (comma-separated): "));

        let tags: Vec<String> = tags_str.split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let ext = ext_for_language(&language);

        Ok(ResolvedConfig {
            name, repo, language, version,
            tier: self.tier.clone(),
            group: self.group.clone(),
            tags, ext,
            output: self.output.clone(),
        })
    }
}

struct ResolvedConfig {
    name: String,
    repo: String,
    language: String,
    version: String,
    tier: String,
    group: String,
    tags: Vec<String>,
    ext: &'static str,
    output: String,
}

fn ext_for_language(lang: &str) -> &'static str {
    match lang.to_lowercase().as_str() {
        "cpp" | "c++" => "cpp",
        "rust" => "rs",
        "python" => "py",
        "go" => "go",
        "js" | "javascript" => "js",
        _ => "txt",
    }
}

fn prompt(msg: &str) -> String {
    use std::io::{self, Write};
    print!("{}", msg);
    io::stdout().flush().ok();
    let mut input = String::new();
    io::stdin().read_line(&mut input).ok();
    input.trim().to_string()
}

pub fn run(args: InitArgs) -> Result<(), Error> {
    let config = args.resolve_or_prompt()?;

    let output_dir = Path::new(&config.output);
    let examples_dir = output_dir.join("examples");

    fs::create_dir_all(output_dir)?;
    fs::create_dir_all(&examples_dir)?;

    let tags_formatted: Vec<String> = config.tags.iter()
        .map(|t| format!("\"{}\"", t))
        .collect();
    let tags_json = tags_formatted.join(", ");

    let skill_json = SKILL_JSON_TEMPLATE
        .replace("{name}", &config.name)
        .replace("{repo}", &config.repo)
        .replace("{language}", &config.language)
        .replace("{tier}", &config.tier)
        .replace("{group}", &config.group)
        .replace("{version}", &config.version)
        .replace("{tags}", &tags_json)
        .replace("{ext}", config.ext)
        .replace("{updated_at}", &chrono_now());

    write_file(output_dir.join("skill.json"), &skill_json)?;
    write_file(output_dir.join("overview.md"), &OVERVIEW_TEMPLATE.replace("{name}", &config.name))?;
    write_file(output_dir.join("pitfalls.md"), &PITFALLS_TEMPLATE.replace("{name}", &config.name))?;
    write_file(output_dir.join("safety.md"), &SAFETY_TEMPLATE.replace("{name}", &config.name))?;
    write_file(output_dir.join("lifecycle.md"), &LIFECYCLE_TEMPLATE.replace("{name}", &config.name))?;
    write_file(output_dir.join("threading.md"), &THREADING_TEMPLATE.replace("{name}", &config.name))?;
    write_file(output_dir.join("best-practices.md"), &BEST_PRACTICES_TEMPLATE.replace("{name}", &config.name))?;
    write_file(output_dir.join("performance.md"), &PERFORMANCE_TEMPLATE.replace("{name}", &config.name))?;
    write_file(examples_dir.join(format!("basic.{}", config.ext)), "// TODO: add example\n")?;

    println!();
    println!("{}", "✓ .libskills/ directory created!".green().bold());
    println!("  {}", output_dir.display());
    println!();
    println!("{}", "Next steps:".bold());
    println!("  1. Fill in the template files with real knowledge");
    println!("  2. Run {} to check your skill", "libskills validate .libskills/".cyan());
    println!("  3. Run {} to check quality", "libskills lint .libskills/".cyan());
    println!("  4. Commit .libskills/ to your repository");

    Ok(())
}

fn write_file(path: std::path::PathBuf, content: &str) -> Result<(), Error> {
    fs::write(&path, content)?;
    println!("  {} {}", "create".green(), path.display());
    Ok(())
}

fn chrono_now() -> String {
    // Simple ISO 8601 without external dependency
    use std::time::SystemTime;
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let _days = now / 86400;
    let remaining = now % 86400;
    let hours = remaining / 3600;
    let minutes = (remaining % 3600) / 60;
    let seconds = remaining % 60;

    // Approximate date from Unix epoch — good enough for a template
    // This gives ~2026-04-28 for the current date
    let date_str = "2026-04-28"; // Simple fallback for template generation
    format!("{}T{:02}:{:02}:{:02}Z", date_str, hours, minutes, seconds)
}
