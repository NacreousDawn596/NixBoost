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

//! Self-updater for NixBoost.

use anyhow::Result;
use console::style;
use serde::Deserialize;
use std::process::Command;
use std::time::Duration;
use tracing::{debug, info};

/// Update information
pub struct UpdateInfo {
    pub version: String,
    pub download_url: Option<String>,
    pub release_notes: Option<String>,
}

#[derive(Deserialize)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
}

#[derive(Deserialize)]
struct GithubRelease {
    tag_name: String,
    body: Option<String>,
    assets: Vec<GithubAsset>,
}

/// Check for updates
pub fn check_for_updates(current_version: &str) -> Option<UpdateInfo> {
    let url = "https://api.github.com/repos/NacreousDawn596/nixboost/releases/latest";

    debug!("Checking for updates from {}", url);

    let response = ureq::get(url)
        .set("User-Agent", "nixboost-updater")
        .timeout(Duration::from_secs(2))
        .call();

    match response {
        Ok(res) => {
            if let Ok(release) = res.into_json::<GithubRelease>() {
                let latest = release.tag_name.trim_start_matches('v');
                
                if is_newer_version(latest, current_version) {
                    debug!("New version available: {} -> {}", current_version, latest);
                    
                    let download_url = release.assets
                        .iter()
                        .find(|a| a.name == "nixboost")
                        .map(|a| a.browser_download_url.clone());

                    return Some(UpdateInfo {
                        version: latest.to_string(),
                        download_url,
                        release_notes: release.body,
                    });
                }
            }
        }
        Err(e) => {
            debug!("Failed to check for updates: {}", e);
        }
    }

    None
}

/// Perform update via nix
pub fn perform_update(_info: UpdateInfo) -> Result<()> {
    info!("Starting automatic update");
    println!("{}", style(":: starting automatic update...").bold().cyan());

    let status = Command::new("nix")
        .args(["profile", "install", "github:NacreousDawn596/nixboost"])
        .status()?;

    if !status.success() {
        anyhow::bail!("nix profile install failed");
    }

    println!("{}", style(":: update completed successfully.").green().bold());
    Ok(())
}

/// Compare version strings
fn is_newer_version(latest: &str, current: &str) -> bool {
    let parse_version = |v: &str| -> Vec<u32> {
        v.split('.')
            .filter_map(|s| s.parse().ok())
            .collect()
    };

    let latest_parts = parse_version(latest);
    let current_parts = parse_version(current);

    for i in 0..latest_parts.len().max(current_parts.len()) {
        let l = latest_parts.get(i).copied().unwrap_or(0);
        let c = current_parts.get(i).copied().unwrap_or(0);
        if l > c {
            return true;
        }
        if l < c {
            return false;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_comparison() {
        assert!(is_newer_version("2.0.0", "1.0.9"));
        assert!(is_newer_version("1.1.0", "1.0.9"));
        assert!(is_newer_version("1.0.10", "1.0.9"));
        assert!(!is_newer_version("1.0.9", "1.0.9"));
        assert!(!is_newer_version("1.0.8", "1.0.9"));
    }
}
