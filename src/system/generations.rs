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

//! Generation management for NixBoost.

use crate::core::error::{Result, SystemError};
use crate::core::types::Generation;
use console::style;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, info};

/// Generation manager
pub struct GenerationManager;

impl GenerationManager {
    /// List all generations
    pub fn list(limit: usize) -> Result<Vec<Generation>> {
        debug!("Listing generations (limit: {})", limit);

        let output = Command::new("nix-env")
            .args(["--list-generations"])
            .output()?;

        if !output.status.success() {
            return Err(SystemError::NixCommandFailed {
                command: "nix-env --list-generations".to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            }.into());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut generations: Vec<Generation> = stdout
            .lines()
            .filter_map(|line| Self::parse_generation_line(line))
            .collect();

        generations.reverse();
        generations.truncate(limit);

        Ok(generations)
    }

    /// Parse a generation line from nix-env output
    fn parse_generation_line(line: &str) -> Option<Generation> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            return None;
        }

        let number: u64 = parts[0].parse().ok()?;
        let is_current = line.contains("(current)");

        // Try to extract timestamp if available
        let created_at = if parts.len() >= 3 {
            // Format: "1   2024-01-01 12:00:00   (current)"
            Self::parse_timestamp(&parts[1..3].join(" ")).unwrap_or(SystemTime::UNIX_EPOCH)
        } else {
            SystemTime::UNIX_EPOCH
        };

        Some(Generation {
            number,
            created_at,
            is_current,
            path: format!("/nix/var/nix/profiles/default-{}-link", number),
        })
    }

    /// Parse timestamp from string
    fn parse_timestamp(s: &str) -> Option<SystemTime> {
        // Try common formats
        // This is a simplified parser - in production you'd use chrono
        None // For simplicity, return None for now
    }

    /// Get the current generation
    pub fn current() -> Result<Option<Generation>> {
        let generations = Self::list(1)?;
        Ok(generations.into_iter().find(|g| g.is_current))
    }

    /// Rollback to previous generation
    pub fn rollback() -> Result<()> {
        info!("Rolling back to previous generation");

        let status = Command::new("nix-env")
            .args(["--rollback"])
            .status()?;

        if !status.success() {
            return Err(SystemError::RollbackFailed("nix-env --rollback failed".to_string()).into());
        }

        Ok(())
    }

    /// Rollback to a specific generation
    pub fn rollback_to(generation: u64) -> Result<()> {
        info!("Rolling back to generation {}", generation);

        // First check if generation exists
        let generations = Self::list(100)?;
        if !generations.iter().any(|g| g.number == generation) {
            return Err(SystemError::GenerationNotFound { generation }.into());
        }

        let status = Command::new("nix-env")
            .args(["--switch-generation", &generation.to_string()])
            .status()?;

        if !status.success() {
            return Err(SystemError::RollbackFailed(
                format!("Failed to switch to generation {}", generation)
            ).into());
        }

        Ok(())
    }

    /// Delete specific generations
    pub fn delete(generations: &[u64]) -> Result<()> {
        if generations.is_empty() {
            return Ok(());
        }

        info!("Deleting {} generation(s)", generations.len());

        for gen in generations {
            debug!("Deleting generation {}", gen);
            let status = Command::new("nix-env")
                .args(["--delete-generations", &gen.to_string()])
                .status()?;

            if !status.success() {
                return Err(SystemError::NixCommandFailed {
                    command: format!("nix-env --delete-generations {}", gen),
                    stderr: "Command failed".to_string(),
                }.into());
            }
        }

        Ok(())
    }

    /// Delete generations keeping the last N
    pub fn delete_old(keep: usize) -> Result<usize> {
        let generations = Self::list(1000)?;
        
        if generations.len() <= keep {
            return Ok(0);
        }

        let to_delete: Vec<u64> = generations
            .iter()
            .skip(keep)
            .filter(|g| !g.is_current)
            .map(|g| g.number)
            .collect();

        let count = to_delete.len();
        if count > 0 {
            Self::delete(&to_delete)?;
        }

        Ok(count)
    }

    /// Diff two generations
    pub fn diff(from: u64, to: u64) -> Result<GenerationDiff> {
        debug!("Diffing generations {} -> {}", from, to);

        let output = Command::new("nix-store")
            .args([
                "--diff-closures",
                &format!("/nix/var/nix/profiles/default-{}-link", from),
                &format!("/nix/var/nix/profiles/default-{}-link", to),
            ])
            .output()?;

        if !output.status.success() {
            return Err(SystemError::NixCommandFailed {
                command: "nix-store --diff-closures".to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            }.into());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let (added, removed, changed) = Self::parse_diff(&stdout);

        Ok(GenerationDiff {
            from,
            to,
            added,
            removed,
            changed,
        })
    }

    /// Parse diff output
    fn parse_diff(output: &str) -> (Vec<String>, Vec<String>, Vec<String>) {
        let mut added = Vec::new();
        let mut removed = Vec::new();
        let mut changed = Vec::new();

        for line in output.lines() {
            let line = line.trim();
            if line.starts_with('+') {
                added.push(line[1..].trim().to_string());
            } else if line.starts_with('-') {
                removed.push(line[1..].trim().to_string());
            } else if line.contains("→") || line.contains("->") {
                changed.push(line.to_string());
            }
        }

        (added, removed, changed)
    }

    /// Print generations table
    pub fn print_list(generations: &[Generation]) {
        use comfy_table::{Table, presets::UTF8_FULL};

        let mut table = Table::new();
        table.load_preset(UTF8_FULL);
        table.set_header(vec!["Generation", "Status", "Path"]);

        for gen in generations {
            let status = if gen.is_current {
                style("(current)").green().to_string()
            } else {
                String::new()
            };

            table.add_row(vec![
                gen.number.to_string(),
                status,
                gen.path.clone(),
            ]);
        }

        println!("{}", table);
    }
}

/// Generation diff result
#[derive(Debug)]
pub struct GenerationDiff {
    pub from: u64,
    pub to: u64,
    pub added: Vec<String>,
    pub removed: Vec<String>,
    pub changed: Vec<String>,
}

impl GenerationDiff {
    pub fn print(&self) {
        println!("Generation {} → {}:", self.from, self.to);
        println!();

        if !self.added.is_empty() {
            println!("{}", style("Added:").green().bold());
            for pkg in &self.added {
                println!("  + {}", style(pkg).green());
            }
        }

        if !self.removed.is_empty() {
            println!("{}", style("Removed:").red().bold());
            for pkg in &self.removed {
                println!("  - {}", style(pkg).red());
            }
        }

        if !self.changed.is_empty() {
            println!("{}", style("Changed:").yellow().bold());
            for pkg in &self.changed {
                println!("  ~ {}", style(pkg).yellow());
            }
        }

        if self.added.is_empty() && self.removed.is_empty() && self.changed.is_empty() {
            println!("{}", style("No differences found").dim());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_generation_line() {
        let line = "   1   2024-01-01 12:00:00   ";
        let gen = GenerationManager::parse_generation_line(line);
        assert!(gen.is_some());
        assert_eq!(gen.unwrap().number, 1);

        let current = "   5   2024-01-15 12:00:00   (current)";
        let gen = GenerationManager::parse_generation_line(current);
        assert!(gen.is_some());
        let gen = gen.unwrap();
        assert_eq!(gen.number, 5);
        assert!(gen.is_current);
    }

    #[test]
    fn test_parse_diff() {
        let output = "+package-1.0\n-oldpackage-0.9\nfoo: 1.0 → 2.0";
        let (added, removed, changed) = GenerationManager::parse_diff(output);
        
        assert_eq!(added.len(), 1);
        assert_eq!(removed.len(), 1);
        assert_eq!(changed.len(), 1);
    }
}
