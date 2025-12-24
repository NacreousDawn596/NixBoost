# Changelog

## [2.0.0] - 2025-12-24 - Major Improvement

### Stats
- **Lines of Code**: 548 -> **5,494** (10x increase)
- **Files**: 7 -> **29** (4x increase)
- **Tests**: 0 -> **54** 
- **Modules**: 1 flat -> **10 domain-driven modules**

### Complete Architecture Overhaul

Transformed NixBoost from a simple CLI utility into an improved, modular package manager with domain-driven design:

```
src/
├── main.rs (584 LOC) - Slim orchestration layer
├── core/              - Configuration, errors, domain types
├── cli/               - Extended CLI with subcommands
├── cache/             - SQLite + LRU caching layer
├── search/            - Parallel fuzzy search engine
├── package/           - Package management core
├── nur/               - NUR integration
├── system/            - Health checks, GC, generations
├── ui/                - Progress bars, output formatting
├── network/           - HTTP client with retry logic
└── utils/             - Self-update, news fetcher
```

### New Features

#### Configuration System (src/core/config.rs)
- **TOML-based configuration** at `~/.config/nixboost/config.toml`
- **XDG Base Directory compliance** for config, cache, and data
- **Environment variable overrides**: `NIXBOOST_VERBOSE`, `NIXBOOST_DEBUG`, `NIXBOOST_NO_COLORS`, `NIXBOOST_NO_CACHE`, `NIXBOOST_TIMEOUT`
- **Proxy support** via `HTTP_PROXY`/`HTTPS_PROXY`
- **Hot-reload ready** configuration structure
- Configurable sections: `[general]`, `[search]`, `[cache]`, `[network]`, `[ui]`

#### Structured Error Handling (src/core/error.rs)
- **Custom error types** with `thiserror`: `PackageError`, `NetworkError`, `CacheError`, `SystemError`, `SearchError`, `NurError`
- **Error codes** for scripting (E001, E010, E020, etc.)
- **Recovery suggestions** for common errors
- **Retryable error detection** for automatic retry logic
- No more bare `anyhow!` calls - all errors are structured

#### Domain Types (src/core/types.rs)
- `Package`, `PackageSource` (Nixpkgs, NUR, Flake)
- `InstalledPackage`, `SearchResult`, `MatchType`
- `OperationStatus`, `OperationType`, `OperationResult`
- `Generation`, `GCPreview`

#### Intelligent Caching Layer (src/cache/)
- **SQLite disk cache** (disk_cache.rs)
  - Persistent storage at `~/.cache/nixboost/cache.db`
  - TTL-based expiration
  - Hit/miss tracking with statistics
  - Prefix deletion for invalidation
  - WAL mode for performance
  - Automatic pruning of expired entries
  - `VACUUM` support for space reclamation
- **LRU memory cache** (memory_cache.rs)
  - Sub-millisecond lookups
  - Configurable capacity
  - Automatic eviction
- **Cache invalidation** (invalidation.rs)
  - TTL constants for different data types
  - Cache key builders
  - Global invalidation support

#### Parallel Fuzzy Search Engine (src/search/engine.rs)
- **Parallel search** with `rayon` for multi-core utilization
- **Fuzzy matching** with `fuzzy-matcher` (Skim algorithm)
- **Multi-tier matching**: ExactName -> NamePrefix -> NameContains -> DescriptionContains -> Fuzzy
- **Relevance scoring** (0.0-1.0)
- **"Did you mean...?"** suggestions for typos
- **Quick search mode** for exact matches only
- **Multi-source search** (nixpkgs + NUR in parallel)

#### Extended CLI (src/cli/args.rs)
New subcommands:
- `nixboost generation list` - List all generations
- `nixboost generation diff <from> <to>` - Diff between generations
- `nixboost generation rollback [gen]` - Rollback to generation
- `nixboost generation delete --keep N` - Delete old generations
- `nixboost cache stats` - Show cache statistics
- `nixboost cache clear` - Clear all cache
- `nixboost cache prune` - Prune expired entries
- `nixboost config show` - Show current configuration
- `nixboost config init` - Generate default config
- `nixboost config edit` - Edit config with $EDITOR
- `nixboost config path` - Show config file path
- `nixboost system health` - Run health check
- `nixboost system gc` - Garbage collection with options
- `nixboost system verify` - Verify Nix store
- `nixboost system optimize` - Optimize Nix store
- `nixboost completions <shell>` - Generate shell completions (bash, zsh, fish, powershell, elvish)

New flags:
- `--dry-run` - Preview operations without executing
- `--yes` / `-y` - Skip confirmation prompts
- `--verbose` / `-v` - Debug output
- `--quiet` / `-q` - Minimal output
- `--no-update-check` - Skip self-update check
- `--config <FILE>` - Use custom config file
- `--max-results <N>` - Limit search results (default: 50)
- `--no-cache` - Disable caching
- `--clear-cache` - Clear cache before operation
- `--cache-stats` - Show cache statistics
- `--output <FORMAT>` - Output format: `human`, `json`, `plain`

#### Package Manager Enhancements (src/package/manager.rs)
- **Cache integration** for search results and installed packages
- **Parallel installation** with configurable concurrency
- **Dry-run checking** via `check_packages()`
- **Package existence verification**
- **Detailed package info** retrieval

#### NUR Client Improvements (src/nur/client.rs)
- **Cache integration** for NUR index
- **Async index loading** with file caching
- **Package resolution** for short names
- **NurPackage type** with full metadata

#### System Utilities (src/system/)
- **Health Checker** (health.rs)
  - Systemd service status
  - Nix store integrity verification
  - Nix daemon status
  - Disk space warnings
  - Comprehensive `HealthReport`
- **Garbage Collector** (garbage_collector.rs)
  - **Preview mode** - see what would be deleted
  - Keep N generations option
  - Delete older than X days
  - Space calculation
  - Pretty output with size formatting
- **Generation Manager** (generations.rs)
  - List generations with metadata
  - Diff between generations
  - Rollback to specific generation
  - Delete old generations
  - Pretty table output

#### UI Improvements (src/ui/)
- **Progress bars** (progress.rs)
  - Spinner for indeterminate operations
  - Bar for determinate operations
  - Download progress with ETA
  - Multi-progress support
- **Output formatting** (output.rs)
  - Human-readable (colored)
  - JSON (for scripting)
  - Plain text (no colors)
  - Unified interface for all output

#### Network Layer (src/network/client.rs)
- **HTTP client** with reqwest
- **Automatic retry** with exponential backoff
- **Rate limit handling** (429 response)
- **Timeout configuration**
- **Proxy support**
- **Connection pooling**

#### Tracing and Logging
- **Structured logging** with `tracing`
- **Environment-based filtering** (`RUST_LOG`)
- **Level-based output**: DEBUG, INFO, WARN, ERROR
- Integration with `tracing-subscriber`

### Dependencies Added

```toml
# Core
tracing = "0.1"
tracing-subscriber = "0.3"
tracing-appender = "0.2"
thiserror = "1.0"
toml = "0.8"
dirs = "5.0"

# Performance
dashmap = "5.5"
parking_lot = "0.12"
rayon = "1.10"
lru = "0.12"

# Database
rusqlite = "0.31" (with bundled SQLite)

# Search
fuzzy-matcher = "0.3"

# Graph
petgraph = "0.6"

# CLI
clap_complete = "4.5"

# Dev
tempfile = "3.10"
criterion = "0.5"
pretty_assertions = "1.4"
```

### Testing
- **54 unit tests** covering:
  - Configuration parsing and serialization
  - Error types and codes
  - Cache operations (get, set, expire, delete, prefix delete, stats)
  - Memory cache LRU behavior
  - Search engine matching algorithms
  - Version comparison
  - Health checks
  - Generation parsing
  - GC size formatting

### 100% Backward Compatible

All original commands work exactly as before:
```bash
nixboost -S firefox      # Install packages
nixboost -R firefox      # Remove packages  
nixboost -Ss browser     # Search nixpkgs
nixboost -A package      # Search NUR
nixboost -l              # List installed
nixboost --health        # Health check
nixboost --clean         # Garbage collect
nixboost --news          # NixOS news
nixboost --history       # Generation history
```

### New File Structure

```
src/
├── main.rs                    # 584 LOC - Orchestration
├── core/
│   ├── mod.rs                 # Module exports
│   ├── config.rs              # 362 LOC - TOML configuration
│   ├── error.rs               # 351 LOC - Structured errors
│   └── types.rs               # 391 LOC - Domain types
├── cli/
│   ├── mod.rs                 # Module exports
│   └── args.rs                # 344 LOC - CLI definitions
├── cache/
│   ├── mod.rs                 # CacheManager
│   ├── disk_cache.rs          # 362 LOC - SQLite cache
│   ├── memory_cache.rs        # 220 LOC - LRU cache
│   └── invalidation.rs        # TTL constants
├── search/
│   ├── mod.rs                 # Module exports
│   └── engine.rs              # 394 LOC - Fuzzy search
├── package/
│   ├── mod.rs                 # Module exports
│   └── manager.rs             # 346 LOC - Package ops
├── nur/
│   ├── mod.rs                 # Module exports
│   └── client.rs              # 291 LOC - NUR client
├── system/
│   ├── mod.rs                 # Module exports
│   ├── health.rs              # 209 LOC - Health checks
│   ├── garbage_collector.rs   # 250 LOC - Smart GC
│   └── generations.rs         # 313 LOC - Generations
├── ui/
│   ├── mod.rs                 # Module exports
│   ├── progress.rs            # Progress bars
│   └── output.rs              # 241 LOC - Formatting
├── network/
│   ├── mod.rs                 # Module exports
│   └── client.rs              # 196 LOC - HTTP client
└── utils/
    ├── mod.rs                 # Module exports
    ├── updater.rs             # Self-update
    └── news.rs                # NixOS news
```

---

## [1.2.0] - 2025-12-24
### Improved
- **Batch Operations**: Major performance boost by grouping package installations and removals into single transactions instead of sequential loops.
- **Architecture Support**: Removed hardcoded `x86_64-linux` dependency. `nixboost` now dynamically detects system architecture (e.g., `aarch64-darwin`), making it compatible with Apple Silicon and ARM devices.
- **Async I/O**: Refactored core package manager logic to use asynchronous process execution for better responsiveness.
- **User Interface**: Replaced basic text prompts with interactive `dialoguer` menus for cleaner confirmations.

## [1.1.0] - 2025-12-23
### Added
- **NixOS Support**: Complete pivot from Arch Linux to NixOS.
- **NUR Support**: Added search and installation support for the Nix User Repository.
- **NixManager**: New backend using `nix search` and `nix-env`.
- **NixOS News**: Added news reader to fetch from the NixOS blog.
- **Generation History**: Added history viewer to show Nix generations.
- **Nix Health Check**: Added health check for Nix store integrity.
- **Garbage Collection**: Added support for `nix-collect-garbage`.

## [1.0.0] - 2025-12-23
### Added
- Initial release of `nixboost`.
