mod cache;
mod commands;
mod error;
mod index;

use clap::{Parser, Subcommand};
use colored::*;

#[derive(Parser)]
#[command(name = "libskills")]
#[command(version = "0.1.0")]
#[command(about = "CLI for the LibSkills ecosystem — Behavioral Knowledge Layer for Open-Source Libraries", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Scaffold a .libskills/ directory with templates
    Init(commands::init::InitArgs),

    /// Validate a skill against the schema
    Validate(commands::validate::ValidateArgs),

    /// Check skill quality (token counts, required files, completeness)
    Lint(commands::lint::LintArgs),

    /// Update the local registry index
    Update(commands::update::UpdateArgs),

    /// Search the registry index for skills
    Search(commands::search::SearchArgs),

    /// Download a skill to the local cache
    Get(commands::get::GetArgs),

    /// Show metadata for a cached skill
    Info(commands::info::InfoArgs),

    /// List cached skills
    List(commands::list::ListArgs),

    /// Manage the local cache
    Cache(commands::cache_cmd::CacheArgs),

    /// Semantic search using content indexing (TF-IDF)
    Find(commands::find::FindArgs),

    /// Start HTTP API server for AI agent integration
    Serve(commands::serve::ServeArgs),
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Init(args) => commands::init::run(args),
        Commands::Validate(args) => commands::validate::run(args),
        Commands::Lint(args) => commands::lint::run(args),
        Commands::Update(args) => commands::update::run(args),
        Commands::Search(args) => commands::search::run(args),
        Commands::Get(args) => commands::get::run(args),
        Commands::Info(args) => commands::info::run(args),
        Commands::List(args) => commands::list::run(args),
        Commands::Cache(args) => commands::cache_cmd::run(args),
        Commands::Find(args) => commands::find::run(args),
        Commands::Serve(args) => commands::serve::run(args),
    };

    if let Err(e) = result {
        eprintln!("{} {}", "error:".red().bold(), e);
        std::process::exit(1);
    }
}
