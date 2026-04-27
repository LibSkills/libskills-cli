# libskills-cli

**Rust CLI for the LibSkills ecosystem — Behavioral Knowledge Layer for Open-Source Libraries.**

Part of the [LibSkills](https://github.com/LibSkills) ecosystem.

## Status

✅ **v0.1.0** — All Phases 2-9 implemented. 11 commands, HTTP API server, semantic search.

## Commands

| Command | Description |
|---------|-------------|
| `init` | Scaffold a `.libskills/` directory with templates |
| `validate <path>` | Validate a skill against the schema |
| `lint <path> [--fix]` | Check quality (tokens, sections, completeness) + auto-repair |
| `update` | Update registry index and build content search index |
| `search <keyword>` | Keyword search registry (name, tags, summary) |
| `find <query>` | Semantic search skill content (TF-IDF) |
| `get <key>` | Download skill to local cache |
| `info <key>` | Show skill metadata |
| `list [-v]` | List cached skills |
| `cache {clear,prune,path}` | Manage local cache |
| `serve [--port 8701]` | Start HTTP API server (6 endpoints) |

## Quickstart

```bash
cargo install libskills

# Create a skill
libskills init --name mylib --repo me/mylib --language python --tags "example"
libskills validate .libskills/
libskills lint .libskills/

# Discover skills
libskills update
libskills search logging
libskills find "fast C++ logger"

# Download and inspect
libskills get cpp/gabime/spdlog
libskills info cpp/gabime/spdlog
libskills list -v

# Start API server
libskills serve
```

## Local Cache

```
~/.libskills/
├── index.json       # Local registry index
├── embedding.json   # Content search index
├── config.toml      # CLI configuration (future)
└── cache/           # Downloaded skills
    └── {lang}/{author}/{name}/
```

## HTTP API

```bash
libskills serve --port 8701
```

| Endpoint | Description |
|----------|-------------|
| `GET /health` | Health check |
| `GET /v1/skills` | List all skills |
| `GET /v1/skills/:lang/:author/:name` | Get full skill with contents |
| `GET /v1/skills/:lang/:author/:name/:section` | Get single section |
| `GET /v1/search?q=` | Keyword search |
| `GET /v1/find?q=` | Semantic search |

## License

Apache 2.0
