# libskills-cli

**Rust CLI for discovering and managing library skills.**

Part of the [LibSkills](https://github.com/LibSkills) ecosystem — the Behavioral Knowledge Layer for open-source libraries.

## Status

🚧 **Phase 2 on the roadmap.** CLI is scaffolded. Actively being implemented.

## Commands

| Command | Description | Status |
|---------|-------------|--------|
| `search <keyword>` | Fuzzy search registry index | 🚧 |
| `get <path>[@version]` | Download skill to local cache | 🚧 |
| `info <path>` | Show skill metadata | 🚧 |
| `update` | Refresh registry index from upstream | 🚧 |
| `list` | List locally cached skills | 🚧 |
| `cache` | Manage local cache (prune, clear) | 🚧 |
| `init` | Generate a skill template for contributors | 📅 Phase 6 |
| `doctor <path>` | Validate a local skill against the schema | 📅 Phase 6 |
| `find <intent>` | Semantic search | 📅 Phase 7 |
| `serve` | MCP/HTTP server | 📅 Phase 10 |

## Quickstart

```bash
cargo install libskills
libskills update
libskills search cpp logging
libskills get cpp/gabime/spdlog
```

## Local Cache

```
~/.libskills/
├── cache/       # Downloaded skill files
├── index.json   # Local registry index
└── config.toml  # CLI configuration
```

## License

Apache 2.0
