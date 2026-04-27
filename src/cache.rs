use std::fs;
use std::path::PathBuf;

use crate::error::Error;

#[derive(Clone)]
pub struct Cache {
    root: PathBuf,
}

impl Cache {
    pub fn default_location() -> PathBuf {
        let home = dirs_next_home();
        home.join(".libskills")
    }

    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn root(&self) -> &std::path::Path {
        &self.root
    }

    pub fn index_path(&self) -> PathBuf {
        self.root.join("index.json")
    }

    pub fn config_path(&self) -> PathBuf {
        self.root.join("config.toml")
    }

    pub fn cache_dir(&self) -> PathBuf {
        self.root.join("cache")
    }

    pub fn skill_dir(&self, key: &str) -> PathBuf {
        self.cache_dir().join(key)
    }

    pub fn ensure_dirs(&self) -> Result<(), Error> {
        fs::create_dir_all(&self.root)?;
        fs::create_dir_all(self.cache_dir())?;
        Ok(())
    }

    pub fn list_cached(&self) -> Result<Vec<String>, Error> {
        let cache = self.cache_dir();
        if !cache.exists() {
            return Ok(Vec::new());
        }

        let mut skills = Vec::new();
        self.collect_skills(&cache, 0, String::new(), &mut skills)?;
        skills.sort();
        Ok(skills)
    }

    fn collect_skills(&self, dir: &std::path::Path, depth: usize, prefix: String, skills: &mut Vec<String>) -> Result<(), Error> {
        if depth >= 3 {
            return Ok(());
        }
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                let name = entry.file_name().to_string_lossy().to_string();
                let new_prefix = if prefix.is_empty() { name.clone() } else { format!("{}/{}", prefix, name) };
                if depth == 2 {
                    // At depth 2 (name), this is a skill directory
                    // Verify it contains skill.json
                    if entry.path().join("skill.json").exists() {
                        skills.push(new_prefix);
                    }
                } else {
                    self.collect_skills(&entry.path(), depth + 1, new_prefix, skills)?;
                }
            }
        }
        Ok(())
    }

    pub fn prune_cache(&self) -> Result<usize, Error> {
        let cache = self.cache_dir();
        if !cache.exists() {
            return Ok(0);
        }
        let mut count = 0;
        self.remove_skills(&cache, &mut count)?;
        Ok(count)
    }

    fn remove_skills(&self, dir: &std::path::Path, count: &mut usize) -> Result<(), Error> {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                if entry.file_type().map_or(false, |t| t.is_dir()) {
                    if entry.path().join("skill.json").exists() {
                        fs::remove_dir_all(entry.path())?;
                        *count += 1;
                    } else {
                        self.remove_skills(&entry.path(), count)?;
                    }
                }
            }
        }
        Ok(())
    }
}

fn dirs_next_home() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home);
    }
    if let Ok(home) = std::env::var("USERPROFILE") {
        return PathBuf::from(home);
    }
    PathBuf::from(".")
}
