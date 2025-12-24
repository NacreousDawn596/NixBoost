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

//! Garbage collection for NixBoost.

use crate::core::error::{Result, SystemError};
use crate::core::types::GCPreview;
use console::style;
use std::process::Command;
use tracing::{debug, info, warn};

/// Smart garbage collector
pub struct GarbageCollector;

impl GarbageCollector {
    /// Run garbage collection
    pub fn run() -> Result<GCResult> {
        info!("Running garbage collection");

        let output = Command::new("nix-collect-garbage")
            .arg("-d")
            .output()?;

        if !output.status.success() {
            return Err(SystemError::GarbageCollectionFailed(
                String::from_utf8_lossy(&output.stderr).to_string()
            ).into());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let bytes_freed = Self::parse_freed_space(&stdout);

        Ok(GCResult {
            success: true,
            bytes_freed,
            message: stdout.to_string(),
        })
    }

    /// Run garbage collection with options
    pub fn run_with_options(keep_generations: usize, delete_older_than: Option<&str>) -> Result<GCResult> {
        info!("Running garbage collection (keep {} generations)", keep_generations);

        let mut args = vec!["-d"];
        
        // Note: nix-collect-garbage doesn't directly support keep_generations
        // We need to use nix-env to delete old generations first
        if keep_generations > 0 {
            Self::delete_old_generations(keep_generations)?;
        }

        // Add older-than option if specified
        let older_than_arg: String;
        if let Some(older_than) = delete_older_than {
            older_than_arg = format!("--delete-older-than {}", older_than);
            args.push("--delete-older-than");
            args.push(older_than);
        }

        let output = Command::new("nix-collect-garbage")
            .args(&args)
            .output()?;

        if !output.status.success() {
            return Err(SystemError::GarbageCollectionFailed(
                String::from_utf8_lossy(&output.stderr).to_string()
            ).into());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let bytes_freed = Self::parse_freed_space(&stdout);

        Ok(GCResult {
            success: true,
            bytes_freed,
            message: stdout.to_string(),
        })
    }

    /// Preview what would be garbage collected
    pub fn preview() -> Result<GCPreview> {
        info!("Previewing garbage collection");

        let output = Command::new("nix-store")
            .args(["--gc", "--print-dead"])
            .output()?;

        if !output.status.success() {
            return Err(SystemError::GarbageCollectionFailed(
                String::from_utf8_lossy(&output.stderr).to_string()
            ).into());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let paths: Vec<String> = stdout
            .lines()
            .filter(|l| !l.is_empty())
            .map(|l| l.to_string())
            .collect();

        let size_bytes = Self::calculate_size(&paths);

        Ok(GCPreview {
            paths,
            size_bytes,
            affected_generations: vec![],
        })
    }

    /// Delete old generations (keeping the last N)
    fn delete_old_generations(keep: usize) -> Result<()> {
        debug!("Deleting old generations, keeping {}", keep);

        // Get list of generations
        let output = Command::new("nix-env")
            .args(["--list-generations"])
            .output()?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let generations: Vec<u64> = stdout
                .lines()
                .filter_map(|l| l.split_whitespace().next())
                .filter_map(|s| s.parse().ok())
                .collect();

            if generations.len() > keep {
                let to_delete = generations.len() - keep;
                let delete_gens: Vec<_> = generations.iter().take(to_delete).collect();

                for gen in delete_gens {
                    debug!("Deleting generation {}", gen);
                    let _ = Command::new("nix-env")
                        .args(["--delete-generations", &gen.to_string()])
                        .output();
                }
            }
        }

        Ok(())
    }

    /// Parse freed space from nix-collect-garbage output
    fn parse_freed_space(output: &str) -> u64 {
        // Look for patterns like "1234 bytes" or "1.2 MiB"
        for line in output.lines() {
            if line.contains("freed") {
                // Try to extract bytes freed
                if let Some(idx) = line.find("freed") {
                    let before = &line[..idx];
                    let parts: Vec<&str> = before.split_whitespace().collect();
                    if let Some(last) = parts.last() {
                        if let Ok(bytes) = last.parse::<u64>() {
                            return bytes;
                        }
                        // Try parsing with suffix
                        return Self::parse_size_string(last);
                    }
                }
            }
        }
        0
    }

    /// Parse size string like "1.5 GiB" to bytes
    fn parse_size_string(s: &str) -> u64 {
        let s = s.trim();
        
        if let Some(mib) = s.strip_suffix("MiB") {
            if let Ok(val) = mib.trim().parse::<f64>() {
                return (val * 1024.0 * 1024.0) as u64;
            }
        }
        if let Some(gib) = s.strip_suffix("GiB") {
            if let Ok(val) = gib.trim().parse::<f64>() {
                return (val * 1024.0 * 1024.0 * 1024.0) as u64;
            }
        }
        if let Some(kib) = s.strip_suffix("KiB") {
            if let Ok(val) = kib.trim().parse::<f64>() {
                return (val * 1024.0) as u64;
            }
        }
        
        s.parse().unwrap_or(0)
    }

    /// Calculate total size of paths
    fn calculate_size(paths: &[String]) -> u64 {
        let mut total: u64 = 0;
        for path in paths {
            if let Ok(meta) = std::fs::metadata(path) {
                total += meta.len();
            }
        }
        total
    }

    /// Print GC result
    pub fn print_result(result: &GCResult) {
        if result.success {
            let size = format_bytes(result.bytes_freed);
            println!("{}", style(format!("✓ Garbage collection completed, freed {}", size)).green());
        } else {
            println!("{}", style("✗ Garbage collection failed").red());
        }
    }
}

/// Garbage collection result
#[derive(Debug)]
pub struct GCResult {
    pub success: bool,
    pub bytes_freed: u64,
    pub message: String,
}

impl GCResult {
    pub fn freed_human(&self) -> String {
        format_bytes(self.bytes_freed)
    }
}

fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KiB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MiB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GiB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_size_string() {
        assert_eq!(GarbageCollector::parse_size_string("1024"), 1024);
        assert_eq!(GarbageCollector::parse_size_string("1.0MiB"), 1048576);
        assert_eq!(GarbageCollector::parse_size_string("1.5GiB"), 1610612736);
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1536), "1.5 KiB");
        assert_eq!(format_bytes(1572864), "1.5 MiB");
    }
}
