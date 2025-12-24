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

//! NUR (Nix User Repository) client for NixBoost.

use crate::core::config::Config;
use crate::core::error::{NixBoostError, NurError, Result};
use crate::core::types::{Package, PackageSource};
use crate::cache::CacheManager;
use crate::cache::invalidation::{CacheKey, TTL};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, info, warn};

const NUR_INDEX_URL: &str = "https://raw.githubusercontent.com/nix-community/nur-search/master/data/packages.json";

/// NUR package information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NurPackage {
    /// Full attribute path (e.g., "repos.mic92.hello")
    pub attr_path: String,
    /// Package name
    pub name: String,
    /// Version
    pub version: String,
    /// Description
    pub description: String,
    /// Repository owner
    pub repo: String,
    /// Homepage URL
    pub homepage: Option<String>,
    /// License
    pub license: Option<String>,
}

impl From<NurPackage> for Package {
    fn from(nur: NurPackage) -> Self {
        Package {
            name: nur.name,
            version: nur.version,
            description: nur.description,
            source: PackageSource::Nur { repo: nur.repo },
            attr_path: Some(nur.attr_path),
            homepage: nur.homepage,
            license: nur.license,
            maintainers: Vec::new(),
            platforms: Vec::new(),
        }
    }
}

/// NUR client for searching and installing NUR packages
pub struct NurClient {
    /// HTTP client
    http: reqwest::Client,
    /// Cache manager (optional)
    cache: Option<Arc<CacheManager>>,
    /// Index cache (in-memory for current session)
    index: Option<HashMap<String, Value>>,
}

impl NurClient {
    /// Create a new NUR client
    pub fn new() -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent(format!("nixboost/{}", env!("CARGO_PKG_VERSION")))
            .build()
            .unwrap_or_default();

        Self {
            http,
            cache: None,
            index: None,
        }
    }

    /// Create with cache manager
    pub fn with_cache(cache: Arc<CacheManager>) -> Self {
        let mut client = Self::new();
        client.cache = Some(cache);
        client
    }

    /// Load or update the NUR index
    pub async fn load_index(&mut self) -> Result<()> {
        // Try cache first
        if let Some(ref cache) = self.cache {
            if let Some(cached) = cache.get::<HashMap<String, Value>>(&CacheKey::nur_index()) {
                debug!("NUR index loaded from cache");
                self.index = Some(cached);
                return Ok(());
            }
        }

        // Try local file cache
        let cache_file = Config::cache_dir().join("nur-packages.json");
        let mut download_needed = true;

        if cache_file.exists() {
            if let Ok(metadata) = std::fs::metadata(&cache_file) {
                if let Ok(modified) = metadata.modified() {
                    if let Ok(elapsed) = modified.elapsed() {
                        if elapsed.as_secs() < 86400 {
                            download_needed = false;
                        }
                    }
                }
            }
        }

        if download_needed {
            info!("Updating NUR package index...");
            self.download_index().await?;
        }

        // Load from file
        let content = std::fs::read_to_string(&cache_file)
            .map_err(|e| NurError::IndexNotAvailable)?;

        let json: HashMap<String, Value> = serde_json::from_str(&content)
            .map_err(|e| NixBoostError::Serialization(e.to_string()))?;

        // Cache in memory cache manager
        if let Some(ref cache) = self.cache {
            let _ = cache.set(&CacheKey::nur_index(), &json, TTL::NUR_INDEX);
        }

        self.index = Some(json);
        Ok(())
    }

    /// Download the NUR index
    async fn download_index(&self) -> Result<()> {
        let response = self.http
            .get(NUR_INDEX_URL)
            .send()
            .await
            .map_err(|e| NurError::IndexUpdateFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(NurError::IndexUpdateFailed(
                format!("HTTP {}", response.status())
            ).into());
        }

        let bytes = response.bytes().await
            .map_err(|e| NurError::IndexUpdateFailed(e.to_string()))?;

        let cache_dir = Config::cache_dir();
        std::fs::create_dir_all(&cache_dir)?;
        std::fs::write(cache_dir.join("nur-packages.json"), bytes)?;

        info!("NUR index updated successfully");
        Ok(())
    }

    /// Search NUR packages
    pub async fn search(&mut self, query: &str) -> Result<Vec<NurPackage>> {
        if self.index.is_none() {
            self.load_index().await?;
        }

        let index = self.index.as_ref()
            .ok_or(NurError::IndexNotAvailable)?;

        let query_lower = query.to_lowercase();
        let mut results = Vec::new();

        for (attr_path, val) in index {
            let description = val["meta"]["description"].as_str().unwrap_or("");
            let name_part = attr_path.split('.').last().unwrap_or(attr_path);

            if attr_path.to_lowercase().contains(&query_lower) ||
               description.to_lowercase().contains(&query_lower) {
                // Parse repo from attr_path (e.g., "repos.username.pkgname")
                let repo = attr_path.strip_prefix("repos.")
                    .and_then(|s| s.split('.').next())
                    .unwrap_or("unknown")
                    .to_string();

                results.push(NurPackage {
                    attr_path: attr_path.clone(),
                    name: name_part.to_string(),
                    version: val["version"].as_str().unwrap_or("").to_string(),
                    description: description.to_string(),
                    repo,
                    homepage: val["meta"]["homepage"].as_str().map(|s| s.to_string()),
                    license: val["meta"]["license"]["spdxId"].as_str().map(|s| s.to_string()),
                });
            }
        }

        debug!("Found {} NUR packages for '{}'", results.len(), query);
        Ok(results)
    }

    /// Resolve a package name to its full NUR attribute path
    pub async fn resolve(&mut self, name: &str) -> Result<Option<String>> {
        if self.index.is_none() {
            self.load_index().await?;
        }

        let index = self.index.as_ref()
            .ok_or(NurError::IndexNotAvailable)?;

        let query = name.to_lowercase();

        // Exact match at end of path
        for (key, _) in index {
            if key.to_lowercase().ends_with(&format!(".{}", query)) || 
               key.to_lowercase() == query {
                return Ok(Some(key.clone()));
            }
        }

        // Partial match
        for (key, _) in index {
            if key.to_lowercase().contains(&query) {
                return Ok(Some(key.clone()));
            }
        }

        Ok(None)
    }

    /// Install a NUR package
    pub async fn install(&mut self, package: &str) -> Result<()> {
        let mut attr_path = package.strip_prefix("nur.")
            .unwrap_or(package)
            .to_string();

        // Resolve if not a full path
        if !attr_path.contains("repos.") {
            info!("Resolving NUR package: {}", package);
            if let Some(resolved) = self.resolve(&attr_path).await? {
                debug!("Resolved {} to {}", package, resolved);
                attr_path = resolved.strip_prefix("nur.")
                    .unwrap_or(&resolved)
                    .to_string();
            } else {
                return Err(NurError::PackageNotFound { name: package.to_string() }.into());
            }
        }

        info!("Installing NUR package: {}", attr_path);

        let status = std::process::Command::new("nix")
            .args(["profile", "install", &format!("github:nix-community/NUR#{}", attr_path)])
            .status()?;

        if !status.success() {
            return Err(NurError::InvalidAttributePath { path: attr_path }.into());
        }

        Ok(())
    }

    /// Get package count in index
    pub fn package_count(&self) -> usize {
        self.index.as_ref().map(|i| i.len()).unwrap_or(0)
    }
}

impl Default for NurClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nur_package_to_package() {
        let nur = NurPackage {
            attr_path: "repos.mic92.hello".to_string(),
            name: "hello".to_string(),
            version: "1.0.0".to_string(),
            description: "Hello world".to_string(),
            repo: "mic92".to_string(),
            homepage: None,
            license: None,
        };

        let pkg: Package = nur.into();
        assert_eq!(pkg.name, "hello");
        assert!(matches!(pkg.source, PackageSource::Nur { ref repo } if repo == "mic92"));
    }
}
