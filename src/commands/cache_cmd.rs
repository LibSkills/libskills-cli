use clap::{Args, Subcommand};
use colored::*;

use crate::cache::Cache;
use crate::error::Error;

#[derive(Args, Debug)]
pub struct CacheArgs {
    #[command(subcommand)]
    pub action: CacheAction,
}

#[derive(Subcommand, Debug)]
pub enum CacheAction {
    /// Clear all cached skills
    Clear,
    /// Prune (same as clear — removes all cached skills)
    Prune,
    /// Show cache directory path
    Path,
}

pub fn run(args: CacheArgs) -> Result<(), Error> {
    let cache = Cache::new(Cache::default_location());

    match args.action {
        CacheAction::Clear | CacheAction::Prune => {
            let count = cache.prune_cache()?;
            println!("{} {} cached skill(s) removed.", "✓".green().bold(), count);
            println!("  Cache directory: {}", cache.cache_dir().display());
        }
        CacheAction::Path => {
            println!("Cache root:       {}", cache.root().display());
            println!("Cache directory:  {}", cache.cache_dir().display());
            println!("Index file:       {}", cache.index_path().display());
            println!("Config file:      {}", cache.config_path().display());
        }
    }

    Ok(())
}
