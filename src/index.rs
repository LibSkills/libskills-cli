use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::cache::Cache;
use crate::error::Error;

#[derive(serde::Serialize, serde::Deserialize, Default)]
pub struct ContentIndex {
    /// Skill key -> map of term -> term frequency
    pub term_freq: HashMap<String, HashMap<String, f64>>,
    /// Term -> inverse document frequency
    pub idf: HashMap<String, f64>,
    /// Total number of documents indexed
    pub doc_count: usize,
}

impl ContentIndex {
    pub fn empty() -> Self {
        Self {
            term_freq: HashMap::new(),
            idf: HashMap::new(),
            doc_count: 0,
        }
    }

    pub fn load_from_cache(cache: &Cache) -> Result<Self, Error> {
        let path = cache.root().join("embedding.json");
        if path.exists() {
            let content = fs::read_to_string(&path)?;
            Ok(serde_json::from_str(&content)?)
        } else {
            Ok(Self::empty())
        }
    }

    pub fn save_to_cache(&self, cache: &Cache) -> Result<(), Error> {
        cache.ensure_dirs()?;
        let path = cache.root().join("embedding.json");
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, &content)?;
        Ok(())
    }

    pub fn build(&mut self, registry_path: &Path) -> Result<(), Error> {
        let skills_dir = registry_path.join("skills");
        if !skills_dir.exists() {
            return Ok(());
        }

        let mut doc_freq: HashMap<String, usize> = HashMap::new();
        let mut all_docs: Vec<(String, HashMap<String, f64>)> = Vec::new();

        self.collect_skills(&skills_dir, String::new(), &mut all_docs)?;

        // First pass: count document frequencies for IDF
        for (_, terms) in &all_docs {
            for term in terms.keys() {
                *doc_freq.entry(term.clone()).or_insert(0) += 1;
            }
        }

        // Compute IDF
        let n = all_docs.len() as f64;
        self.idf.clear();
        for (term, df) in &doc_freq {
            let idf = ((n + 1.0) / (*df as f64 + 1.0)).ln() + 1.0;
            self.idf.insert(term.clone(), idf);
        }

        // Store term frequencies
        self.term_freq.clear();
        for (_key, terms) in all_docs.iter() {
            for term in terms.keys() {
                *doc_freq.entry(term.clone()).or_insert(0) += 1;
            }
        }

        // Compute IDF
        let n = all_docs.len() as f64;
        self.idf.clear();
        for (term, df) in &doc_freq {
            let idf = ((n + 1.0) / (*df as f64 + 1.0)).ln() + 1.0;
            self.idf.insert(term.clone(), idf);
        }

        // Store term frequencies
        self.doc_count = all_docs.len();
        self.term_freq.clear();
        for (key, terms) in all_docs {
            self.term_freq.insert(key, terms);
        }
        Ok(())
    }

    fn collect_skills(
        &self,
        dir: &Path,
        prefix: String,
        skills: &mut Vec<(String, HashMap<String, f64>)>,
    ) -> Result<(), Error> {
        if dir.join("skill.json").exists() {
            // This is a skill directory
            let mut terms = HashMap::new();
            self.index_markdown_files(dir, &mut terms)?;
            skills.push((prefix, terms));
            return Ok(());
        }

        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                if entry.file_type().map_or(false, |t| t.is_dir()) {
                    let name = entry.file_name().to_string_lossy().to_string();
                    let new_prefix = if prefix.is_empty() {
                        name.clone()
                    } else {
                        format!("{}/{}", prefix, name)
                    };
                    self.collect_skills(&entry.path(), new_prefix, skills)?;
                }
            }
        }
        Ok(())
    }

    fn index_markdown_files(&self, dir: &Path, terms: &mut HashMap<String, f64>) -> Result<(), Error> {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(ext) = path.extension() {
                        if ext == "md" {
                            if let Ok(content) = fs::read_to_string(&path) {
                                let tokens = tokenize(&content);
                                for token in tokens {
                                    *terms.entry(token).or_insert(0.0) += 1.0;
                                }
                            }
                        }
                    }
                } else if path.is_dir() && path.file_name().map_or(false, |n| n != "examples") {
                    // Index subdirectories too (but skip examples)
                }
            }
        }
        Ok(())
    }

    pub fn search(&self, query: &str, top_k: usize) -> Vec<ScoredResult> {
        let query_tokens = tokenize(query);
        if query_tokens.is_empty() {
            return Vec::new();
        }

        // Build query vector (TF-IDF weighted)
        let mut query_vec: HashMap<String, f64> = HashMap::new();
        for token in &query_tokens {
            *query_vec.entry(token.clone()).or_insert(0.0) += 1.0;
        }
        // Normalize query
        let query_sum: f64 = query_vec.values().sum();
        if query_sum > 0.0 {
            for v in query_vec.values_mut() {
                *v /= query_sum;
            }
        }

        let mut results: Vec<ScoredResult> = Vec::new();

        for (key, doc_terms) in &self.term_freq {
            let mut score = 0.0;

            // Compute dot product between query vector and document vector (TF-IDF weighted)
            let doc_sum: f64 = doc_terms.values().sum();
            for (term, query_weight) in &query_vec {
                if let Some(doc_tf) = doc_terms.get(term) {
                    let idf = self.idf.get(term).copied().unwrap_or(1.0);
                    let doc_weight = if doc_sum > 0.0 { doc_tf / doc_sum } else { 0.0 };
                    score += query_weight * doc_weight * idf;
                }
            }

            if score > 0.0 {
                results.push(ScoredResult {
                    key: key.clone(),
                    score,
                });
            }
        }

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(top_k);
        results
    }
}

#[derive(Debug)]
pub struct ScoredResult {
    pub key: String,
    pub score: f64,
}

fn tokenize(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric() && c != '-' && c != '_')
        .filter(|s| !s.is_empty())
        .filter(|s| s.len() >= 2)
        .map(|s| s.to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_simple() {
        let tokens = tokenize("Hello World");
        assert!(tokens.contains(&"hello".to_string()));
        assert!(tokens.contains(&"world".to_string()));
    }

    #[test]
    fn test_tokenize_filters_short() {
        let tokens = tokenize("a bc def gh");
        assert!(!tokens.contains(&"a".to_string()));
        assert!(tokens.contains(&"bc".to_string()));
        assert!(tokens.contains(&"def".to_string()));
        assert!(tokens.contains(&"gh".to_string()));
    }

    #[test]
    fn test_content_index_build_and_search() {
        let mut index = ContentIndex::empty();
        // Simulate indexing — direct insertion into term_freq
        let mut terms1 = std::collections::HashMap::new();
        terms1.insert("logging".to_string(), 3.0);
        terms1.insert("async".to_string(), 2.0);
        terms1.insert("fast".to_string(), 1.0);
        index.term_freq.insert("cpp/spdlog".into(), terms1);

        let mut terms2 = std::collections::HashMap::new();
        terms2.insert("serialization".to_string(), 4.0);
        terms2.insert("json".to_string(), 3.0);
        index.term_freq.insert("rust/serde".into(), terms2);

        index.doc_count = 2;
        index.idf.insert("logging".into(), 1.2);
        index.idf.insert("async".into(), 1.2);
        index.idf.insert("fast".into(), 1.5);
        index.idf.insert("serialization".into(), 1.2);
        index.idf.insert("json".into(), 1.0);

        let results = index.search("async logging", 10);
        assert!(!results.is_empty());
        assert_eq!(results[0].key, "cpp/spdlog");
    }

    #[test]
    fn test_content_index_empty() {
        let index = ContentIndex::empty();
        let results = index.search("anything", 10);
        assert!(results.is_empty());
    }
}
