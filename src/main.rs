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

/*
 * NixBoost v2.0 - Enterprise-Grade NixOS Package Manager Frontend
 * 
 * A high-performance, modular package management tool for NixOS with:
 * - Intelligent caching (SQLite + LRU)
 * - Parallel fuzzy search
 * - NUR integration
 * - Comprehensive system utilities
 */

use anyhow::Result;
use clap::Parser;
use console::style;
use dialoguer::{Confirm, theme::ColorfulTheme};
use std::time::Duration;
use tracing::{debug, info, warn, Level};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

// Module declarations
mod cli;
mod core;
mod cache;
mod search;
mod package;
mod nur;
mod system;
mod network;
mod ui;
mod utils;

use cli::{Cli, Commands, VERSION};
use cli::args::OutputFormat;
use core::config::Config;
use package::PackageManager;
use nur::NurClient;
use system::{HealthChecker, GarbageCollector, GenerationManager};
use ui::output::Output;
use ui::progress;
use utils::{check_for_updates, perform_update, fetch_nixos_news};

#[tokio::main]
async fn main() -> Result<()> {
    // Parse CLI arguments
    let cli = Cli::parse();

    // Initialize logging
    init_logging(&cli);

    // Initialize configuration
    let config = match Config::load() {
        Ok(c) => c.with_env_overrides(),
        Err(e) => {
            warn!("Failed to load config, using defaults: {}", e);
            Config::default()
        }
    };

    // Initialize output formatter
    let output = Output::new(cli.output)
        .no_colors(!config.ui.colors || cli.output == OutputFormat::Plain);

    // Check for updates (unless skipped)
    if config.general.check_updates && !cli.no_update_check && !cli.quiet {
        check_and_prompt_update(&cli)?;
    }

    // Handle subcommands first
    if let Some(ref cmd) = cli.command {
        return handle_subcommand(cmd, &output).await;
    }

    // Handle utility flags
    if cli.cache_stats {
        return show_cache_stats(&output);
    }

    if cli.news {
        return fetch_nixos_news().await;
    }

    if cli.history {
        return show_history(&output);
    }

    if cli.health {
        return run_health_check(&output);
    }

    if cli.clean {
        return run_garbage_collection(&cli, &output);
    }

    // Initialize cache manager
    let cache_manager = if !cli.no_cache && config.cache.enabled {
        match cache::CacheManager::new(config.cache.memory_cache_size) {
            Ok(cm) => {
                if cli.clear_cache {
                    let _ = cm.clear();
                    output.info("Cache cleared");
                }
                Some(std::sync::Arc::new(cm))
            }
            Err(e) => {
                warn!("Failed to initialize cache: {}", e);
                None
            }
        }
    } else {
        None
    };

    // Initialize package manager
    let manager = if let Some(ref cache) = cache_manager {
        PackageManager::with_cache(cache.clone())?
    } else {
        PackageManager::new()?
    };

    // Handle list command
    if cli.list {
        return list_installed(&manager, &output).await;
    }

    // Handle NUR operations
    if cli.nur {
        return handle_nur(&cli, cache_manager.clone(), &output).await;
    }

    // Handle search
    if cli.sync && cli.search {
        return search_packages(&manager, &cli, &output).await;
    }

    // Handle install/remove
    if cli.targets.is_empty() {
        if !cli.has_operation() {
            // Show help if no operation
            debug!("No operation specified");
        }
        return Ok(());
    }

    if cli.sync {
        return install_packages(&manager, &cli, cache_manager.clone(), &output).await;
    }

    if cli.remove {
        return remove_packages(&manager, &cli, &output).await;
    }

    output.success("Operation finished");
    Ok(())
}

/// Initialize logging based on CLI flags
fn init_logging(cli: &Cli) {
    let level = if cli.verbose {
        Level::DEBUG
    } else if cli.quiet {
        Level::ERROR
    } else {
        Level::INFO
    };

    let filter = EnvFilter::new(format!("nixboost={}", level))
        .add_directive("reqwest=warn".parse().unwrap())
        .add_directive("rusqlite=warn".parse().unwrap());

    tracing_subscriber::registry()
        .with(fmt::layer().with_target(false).without_time())
        .with(filter)
        .init();
}

/// Check for updates and prompt user
fn check_and_prompt_update(cli: &Cli) -> Result<()> {
    let pb = progress::spinner("checking for updates...");

    if let Some(info) = check_for_updates(VERSION) {
        pb.finish_and_clear();
        println!(
            "{} a new version is available: {} -> {}",
            style("::").bold().cyan(),
            VERSION,
            info.version
        );

        if !cli.yes {
            if Confirm::with_theme(&ColorfulTheme::default())
                .with_prompt("Update now?")
                .default(true)
                .interact()?
            {
                if let Err(e) = perform_update(info) {
                    eprintln!("{} update failed: {}", style("error:").red().bold(), e);
                } else {
                    println!("   Please restart nixboost.");
                    std::process::exit(0);
                }
            }
        }
    } else {
        pb.finish_and_clear();
    }

    Ok(())
}

/// Handle subcommands
async fn handle_subcommand(cmd: &Commands, output: &Output) -> Result<()> {
    match cmd {
        Commands::Info { package } => {
            let manager = PackageManager::new()?;
            if let Some(pkg) = manager.package_info(package).await? {
                output.print_packages(&[pkg]);
            } else {
                output.error(&format!("Package '{}' not found", package));
            }
        }
        Commands::Generation { action } => {
            use cli::args::GenerationAction;
            match action {
                GenerationAction::List { limit } => {
                    let generations = GenerationManager::list(*limit)?;
                    GenerationManager::print_list(&generations);
                }
                GenerationAction::Diff { from, to } => {
                    let diff = GenerationManager::diff(*from, *to)?;
                    diff.print();
                }
                GenerationAction::Rollback { generation } => {
                    if let Some(gen) = generation {
                        GenerationManager::rollback_to(*gen)?;
                    } else {
                        GenerationManager::rollback()?;
                    }
                    output.success("Rollback completed");
                }
                GenerationAction::Delete { keep } => {
                    let deleted = GenerationManager::delete_old(*keep)?;
                    output.success(&format!("Deleted {} generations", deleted));
                }
            }
        }
        Commands::Cache { action } => {
            use cli::args::CacheAction;
            match action {
                CacheAction::Stats => show_cache_stats(output)?,
                CacheAction::Clear => {
                    if let Ok(cache) = cache::CacheManager::new(100) {
                        cache.clear()?;
                        output.success("Cache cleared");
                    }
                }
                CacheAction::Verify => {
                    output.info("Cache verification not yet implemented");
                }
                CacheAction::Prune => {
                    if let Ok(cache) = cache::CacheManager::new(100) {
                        let pruned = cache.disk.prune()?;
                        output.success(&format!("Pruned {} expired entries", pruned));
                    }
                }
            }
        }
        Commands::Config { action } => {
            use cli::args::ConfigAction;
            match action {
                ConfigAction::Show => {
                    let config = Config::load()?;
                    println!("{}", toml::to_string_pretty(&config)?);
                }
                ConfigAction::Init { force } => {
                    let path = Config::config_path();
                    if path.exists() && !force {
                        output.error("Config already exists. Use --force to overwrite.");
                    } else {
                        let config = Config::default();
                        config.save()?;
                        output.success(&format!("Config saved to {:?}", path));
                    }
                }
                ConfigAction::Edit => {
                    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "nano".to_string());
                    let path = Config::config_path();
                    std::process::Command::new(editor).arg(&path).status()?;
                }
                ConfigAction::Validate => {
                    match Config::load() {
                        Ok(_) => output.success("Config is valid"),
                        Err(e) => output.error(&format!("Config is invalid: {}", e)),
                    }
                }
                ConfigAction::Path => {
                    println!("{}", Config::config_path().display());
                }
            }
        }
        Commands::System { action } => {
            use cli::args::SystemAction;
            match action {
                SystemAction::Health => run_health_check(output)?,
                SystemAction::Gc { keep_generations, dry_run } => {
                    if *dry_run {
                        let preview = GarbageCollector::preview()?;
                        output.info(&format!(
                            "Would delete {} paths, freeing {}",
                            preview.paths.len(),
                            preview.size_human()
                        ));
                    } else {
                        let result = GarbageCollector::run_with_options(*keep_generations, None)?;
                        GarbageCollector::print_result(&result);
                    }
                }
                SystemAction::Verify => {
                    output.info("Verifying Nix store...");
                    let report = HealthChecker::run()?;
                    if report.nix_store_ok {
                        output.success("Nix store is healthy");
                    } else {
                        output.error("Nix store has issues");
                    }
                }
                SystemAction::Optimize => {
                    output.info("Optimizing Nix store...");
                    std::process::Command::new("nix-store")
                        .arg("--optimise")
                        .status()?;
                    output.success("Optimization complete");
                }
                SystemAction::DiskUsage => {
                    std::process::Command::new("nix")
                        .args(["path-info", "--size", "--recursive", "/run/current-system"])
                        .status()?;
                }
            }
        }
        Commands::Completions { shell } => {
            use clap::CommandFactory;
            let mut cmd = Cli::command();
            let name = cmd.get_name().to_string();
            
            match shell {
                cli::args::Shell::Bash => {
                    clap_complete::generate(clap_complete::shells::Bash, &mut cmd, name, &mut std::io::stdout());
                }
                cli::args::Shell::Zsh => {
                    clap_complete::generate(clap_complete::shells::Zsh, &mut cmd, name, &mut std::io::stdout());
                }
                cli::args::Shell::Fish => {
                    clap_complete::generate(clap_complete::shells::Fish, &mut cmd, name, &mut std::io::stdout());
                }
                cli::args::Shell::PowerShell => {
                    clap_complete::generate(clap_complete::shells::PowerShell, &mut cmd, name, &mut std::io::stdout());
                }
                cli::args::Shell::Elvish => {
                    clap_complete::generate(clap_complete::shells::Elvish, &mut cmd, name, &mut std::io::stdout());
                }
            }
        }
    }
    Ok(())
}

/// List installed packages
async fn list_installed(manager: &PackageManager, output: &Output) -> Result<()> {
    let installed = manager.list_installed().await?;
    output.print_installed(&installed);
    Ok(())
}

/// Search packages
async fn search_packages(manager: &PackageManager, cli: &Cli, output: &Output) -> Result<()> {
    let query = cli.targets.join(" ");
    let results = manager.search(&query).await?;

    if results.is_empty() {
        println!("No matches found.");
    } else {
        output.print_packages(&results[..results.len().min(cli.max_results)]);
    }

    Ok(())
}

/// Handle NUR operations
async fn handle_nur(
    cli: &Cli,
    cache: Option<std::sync::Arc<cache::CacheManager>>,
    output: &Output,
) -> Result<()> {
    let targets = &cli.targets;

    if targets.is_empty() {
        output.error("No targets specified for NUR search");
        return Ok(());
    }

    output.info("Searching NUR...");

    let mut nur = if let Some(c) = cache {
        NurClient::with_cache(c)
    } else {
        NurClient::new()
    };

    // Search NUR
    let mut all_results = Vec::new();
    for target in targets {
        match nur.search(target).await {
            Ok(results) => {
                for pkg in results {
                    all_results.push(pkg.into());
                }
            }
            Err(e) => {
                output.warn(&format!("Failed to search NUR: {}", e));
            }
        }
    }

    if all_results.is_empty() {
        output.warn("No matches found in NUR");
    } else {
        output.print_packages(&all_results);
    }

    Ok(())
}

/// Install packages
async fn install_packages(
    manager: &PackageManager,
    cli: &Cli,
    cache: Option<std::sync::Arc<cache::CacheManager>>,
    output: &Output,
) -> Result<()> {
    let targets = &cli.targets;
    output.info(&format!("Installing {} package(s)...", targets.len()));

    if cli.dry_run {
        output.info("Dry run - checking packages...");
        let checks = manager.check_packages(targets).await;
        for (pkg, exists) in checks {
            if exists {
                println!("  {} {}", style("✓").green(), pkg);
            } else {
                println!("  {} {} (not found in nixpkgs)", style("?").yellow(), pkg);
            }
        }
        return Ok(());
    }

    // Try batch install first
    match manager.install(targets).await {
        Ok(()) => {
            output.success(&format!("Installed {} package(s)", targets.len()));
        }
        Err(_) => {
            output.warn("Batch install failed, falling back to individual install...");
            
            let mut nur = if let Some(c) = cache {
                NurClient::with_cache(c)
            } else {
                NurClient::new()
            };

            for target in targets {
                output.info(&format!("Installing {}...", target));
                
                match manager.install(&[target.clone()]).await {
                    Ok(()) => {
                        output.success(&format!("Installed {}", target));
                    }
                    Err(_) => {
                        output.warn(&format!("{} not found in nixpkgs, checking NUR...", target));
                        
                        if let Err(e) = nur.install(target).await {
                            output.error(&format!("Failed to install {}: {}", target, e));
                        }
                    }
                }
            }
        }
    }

    output.success("Operation finished");
    Ok(())
}

/// Remove packages
async fn remove_packages(manager: &PackageManager, cli: &Cli, output: &Output) -> Result<()> {
    let targets = &cli.targets;

    if targets.is_empty() {
        let installed = manager.list_installed().await?;
        output.print_installed(&installed);
        println!("\nUse 'nixboost -R <package>' to remove one.");
        return Ok(());
    }

    println!("{} The following packages will be removed:", style("::").bold().yellow());
    for t in targets {
        println!("   {}", t);
    }

    if !cli.skip_confirm() {
        if !Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("Proceed with removal?")
            .default(true)
            .interact()?
        {
            println!(":: Removal cancelled.");
            return Ok(());
        }
    }

    if cli.dry_run {
        output.info("Dry run - would remove the above packages");
        return Ok(());
    }

    output.info(&format!("Removing {} package(s)...", targets.len()));

    if let Err(e) = manager.remove(targets).await {
        output.error(&format!("Failed to remove packages: {}", e));
    } else {
        output.success("Packages removed");
    }

    Ok(())
}

/// Show cache statistics
fn show_cache_stats(output: &Output) -> Result<()> {
    match cache::CacheManager::new(100) {
        Ok(cache) => {
            let stats = cache.stats();
            println!("{}", style(":: Cache Statistics").bold());
            println!("   Memory entries: {}", stats.memory_entries);
            println!("   Memory hit rate: {:.1}%", stats.hit_rate() * 100.0);
            println!("   Disk entries: {}", stats.disk_entries);
            println!("   Disk size: {}", stats.size_human());
        }
        Err(e) => {
            output.error(&format!("Failed to access cache: {}", e));
        }
    }
    Ok(())
}

/// Show nix generation history
fn show_history(output: &Output) -> Result<()> {
    output.info("Generation history (last 20):");
    let generations = GenerationManager::list(20)?;
    GenerationManager::print_list(&generations);
    Ok(())
}

/// Run health check
fn run_health_check(output: &Output) -> Result<()> {
    output.info("Running system health check...");
    let report = HealthChecker::run()?;
    report.print();
    Ok(())
}

/// Run garbage collection
fn run_garbage_collection(cli: &Cli, output: &Output) -> Result<()> {
    if cli.dry_run {
        let preview = GarbageCollector::preview()?;
        output.info(&format!(
            "Would delete {} paths, freeing {}",
            preview.paths.len(),
            preview.size_human()
        ));
    } else {
        output.info("Collecting garbage...");
        let result = GarbageCollector::run()?;
        GarbageCollector::print_result(&result);
    }
    Ok(())
}
