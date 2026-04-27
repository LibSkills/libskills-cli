use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::get,
    Router,
};
use clap::Args;
use colored::*;
use serde_json::{json, Value};

use crate::cache::Cache;
use crate::error::Error;
use crate::index::ContentIndex;

#[derive(Args, Debug)]
pub struct ServeArgs {
    /// Port to listen on
    #[arg(short, long, default_value = "8701")]
    pub port: u16,

    /// Host to bind to
    #[arg(short = 'H', long, default_value = "127.0.0.1")]
    pub host: String,

    /// Path to the registry directory
    #[arg(short, long)]
    registry: Option<String>,
}

#[derive(Clone)]
struct AppState {
    registry_path: PathBuf,
    cache: Cache,
    index: Arc<ContentIndex>,
}

pub fn run(args: ServeArgs) -> Result<(), Error> {
    let cache = Cache::new(Cache::default_location());
    cache.ensure_dirs()?;

    let registry_path = resolve_registry_path(args.registry)?;

    // Build content index
    let mut index = ContentIndex::load_from_cache(&cache).unwrap_or_default();
    if index.doc_count == 0 {
        index.build(&registry_path)?;
        index.save_to_cache(&cache)?;
    }

    let state = AppState {
        registry_path,
        cache,
        index: Arc::new(index),
    };

    let addr = format!("{}:{}", args.host, args.port);

    let app = Router::new()
        .route("/health", get(health))
        .route("/v1/skills", get(list_skills))
        .route("/v1/skills/:lang/:author/:name", get(get_skill))
        .route("/v1/skills/:lang/:author/:name/:section", get(get_section))
        .route("/v1/search", get(search))
        .route("/v1/find", get(find))
        .with_state(state);

    println!("{}", "LibSkills HTTP API".bold());
    println!("  Listening on http://{}", addr.cyan());
    println!();
    println!("  Endpoints:");
    println!("    GET /health");
    println!("    GET /v1/skills");
    println!("    GET /v1/skills/{{lang}}/{{author}}/{{name}}");
    println!("    GET /v1/skills/{{lang}}/{{author}}/{{name}}/{{section}}");
    println!("    GET /v1/search?q={{keyword}}");
    println!("    GET /v1/find?q={{intent}}");
    println!();
    println!("  Press Ctrl+C to stop.");

    let rt = tokio::runtime::Runtime::new().map_err(|e| Error::Schema(e.to_string()))?;
    rt.block_on(async {
        let listener = tokio::net::TcpListener::bind(&addr).await.map_err(|e| Error::Schema(e.to_string()))?;
        axum::serve(listener, app).await.map_err(|e| Error::Schema(e.to_string()))
    })?;

    Ok(())
}

async fn health() -> Json<Value> {
    Json(json!({"status": "ok", "version": "0.1.0"}))
}

async fn list_skills(State(state): State<AppState>) -> Result<Json<Value>, (StatusCode, String)> {
    let index_path = state.cache.index_path();
    if !index_path.exists() {
        return Err((StatusCode::NOT_FOUND, "No registry index. Run 'libskills update'.".into()));
    }
    let content = fs::read_to_string(&index_path).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let index: Value = serde_json::from_str(&content).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(index))
}

async fn get_skill(
    State(state): State<AppState>,
    Path((lang, author, name)): Path<(String, String, String)>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let key = format!("{}/{}/{}", lang, author, name);
    let skill_dir = state.registry_path.join("skills").join(&key);

    let skill_json = skill_dir.join("skill.json");
    if !skill_json.exists() {
        return Err((StatusCode::NOT_FOUND, format!("Skill '{}' not found", key)));
    }

    let content = fs::read_to_string(&skill_json).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let mut skill: Value = serde_json::from_str(&content).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Collect file contents
    let mut files = json!({});
    if let Some(file_map) = skill.get("files") {
        for priority in &["P0", "P1", "P2", "P3"] {
            if let Some(list) = file_map.get(priority).and_then(|l| l.as_array()) {
                for f in list {
                    if let Some(filename) = f.as_str() {
                        let path = skill_dir.join(filename);
                        if let Ok(text) = fs::read_to_string(&path) {
                            files[filename] = json!(text);
                        }
                    }
                }
            }
        }
    }
    skill["_contents"] = files;

    Ok(Json(skill))
}

async fn get_section(
    State(state): State<AppState>,
    Path((lang, author, name, section)): Path<(String, String, String, String)>,
) -> Result<String, (StatusCode, String)> {
    let key = format!("{}/{}/{}", lang, author, name);
    let file_path = state.registry_path.join("skills").join(&key).join(&section);

    // Try with .md extension if not present
    let file_path = if file_path.exists() {
        file_path
    } else {
        let with_ext = state.registry_path.join("skills").join(&key).join(format!("{}.md", section));
        if with_ext.exists() { with_ext } else { file_path }
    };

    if !file_path.exists() {
        return Err((StatusCode::NOT_FOUND, format!("Section '{}' not found in skill '{}'", section, key)));
    }

    fs::read_to_string(&file_path).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

#[derive(serde::Deserialize)]
struct SearchQuery {
    q: String,
}

async fn search(
    State(state): State<AppState>,
    Query(query): Query<SearchQuery>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let index_path = state.cache.index_path();
    if !index_path.exists() {
        return Err((StatusCode::NOT_FOUND, "No registry index. Run 'libskills update'.".into()));
    }

    let content = fs::read_to_string(&index_path).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let index: Value = serde_json::from_str(&content).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let keyword = query.q.to_lowercase();

    let skills = index.get("skills").and_then(|s| s.as_array());

    let mut results: Vec<(Value, u32)> = Vec::new();
    if let Some(skills) = skills {
        for skill in skills {
            let name = skill["name"].as_str().map(|s| s.to_lowercase()).unwrap_or_default();
            let tags: Vec<String> = skill["tags"].as_array()
                .map(|a| a.iter().filter_map(|t| t.as_str().map(|s| s.to_lowercase())).collect())
                .unwrap_or_default();
            let summary = skill["summary"].as_str().unwrap_or("").to_lowercase();

            let mut score = 0u32;
            if name == keyword { score += 100; }
            if name.contains(&keyword) { score += 50; }
            if tags.iter().any(|t| t.contains(&keyword)) { score += 30; }
            if summary.contains(&keyword) { score += 20; }

            if score > 0 {
                results.push((skill.clone(), score));
            }
        }
    }

    results.sort_by(|a, b| b.1.cmp(&a.1));

    let output: Vec<Value> = results.into_iter().map(|(mut skill, score)| {
        skill["_score"] = json!(score);
        skill
    }).collect();

    Ok(Json(json!({"results": output, "query": query.q})))
}

#[derive(serde::Deserialize)]
struct FindQuery {
    q: String,
    #[serde(default = "default_limit")]
    limit: usize,
}

fn default_limit() -> usize { 10 }

async fn find(
    State(state): State<AppState>,
    Query(query): Query<FindQuery>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let results = state.index.search(&query.q, query.limit);
    let max_score = results.first().map(|r| r.score).unwrap_or(0.0);

    let output: Vec<Value> = results.iter().map(|r| {
        let normalized = if max_score > 0.0 { r.score / max_score * 100.0 } else { 0.0 };
        json!({
            "key": r.key,
            "score": normalized,
            "raw_score": r.score,
        })
    }).collect();

    Ok(Json(json!({"results": output, "query": query.q})))
}

fn resolve_registry_path(user_path: Option<String>) -> Result<PathBuf, Error> {
    if let Some(path) = user_path {
        let pb = PathBuf::from(&path);
        if pb.exists() { return Ok(pb); }
        return Err(Error::Schema(format!("Registry path '{}' not found", path)));
    }

    let exe_path = std::env::current_exe().unwrap_or_default();
    let mut search = exe_path.clone();
    for _ in 0..6 {
        search = match search.parent() { Some(p) => p.to_path_buf(), None => break };
        let candidate = search.join("libskills-registry");
        if candidate.exists() { return Ok(candidate); }
    }

    let cwd = std::env::current_dir().unwrap_or_default();
    for ancestor in cwd.ancestors().take(6) {
        let candidate = ancestor.join("libskills-registry");
        if candidate.exists() { return Ok(candidate); }
    }

    Err(Error::Schema("Could not find libskills-registry. Use --registry <path>.".into()))
}
