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

//! System health checks for NixBoost.

use crate::core::error::{Result, SystemError};
use console::style;
use std::process::Command;
use tracing::{debug, info, warn};

/// Health check results
#[derive(Debug, Clone)]
pub struct HealthReport {
    pub systemd_ok: bool,
    pub systemd_failed: Vec<String>,
    pub nix_store_ok: bool,
    pub nix_store_issues: Vec<String>,
    pub disk_space_ok: bool,
    pub disk_space_warning: Option<String>,
    pub nix_daemon_ok: bool,
}

impl HealthReport {
    pub fn is_healthy(&self) -> bool {
        self.systemd_ok && self.nix_store_ok && self.nix_daemon_ok
    }

    pub fn print(&self) {
        if self.systemd_ok {
            println!("{}", style("✓ All systemd services are running fine").green());
        } else {
            println!("{}", style("✗ Some systemd services have failed:").red());
            for svc in &self.systemd_failed {
                println!("  - {}", svc);
            }
        }

        if self.nix_store_ok {
            println!("{}", style("✓ Nix store is healthy").green());
        } else {
            println!("{}", style("✗ Nix store issues detected:").red());
            for issue in &self.nix_store_issues {
                println!("  - {}", issue);
            }
        }

        if self.nix_daemon_ok {
            println!("{}", style("✓ Nix daemon is running").green());
        } else {
            println!("{}", style("⚠ Nix daemon not detected (multi-user mode may not work)").yellow());
        }

        if let Some(ref warning) = self.disk_space_warning {
            println!("{}", style(format!("⚠ {}", warning)).yellow());
        }
    }
}

/// System health checker
pub struct HealthChecker;

impl HealthChecker {
    /// Run all health checks
    pub fn run() -> Result<HealthReport> {
        info!("Running system health check");

        let systemd_result = Self::check_systemd();
        let nix_store_result = Self::check_nix_store();
        let nix_daemon_ok = Self::check_nix_daemon();
        let disk_check = Self::check_disk_space();

        Ok(HealthReport {
            systemd_ok: systemd_result.0,
            systemd_failed: systemd_result.1,
            nix_store_ok: nix_store_result.0,
            nix_store_issues: nix_store_result.1,
            disk_space_ok: disk_check.0,
            disk_space_warning: disk_check.1,
            nix_daemon_ok,
        })
    }

    /// Check systemd services
    fn check_systemd() -> (bool, Vec<String>) {
        debug!("Checking systemd services");

        let output = Command::new("systemctl")
            .args(["--failed", "--no-pager", "--plain"])
            .output();

        match output {
            Ok(o) if o.status.success() => {
                let stdout = String::from_utf8_lossy(&o.stdout);
                let failed: Vec<String> = stdout
                    .lines()
                    .filter(|l| l.contains("failed"))
                    .map(|l| l.split_whitespace().next().unwrap_or("").to_string())
                    .filter(|s| !s.is_empty())
                    .collect();

                (failed.is_empty(), failed)
            }
            Ok(_) => (true, vec![]),
            Err(_) => {
                debug!("systemctl not available");
                (true, vec![])
            }
        }
    }

    /// Check Nix store integrity
    fn check_nix_store() -> (bool, Vec<String>) {
        debug!("Checking Nix store integrity");

        let output = Command::new("nix-store")
            .arg("--verify")
            .arg("--check-contents")
            .output();

        match output {
            Ok(o) => {
                let stderr = String::from_utf8_lossy(&o.stderr);
                let issues: Vec<String> = stderr
                    .lines()
                    .filter(|l| l.contains("error") || l.contains("warning"))
                    .map(|l| l.to_string())
                    .collect();

                (o.status.success() && issues.is_empty(), issues)
            }
            Err(e) => (false, vec![format!("Failed to run nix-store: {}", e)]),
        }
    }

    /// Check if Nix daemon is running
    fn check_nix_daemon() -> bool {
        debug!("Checking Nix daemon");

        let output = Command::new("systemctl")
            .args(["is-active", "nix-daemon"])
            .output();

        match output {
            Ok(o) => {
                let stdout = String::from_utf8_lossy(&o.stdout);
                stdout.trim() == "active"
            }
            Err(_) => {
                // Try socket check
                std::path::Path::new("/nix/var/nix/daemon-socket/socket").exists()
            }
        }
    }

    /// Check disk space
    fn check_disk_space() -> (bool, Option<String>) {
        debug!("Checking disk space");

        let output = Command::new("df")
            .args(["-h", "/nix/store"])
            .output();

        match output {
            Ok(o) if o.status.success() => {
                let stdout = String::from_utf8_lossy(&o.stdout);
                for line in stdout.lines().skip(1) {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 5 {
                        let usage = parts[4].trim_end_matches('%');
                        if let Ok(pct) = usage.parse::<u32>() {
                            if pct > 90 {
                                return (false, Some(format!("Disk usage is at {}%", pct)));
                            } else if pct > 80 {
                                return (true, Some(format!("Disk usage is at {}%", pct)));
                            }
                        }
                    }
                }
                (true, None)
            }
            _ => (true, None),
        }
    }

    /// Quick check - just essential services
    pub fn quick_check() -> bool {
        let nix_ok = Command::new("nix")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        let store_ok = std::path::Path::new("/nix/store").exists();

        nix_ok && store_ok
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quick_check() {
        // This test requires Nix to be installed
        if std::process::Command::new("nix").arg("--version").output().is_ok() {
            let ok = HealthChecker::quick_check();
            // Should pass on a working NixOS/Nix system
            assert!(ok);
        }
    }
}
