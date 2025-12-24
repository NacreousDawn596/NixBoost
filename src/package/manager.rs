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

//! Package manager - core Nix operations with caching and parallel execution.

use crate::core::error::{NixBoostError, PackageError, Result, SystemError};
use crate::core::types::{Package, PackageSource};
use crate::cache::CacheManager;
use crate::cache::invalidation::{CacheKey, TTL};
use tokio::process::Command;
use serde_json::Value;
use std::sync::Arc;
use tracing::{debug, info, warn, error};
use futures::future::join_all;

/// Package manager for Nix operations
pub struct PackageManager {
    /// System architecture
    arch: String,
    /// Cache manager (optional)
    cache: Option<Arc<CacheManager>>,
}

impl PackageManager {
    /// Create a new package manager
    pub fn new() -> Result<Self> {
        let arch = detect_system_arch()?;
        info!("PackageManager initialized for {}", arch);
        
        Ok(Self { 
            arch,
            cache: None,
        })
    }

    /// Create with cache manager
    pub fn with_cache(cache: Arc<CacheManager>) -> Result<Self> {
        let arch = detect_system_arch()?;
        Ok(Self {
            arch,
            cache: Some(cache),
        })
    }

    /// Get the system architecture
    pub fn arch(&self) -> &str {
        &self.arch
    }

    /// Search nixpkgs for packages
    pub async fn search(&self, query: &str) -> Result<Vec<Package>> {
        // Check cache first
        let cache_key = CacheKey::search(query);
        if let Some(ref cache) = self.cache {
            if let Some(cached) = cache.get::<Vec<Package>>(&cache_key) {
                debug!("Search cache hit for '{}'", query);
                return Ok(cached);
            }
        }

        debug!("Searching nixpkgs for '{}'", query);
        let legacy_prefix = format!("legacyPackages.{}.", self.arch);

        let output = Command::new("nix")
            .args(["search", "--json", "nixpkgs", query])
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SystemError::NixCommandFailed {
                command: "nix search".to_string(),
                stderr: stderr.to_string(),
            }.into());
        }

        let json: Value = serde_json::from_slice(&output.stdout)
            .map_err(|e| NixBoostError::Serialization(e.to_string()))?;

        let mut results = Vec::new();

        if let Some(obj) = json.as_object() {
            for (key, val) in obj {
                let name = key.strip_prefix(&legacy_prefix)
                    .or_else(|| key.strip_prefix("legacyPackages.x86_64-linux."))
                    .unwrap_or(key)
                    .to_string();

                let version = val["version"].as_str().unwrap_or("unknown").to_string();
                let description = val["description"].as_str().unwrap_or("").to_string();

                results.push(Package::from_nixpkgs(name, version, description));
            }
        }

        // Cache results
        if let Some(ref cache) = self.cache {
            if let Err(e) = cache.set(&cache_key, &results, TTL::SEARCH) {
                warn!("Failed to cache search results: {}", e);
            }
        }

        info!("Found {} packages for '{}'", results.len(), query);
        Ok(results)
    }

    /// Install packages (batch operation)
    pub async fn install(&self, packages: &[String]) -> Result<()> {
        if packages.is_empty() {
            return Ok(());
        }

        info!("Installing {} package(s)", packages.len());

        let install_args: Vec<String> = packages.iter()
            .map(|p| format!("nixpkgs#{}", p))
            .collect();

        let mut args = vec!["profile", "install"];
        let refs: Vec<&str> = install_args.iter().map(|s| s.as_str()).collect();
        args.extend(refs);

        let status = Command::new("nix")
            .args(&args)
            .status()
            .await?;

        if !status.success() {
            return Err(PackageError::InstallFailed {
                name: packages.join(", "),
                reason: "nix profile install failed".to_string(),
            }.into());
        }

        // Invalidate installed packages cache
        if let Some(ref cache) = self.cache {
            let _ = cache.disk.delete(&CacheKey::installed());
        }

        Ok(())
    }

    /// Install a single package with detailed error reporting
    pub async fn install_single(&self, package: &str) -> Result<()> {
        debug!("Installing package: {}", package);

        let status = Command::new("nix")
            .args(["profile", "install", &format!("nixpkgs#{}", package)])
            .status()
            .await?;

        if !status.success() {
            return Err(PackageError::InstallFailed {
                name: package.to_string(),
                reason: "nix profile install failed".to_string(),
            }.into());
        }

        Ok(())
    }

    /// Install packages in parallel (for independent packages)
    pub async fn install_parallel(&self, packages: &[String], max_concurrent: usize) -> Vec<Result<()>> {
        info!("Installing {} packages in parallel (max {})", packages.len(), max_concurrent);

        let chunks: Vec<_> = packages.chunks(max_concurrent).collect();
        let mut all_results = Vec::new();

        for chunk in chunks {
            let futures: Vec<_> = chunk.iter()
                .map(|pkg| self.install_single(pkg))
                .collect();
            
            let results = join_all(futures).await;
            all_results.extend(results);
        }

        all_results
    }

    /// Remove packages (batch operation)
    pub async fn remove(&self, packages: &[String]) -> Result<()> {
        if packages.is_empty() {
            return Ok(());
        }

        info!("Removing {} package(s)", packages.len());

        let mut args = vec!["profile", "remove"];
        let refs: Vec<&str> = packages.iter().map(|s| s.as_str()).collect();
        args.extend(refs);

        let status = Command::new("nix")
            .args(&args)
            .status()
            .await?;

        if !status.success() {
            return Err(PackageError::RemoveFailed {
                name: packages.join(", "),
                reason: "nix profile remove failed".to_string(),
            }.into());
        }

        // Invalidate installed packages cache
        if let Some(ref cache) = self.cache {
            let _ = cache.disk.delete(&CacheKey::installed());
        }

        Ok(())
    }

    /// List installed packages
    pub async fn list_installed(&self) -> Result<Vec<String>> {
        // Check cache first
        let cache_key = CacheKey::installed();
        if let Some(ref cache) = self.cache {
            if let Some(cached) = cache.get::<Vec<String>>(&cache_key) {
                debug!("Installed packages cache hit");
                return Ok(cached);
            }
        }

        let output = Command::new("nix")
            .args(["profile", "list", "--json"])
            .output()
            .await?;

        if !output.status.success() {
            return Err(SystemError::NixCommandFailed {
                command: "nix profile list".to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            }.into());
        }

        let json: Value = serde_json::from_slice(&output.stdout)
            .map_err(|e| NixBoostError::Serialization(e.to_string()))?;

        let mut installed = Vec::new();

        if let Some(elements) = json["elements"].as_object() {
            for (name, _) in elements {
                installed.push(name.clone());
            }
        }

        installed.sort();

        // Cache results
        if let Some(ref cache) = self.cache {
            if let Err(e) = cache.set(&cache_key, &installed, TTL::INSTALLED) {
                warn!("Failed to cache installed packages: {}", e);
            }
        }

        Ok(installed)
    }

    /// Dry run install - check if packages exist without installing
    pub async fn check_packages(&self, packages: &[String]) -> Vec<(String, bool)> {
        let futures: Vec<_> = packages.iter()
            .map(|pkg| async move {
                let exists = self.package_exists(pkg).await;
                (pkg.clone(), exists)
            })
            .collect();
        
        join_all(futures).await
    }

    /// Check if a package exists in nixpkgs
    pub async fn package_exists(&self, package: &str) -> bool {
        let output = Command::new("nix")
            .args(["eval", "--raw", &format!("nixpkgs#{}.meta.name", package)])
            .output()
            .await;

        match output {
            Ok(o) => o.status.success(),
            Err(_) => false,
        }
    }

    /// Get package info
    pub async fn package_info(&self, package: &str) -> Result<Option<Package>> {
        debug!("Getting info for package: {}", package);

        let output = Command::new("nix")
            .args(["eval", "--json", &format!("nixpkgs#{}", package)])
            .output()
            .await?;

        if !output.status.success() {
            return Ok(None);
        }

        let json: Value = serde_json::from_slice(&output.stdout)
            .map_err(|e| NixBoostError::Serialization(e.to_string()))?;

        let name = json["pname"].as_str().unwrap_or(package);
        let version = json["version"].as_str().unwrap_or("unknown");
        let description = json["meta"]["description"].as_str().unwrap_or("");

        let mut pkg = Package::from_nixpkgs(name, version, description);
        
        if let Some(homepage) = json["meta"]["homepage"].as_str() {
            pkg.homepage = Some(homepage.to_string());
        }
        if let Some(license) = json["meta"]["license"]["spdxId"].as_str() {
            pkg.license = Some(license.to_string());
        }

        Ok(Some(pkg))
    }
}

/// Detect the system architecture using Nix
fn detect_system_arch() -> Result<String> {
    let output = std::process::Command::new("nix")
        .args(["eval", "--raw", "--impure", "--expr", "builtins.currentSystem"])
        .output()?;

    if !output.status.success() {
        return Err(SystemError::ArchDetectionFailed.into());
    }

    let arch = String::from_utf8(output.stdout)
        .map_err(|_| SystemError::ArchDetectionFailed)?
        .trim()
        .to_string();

    Ok(arch)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_arch() {
        // This test requires Nix to be installed
        if std::process::Command::new("nix").arg("--version").output().is_ok() {
            let arch = detect_system_arch();
            assert!(arch.is_ok());
            let arch = arch.unwrap();
            assert!(arch.contains("-linux") || arch.contains("-darwin"));
        }
    }
}
