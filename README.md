# libskills-cli

**Rust CLI for the LibSkills ecosystem.**

```bash
libskills search cpp logging      # Fuzzy search registry
libskills get cpp/gabime/spdlog    # Download a skill
libskills info spdlog              # Show skill metadata
libskills update                   # Refresh registry index
libskills list                     # List cached skills
libskills cache                    # Manage local cache
libskills init                     # Generate skill template
libskills doctor spdlog            # Validate local skill
libskills find "async http"        # Semantic search (future)
libskills serve                    # HTTP/MCP server (future)
```

## Quickstart

```bash
cargo install libskills
libskills update
libskills search cpp logging
libskills get cpp/gabime/spdlog
```

## Commands

| Command | Description | Status |
|---------|-------------|--------|
| `search <keyword>` | Fuzzy search registry index | 🚧 |
| `get <path>[@version]` | Download skill to local cache | 🚧 |
| `info <path>` | Show skill metadata | 🚧 |
| `update` | Refresh registry index | 🚧 |
| `list` | List cached skills | 🚧 |
| `cache` | Manage local cache | 🚧 |
| `init` | Generate skill template | 📅 |
| `doctor <path>` | Validate and audit skill | 📅 |
| `find <intent>` | Semantic search | 📅 |
| `serve` | MCP/HTTP server on :8701 | 📅 |

## Local Cache

```
~/.libskills/
├── cache/
├── skills/
├── index.db
├── config.toml
└── logs/
```

## License

Apache 2.0
