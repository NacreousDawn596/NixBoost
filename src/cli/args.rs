// NixBoost - High-performance NixOS package manager frontend
// Copyright (C) 2025 nacreousdawn596, compiledkernel-idk and NixBoost contributors
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

//! CLI argument definitions for NixBoost.

use clap::{Parser, Subcommand, ValueEnum};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Parser)]
#[command(name = "nixboost")]
#[command(author = "NixBoost Team")]
#[command(version = VERSION)]
#[command(about = "High-performance, enterprise-grade NixOS package manager frontend")]
#[command(long_about = "NixBoost is a fast, user-friendly frontend for Nix package management.\n\n\
    It supports nixpkgs and NUR packages, provides intelligent caching,\n\
    parallel search, and a modern CLI experience.")]
pub struct Cli {
    /// Sync/install packages (like pacman -S)
    #[arg(short = 'S', long)]
    pub sync: bool,

    /// Remove packages (like pacman -R)
    #[arg(short = 'R', long)]
    pub remove: bool,

    /// Search packages (use with -S for nixpkgs search)
    #[arg(short = 's', long)]
    pub search: bool,

    /// Search/install from NUR (Nix User Repository)
    #[arg(short = 'A', long)]
    pub nur: bool,

    /// List installed packages
    #[arg(short = 'l', long)]
    pub list: bool,

    /// Show nix-env generation history
    #[arg(long)]
    pub history: bool,

    /// Run garbage collection
    #[arg(long)]
    pub clean: bool,

    /// Fetch NixOS news
    #[arg(long)]
    pub news: bool,

    /// Run system health check
    #[arg(long)]
    pub health: bool,

    /// Show this package's info
    #[arg(short = 'i', long)]
    pub info: bool,

    /// Dry run - don't actually perform operations
    #[arg(long)]
    pub dry_run: bool,

    /// Don't ask for confirmation
    #[arg(short = 'y', long)]
    pub yes: bool,

    /// Be verbose (show debug info)
    #[arg(short = 'v', long)]
    pub verbose: bool,

    /// Be quiet (minimal output)
    #[arg(short = 'q', long)]
    pub quiet: bool,

    /// Skip update check
    #[arg(long)]
    pub no_update_check: bool,

    /// Use specific config file
    #[arg(long, value_name = "FILE")]
    pub config: Option<String>,

    /// Maximum number of results to show
    #[arg(long, default_value = "50")]
    pub max_results: usize,

    /// Disable cache
    #[arg(long)]
    pub no_cache: bool,

    /// Clear cache before operation
    #[arg(long)]
    pub clear_cache: bool,

    /// Show cache statistics
    #[arg(long)]
    pub cache_stats: bool,

    /// Output format
    #[arg(long, value_enum, default_value = "human")]
    pub output: OutputFormat,

    /// Target packages or search queries
    #[arg(value_name = "TARGETS")]
    pub targets: Vec<String>,

    /// Subcommands for advanced operations
    #[command(subcommand)]
    pub command: Option<Commands>,
}

/// Output format options
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    /// Human-readable output with colors
    Human,
    /// JSON output for scripting
    Json,
    /// Plain text (no colors, simple format)
    Plain,
}

/// Advanced subcommands
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Show package information
    Info {
        /// Package name
        package: String,
    },

    /// Manage generations
    Generation {
        #[command(subcommand)]
        action: GenerationAction,
    },

    /// Cache management
    Cache {
        #[command(subcommand)]
        action: CacheAction,
    },

    /// Configuration management
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },

    /// System operations
    System {
        #[command(subcommand)]
        action: SystemAction,
    },

    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: Shell,
    },
}

/// Generation subcommands
#[derive(Subcommand, Debug)]
pub enum GenerationAction {
    /// List all generations
    List {
        /// Maximum number to show
        #[arg(short, long, default_value = "20")]
        limit: usize,
    },
    /// Show diff between generations
    Diff {
        /// First generation
        from: u64,
        /// Second generation
        to: u64,
    },
    /// Rollback to a specific generation
    Rollback {
        /// Generation number (omit for previous)
        generation: Option<u64>,
    },
    /// Delete old generations
    Delete {
        /// Keep last N generations
        #[arg(short, long, default_value = "5")]
        keep: usize,
    },
}

/// Cache subcommands
#[derive(Subcommand, Debug)]
pub enum CacheAction {
    /// Show cache statistics
    Stats,
    /// Clear all cache
    Clear,
    /// Verify cache integrity
    Verify,
    /// Prune expired entries
    Prune,
}

/// Config subcommands
#[derive(Subcommand, Debug)]
pub enum ConfigAction {
    /// Show current configuration
    Show,
    /// Generate default config file
    Init {
        /// Overwrite existing config
        #[arg(short, long)]
        force: bool,
    },
    /// Edit config file
    Edit,
    /// Validate config file
    Validate,
    /// Show config file path
    Path,
}

/// System subcommands
#[derive(Subcommand, Debug)]
pub enum SystemAction {
    /// Run health check
    Health,
    /// Run garbage collection
    Gc {
        /// Keep minimum generations
        #[arg(short, long, default_value = "3")]
        keep_generations: usize,
        /// Dry run (show what would be deleted)
        #[arg(short, long)]
        dry_run: bool,
    },
    /// Verify Nix store
    Verify,
    /// Optimize Nix store
    Optimize,
    /// Show disk usage
    DiskUsage,
}

/// Shell types for completion generation
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Shell {
    Bash,
    Zsh,
    Fish,
    PowerShell,
    Elvish,
}

impl Cli {
    /// Check if any operation is requested
    pub fn has_operation(&self) -> bool {
        self.sync || self.remove || self.search || self.nur || self.list || 
        self.history || self.clean || self.news || self.health || self.info ||
        self.cache_stats || self.command.is_some()
    }

    /// Check if this is a read-only operation
    pub fn is_read_only(&self) -> bool {
        self.search || self.list || self.history || self.news || self.health || 
        self.info || self.cache_stats || self.dry_run
    }

    /// Check if confirmation should be skipped
    pub fn skip_confirm(&self) -> bool {
        self.yes || self.dry_run
    }

    /// Get effective verbosity level
    pub fn verbosity(&self) -> Verbosity {
        if self.quiet {
            Verbosity::Quiet
        } else if self.verbose {
            Verbosity::Verbose
        } else {
            Verbosity::Normal
        }
    }
}

/// Verbosity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verbosity {
    Quiet,
    Normal,
    Verbose,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parsing() {
        let cli = Cli::parse_from(["nixboost", "-S", "firefox"]);
        assert!(cli.sync);
        assert_eq!(cli.targets, vec!["firefox"]);
    }

    #[test]
    fn test_search_parsing() {
        let cli = Cli::parse_from(["nixboost", "-Ss", "firefox"]);
        assert!(cli.sync);
        assert!(cli.search);
    }

    #[test]
    fn test_has_operation() {
        let cli = Cli::parse_from(["nixboost", "-S", "pkg"]);
        assert!(cli.has_operation());

        let cli_empty = Cli::parse_from(["nixboost"]);
        assert!(!cli_empty.has_operation());
    }

    #[test]
    fn test_is_read_only() {
        let cli = Cli::parse_from(["nixboost", "-Ss", "query"]);
        assert!(cli.is_read_only());

        let cli_install = Cli::parse_from(["nixboost", "-S", "pkg"]);
        assert!(!cli_install.is_read_only());
    }

    #[test]
    fn test_verbosity() {
        let cli = Cli::parse_from(["nixboost", "-v"]);
        assert_eq!(cli.verbosity(), Verbosity::Verbose);

        let cli_quiet = Cli::parse_from(["nixboost", "-q"]);
        assert_eq!(cli_quiet.verbosity(), Verbosity::Quiet);
    }

    #[test]
    fn test_dry_run() {
        let cli = Cli::parse_from(["nixboost", "-S", "--dry-run", "pkg"]);
        assert!(cli.dry_run);
        assert!(cli.is_read_only());
    }
}
