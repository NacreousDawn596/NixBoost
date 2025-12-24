# nixboost 🚀

**nixboost** is a fast, improved frontend for managing packages on **NixOS**.
It makes searching and installing packages from **nixpkgs** and **NUR** feel instant, predictable, and actually enjoyable.

Think *pacman UX*, but for NixOS — without breaking purity :3

---

## 🤔 Why NixBoost?

### The Problem with Raw Nix Commands

- `nix search` is slow and doesn't cache results 🐢
- `nix profile install` gives cryptic errors 😵
- No unified way to search nixpkgs AND NUR together
- Managing generations requires remembering multiple commands
- No progress feedback during long operations

### ✨ The NixBoost Solution

| Feature | Raw Nix | NixBoost |
|---------|---------|----------|
| Search speed | 2-5 seconds | **<100ms** (cached) ⚡ |
| NUR integration | Manual | **Automatic fallback** 🔄 |
| Batch install | Sequential | **Parallel** 🚀 |
| Progress feedback | None | **Real-time spinners** 🌀 |
| Error messages | Cryptic | **Human-readable + suggestions** 💡 |
| Configuration | None | **TOML config file** ⚙️ |
| Caching | None | **SQLite + LRU memory cache** 🗄️ |

---

## ✨ Features

### ⚡ Performance

- **Intelligent Caching** - SQLite disk cache + LRU memory cache for sub-100ms responses
- **Parallel Search** - Multi-core fuzzy search with rayon
- **Batch Operations** - Install multiple packages in a single transaction
- **Connection Pooling** - HTTP/2 with automatic retry and exponential backoff

### 📦 Package Management

- **Unified Search** - Search nixpkgs and NUR from one command
- **Smart Fallback** - If a package isn't in nixpkgs, automatically check NUR
- **Fuzzy Matching** - Find packages even with typos ("firefx" finds "firefox")
- **Dry Run Mode** - Preview what would happen without making changes
- **JSON Output** - Machine-readable output for scripting

### 🛠️ System Utilities

- **Health Checks** - Verify systemd services, Nix store integrity, daemon status
- **Smart Garbage Collection** - Preview space savings before cleaning
- **Generation Management** - List, diff, and rollback generations
- **NixOS News** - Stay updated with the latest blog posts

### 🧑‍💻 Developer Experience

- **TOML Configuration** - Customize behavior via `~/.config/nixboost/config.toml`
- **Shell Completions** - Tab completion for bash, zsh, fish, PowerShell, elvish
- **Structured Errors** - Error codes and recovery suggestions
- **Verbose/Quiet Modes** - Control output verbosity

---

## 📦 Installation

### Recommended (Flake-based)

```bash
nix profile install github:NacreousDawn596/nixboost
```

> Reproducible, rebuild-safe, zero `/usr/local` crimes. 😇

### From Source

```bash
git clone https://github.com/NacreousDawn596/nixboost
cd nixboost
nix-shell
cargo build --release
./target/release/nixboost --version
```

---

## 🧑‍💻 Usage

### Package Management

```bash
nixboost -S <pkg>           # Install package (checks nixpkgs, then NUR)
nixboost -S pkg1 pkg2 pkg3  # Install multiple packages (batch) 🚀
nixboost -R <pkg>           # Remove package with confirmation
nixboost -Ss <query>        # Search nixpkgs
nixboost -A <query>         # Search NUR
nixboost -l                 # List installed packages
```

### 🆕 New in v2.0

```bash
# Dry run - see what would happen
nixboost -S firefox --dry-run

# JSON output for scripting
nixboost -Ss browser --output json

# Skip confirmations
nixboost -R firefox --yes

# Verbose mode for debugging
nixboost -S firefox --verbose
```

### 🛠️ System Utilities

```bash
nixboost --news             # Latest NixOS news 📰
nixboost --history          # View generation history 📜
nixboost --health           # System health check 🏥
nixboost --clean            # Garbage collection 🧹
nixboost --clean --dry-run  # Preview what would be cleaned
```

### 🔄 Generation Management

```bash
nixboost generation list              # List all generations
nixboost generation diff 10 15        # Diff two generations
nixboost generation rollback          # Rollback to previous
nixboost generation rollback 10       # Rollback to specific generation
nixboost generation delete --keep 5   # Keep only last 5 generations
```

### 🗄️ Cache Management

```bash
nixboost --cache-stats      # Show cache statistics
nixboost cache clear        # Clear all cache
nixboost cache prune        # Remove expired entries
nixboost --no-cache -Ss vim # Search without using cache
```

### ⚙️ Configuration

```bash
nixboost config show        # Display current config
nixboost config init        # Generate default config file
nixboost config edit        # Open config in $EDITOR
nixboost config path        # Show config file location
```

### 🐚 Shell Completions

```bash
# Bash
nixboost completions bash > ~/.local/share/bash-completion/completions/nixboost

# Zsh
nixboost completions zsh > ~/.zfunc/_nixboost

# Fish
nixboost completions fish > ~/.config/fish/completions/nixboost.fish
```

---

## ⚙️ Configuration

NixBoost stores its configuration at `~/.config/nixboost/config.toml`:

```toml
[general]
verbose = false
check_updates = true

[search]
max_results = 50
fuzzy = true
fuzzy_threshold = 0.6

[cache]
enabled = true
max_size_mb = 500
package_ttl_secs = 3600
search_ttl_secs = 300

[network]
timeout_secs = 30
max_retries = 3

[ui]
colors = true
progress = true
unicode = true
```

### 🌍 Environment Variables

- `NIXBOOST_VERBOSE` - Enable verbose output
- `NIXBOOST_DEBUG` - Enable debug logging
- `NIXBOOST_NO_COLORS` - Disable colored output
- `NIXBOOST_NO_CACHE` - Disable caching
- `NIXBOOST_TIMEOUT` - Set network timeout in seconds
- `HTTP_PROXY` / `HTTPS_PROXY` - Configure proxy

---

## ⚡ Why It's Fast

1. **SQLite Caching** - Search results, package metadata, and NUR index are cached to disk with configurable TTL
2. **LRU Memory Cache** - Hot data stays in RAM for sub-millisecond access
3. **Parallel Processing** - Search uses all CPU cores via rayon
4. **Batch Operations** - Multiple packages are installed in a single `nix profile install` call
5. **Connection Reuse** - HTTP client pools connections and uses HTTP/2
6. **Lazy Loading** - Modules are loaded on-demand, keeping startup time under 100ms

### 📊 Benchmarks

| Operation | Raw Nix | NixBoost | Speedup |
|-----------|---------|----------|---------|
| Search (cold) | 3.2s | 0.8s | **4x** 🚀 |
| Search (cached) | 3.2s | 0.05s | **64x** ⚡ |
| Install 5 packages | 45s | 12s | **3.7x** 🔥 |
| List installed | 1.5s | 0.2s | **7.5x** 💨 |

---

## 🏗️ Architecture

```
src/
├── main.rs           # CLI orchestration
├── core/             # Config, errors, domain types
├── cli/              # Argument parsing, subcommands
├── cache/            # SQLite + LRU caching
├── search/           # Parallel fuzzy search
├── package/          # Package operations
├── nur/              # NUR integration
├── system/           # Health, GC, generations
├── network/          # HTTP with retry
└── ui/               # Progress, output formatting
```

- **5,500+ lines** of production Rust 🦀
- **54 unit tests** ✅
- **10 modular components** 📦
- **Zero unsafe code** 🔒

---

## 🧠 Philosophy

- **Fast feedback** - Never wait more than a second for common operations ⚡
- **Minimal friction** - Sensible defaults, optional configuration
- **No magic state** - Everything is reproducible and cacheable
- **Nix-first** - Respects NixOS semantics, never fights the system
- **Helpful errors** - Every error includes a suggestion for recovery 💡

If it doesn't respect NixOS semantics, it doesn't belong here.

---

## 🤝 Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

---

## 📜 License

**GPL-3.0**

Copyright (C) 2025 nacreousdawn596, compiledkernel-idk and NixBoost contributors

Built with too much caffeine ☕, exam pressure, zero sleep, and ❤️ for the NixOS community.
