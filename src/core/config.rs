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

//! Configuration system for NixBoost - TOML-based with XDG compliance.

use crate::core::error::{NixBoostError, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::OnceLock;
use tracing::{debug, info, warn};

/// Global configuration instance
static CONFIG: OnceLock<Config> = OnceLock::new();

/// Main configuration structure for NixBoost
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// General settings
    pub general: GeneralConfig,
    /// Search-related settings
    pub search: SearchConfig,
    /// Cache settings
    pub cache: CacheConfig,
    /// Network settings
    pub network: NetworkConfig,
    /// UI preferences
    pub ui: UiConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            search: SearchConfig::default(),
            cache: CacheConfig::default(),
            network: NetworkConfig::default(),
            ui: UiConfig::default(),
        }
    }
}

/// General application settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GeneralConfig {
    /// Enable verbose logging
    pub verbose: bool,
    /// Enable debug mode
    pub debug: bool,
    /// Log file location (relative to XDG data dir)
    pub log_file: Option<String>,
    /// Check for updates on startup
    pub check_updates: bool,
    /// Default operation mode: "user" or "system"
    pub mode: String,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            verbose: false,
            debug: false,
            log_file: Some("nixboost.log".to_string()),
            check_updates: true,
            mode: "user".to_string(),
        }
    }
}

/// Search-related settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SearchConfig {
    /// Maximum number of search results to display
    pub max_results: usize,
    /// Enable fuzzy matching
    pub fuzzy: bool,
    /// Fuzzy match threshold (0.0 - 1.0)
    pub fuzzy_threshold: f64,
    /// Include NUR in searches by default
    pub include_nur: bool,
    /// Parallel search threads
    pub parallel_threads: usize,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            max_results: 50,
            fuzzy: true,
            fuzzy_threshold: 0.6,
            include_nur: false,
            parallel_threads: 4,
        }
    }
}

/// Cache settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CacheConfig {
    /// Enable disk cache
    pub enabled: bool,
    /// Cache directory (relative to XDG cache dir)
    pub directory: String,
    /// Maximum cache size in MB
    pub max_size_mb: u64,
    /// TTL for package metadata in seconds (default: 1 hour)
    pub package_ttl_secs: u64,
    /// TTL for search results in seconds (default: 5 minutes)
    pub search_ttl_secs: u64,
    /// TTL for NUR index in seconds (default: 24 hours)
    pub nur_ttl_secs: u64,
    /// Enable compression (zstd)
    pub compression: bool,
    /// In-memory LRU cache size
    pub memory_cache_size: usize,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            directory: "nixboost".to_string(),
            max_size_mb: 500,
            package_ttl_secs: 3600,      // 1 hour
            search_ttl_secs: 300,         // 5 minutes
            nur_ttl_secs: 86400,          // 24 hours
            compression: true,
            memory_cache_size: 1000,
        }
    }
}

/// Network settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct NetworkConfig {
    /// Request timeout in seconds
    pub timeout_secs: u64,
    /// Connection timeout in seconds
    pub connect_timeout_secs: u64,
    /// Maximum retry attempts
    pub max_retries: u32,
    /// Retry delay in milliseconds
    pub retry_delay_ms: u64,
    /// HTTP proxy (optional)
    pub proxy: Option<String>,
    /// User agent string
    pub user_agent: String,
    /// Enable HTTP/2
    pub http2: bool,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            timeout_secs: 30,
            connect_timeout_secs: 10,
            max_retries: 3,
            retry_delay_ms: 1000,
            proxy: None,
            user_agent: format!("nixboost/{}", env!("CARGO_PKG_VERSION")),
            http2: true,
        }
    }
}

/// UI preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UiConfig {
    /// Enable colored output
    pub colors: bool,
    /// Enable progress bars
    pub progress: bool,
    /// Enable unicode symbols
    pub unicode: bool,
    /// Table style: "unicode", "ascii", "minimal"
    pub table_style: String,
    /// Progress bar refresh rate in milliseconds
    pub progress_refresh_ms: u64,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            colors: true,
            progress: true,
            unicode: true,
            table_style: "unicode".to_string(),
            progress_refresh_ms: 100,
        }
    }
}

impl Config {
    /// Get the configuration directory path
    pub fn config_dir() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("nixboost")
    }

    /// Get the configuration file path
    pub fn config_path() -> PathBuf {
        Self::config_dir().join("config.toml")
    }

    /// Get the cache directory path
    pub fn cache_dir() -> PathBuf {
        dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from(".cache"))
            .join("nixboost")
    }

    /// Get the data directory path
    pub fn data_dir() -> PathBuf {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from(".local/share"))
            .join("nixboost")
    }

    /// Load configuration from file, or create default if not exists
    pub fn load() -> Result<Self> {
        let path = Self::config_path();
        
        if path.exists() {
            debug!("Loading config from {:?}", path);
            let content = std::fs::read_to_string(&path)
                .map_err(|e| NixBoostError::Config(format!("Failed to read config: {}", e)))?;
            
            let config: Config = toml::from_str(&content)
                .map_err(|e| NixBoostError::Config(format!("Failed to parse config: {}", e)))?;
            
            info!("Configuration loaded successfully");
            Ok(config)
        } else {
            debug!("Config file not found, using defaults");
            let config = Config::default();
            
            // Try to save default config
            if let Err(e) = config.save() {
                warn!("Failed to save default config: {}", e);
            }
            
            Ok(config)
        }
    }

    /// Save configuration to file
    pub fn save(&self) -> Result<()> {
        let dir = Self::config_dir();
        std::fs::create_dir_all(&dir)
            .map_err(|e| NixBoostError::Config(format!("Failed to create config dir: {}", e)))?;
        
        let path = Self::config_path();
        let content = toml::to_string_pretty(self)
            .map_err(|e| NixBoostError::Config(format!("Failed to serialize config: {}", e)))?;
        
        std::fs::write(&path, content)
            .map_err(|e| NixBoostError::Config(format!("Failed to write config: {}", e)))?;
        
        info!("Configuration saved to {:?}", path);
        Ok(())
    }

    /// Initialize global configuration
    pub fn init() -> Result<&'static Config> {
        let config = Self::load()?;
        Ok(CONFIG.get_or_init(|| config))
    }

    /// Get global configuration (panics if not initialized)
    pub fn get() -> &'static Config {
        CONFIG.get().expect("Config not initialized - call Config::init() first")
    }

    /// Try to get global configuration
    pub fn try_get() -> Option<&'static Config> {
        CONFIG.get()
    }

    /// Apply environment variable overrides
    pub fn with_env_overrides(mut self) -> Self {
        // NIXBOOST_VERBOSE
        if std::env::var("NIXBOOST_VERBOSE").is_ok() {
            self.general.verbose = true;
        }
        
        // NIXBOOST_DEBUG
        if std::env::var("NIXBOOST_DEBUG").is_ok() {
            self.general.debug = true;
        }
        
        // NIXBOOST_NO_COLORS
        if std::env::var("NIXBOOST_NO_COLORS").is_ok() || std::env::var("NO_COLOR").is_ok() {
            self.ui.colors = false;
        }
        
        // NIXBOOST_NO_CACHE
        if std::env::var("NIXBOOST_NO_CACHE").is_ok() {
            self.cache.enabled = false;
        }
        
        // NIXBOOST_TIMEOUT
        if let Ok(timeout) = std::env::var("NIXBOOST_TIMEOUT") {
            if let Ok(secs) = timeout.parse() {
                self.network.timeout_secs = secs;
            }
        }
        
        // HTTP_PROXY / HTTPS_PROXY
        if let Ok(proxy) = std::env::var("HTTPS_PROXY").or_else(|_| std::env::var("HTTP_PROXY")) {
            self.network.proxy = Some(proxy);
        }
        
        self
    }
}

/// Generate default configuration file content
pub fn generate_default_config() -> String {
    let config = Config::default();
    toml::to_string_pretty(&config).unwrap_or_else(|_| String::from("# Failed to generate config"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert!(config.cache.enabled);
        assert!(config.ui.colors);
        assert_eq!(config.search.max_results, 50);
    }

    #[test]
    fn test_config_serialization() {
        let config = Config::default();
        let toml_str = toml::to_string(&config).unwrap();
        let parsed: Config = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.search.max_results, config.search.max_results);
    }

    #[test]
    fn test_config_paths() {
        let config_dir = Config::config_dir();
        assert!(config_dir.to_string_lossy().contains("nixboost"));
        
        let cache_dir = Config::cache_dir();
        assert!(cache_dir.to_string_lossy().contains("nixboost"));
    }

    #[test]
    fn test_generate_default_config() {
        let content = generate_default_config();
        assert!(content.contains("[general]"));
        assert!(content.contains("[search]"));
        assert!(content.contains("[cache]"));
    }
}
