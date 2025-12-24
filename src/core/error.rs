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

//! Error handling for NixBoost - structured errors with context and recovery suggestions.

use std::fmt;
use thiserror::Error;

/// Result type alias for NixBoost operations
pub type Result<T> = std::result::Result<T, NixBoostError>;

/// Main error type for NixBoost
#[derive(Error, Debug)]
pub enum NixBoostError {
    /// Configuration errors
    #[error("Configuration error: {0}")]
    Config(String),

    /// Package-related errors
    #[error("Package error: {0}")]
    Package(#[from] PackageError),

    /// Network-related errors
    #[error("Network error: {0}")]
    Network(#[from] NetworkError),

    /// Cache-related errors
    #[error("Cache error: {0}")]
    Cache(#[from] CacheError),

    /// System/Nix-related errors
    #[error("System error: {0}")]
    System(#[from] SystemError),

    /// Search-related errors
    #[error("Search error: {0}")]
    Search(#[from] SearchError),

    /// NUR-related errors
    #[error("NUR error: {0}")]
    Nur(#[from] NurError),

    /// IO errors
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization errors
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Generic wrapped error
    #[error("{0}")]
    Other(#[from] anyhow::Error),
}

/// Package operation errors
#[derive(Error, Debug)]
pub enum PackageError {
    #[error("Package not found: {name}")]
    NotFound { name: String },

    #[error("Package already installed: {name}")]
    AlreadyInstalled { name: String },

    #[error("Package not installed: {name}")]
    NotInstalled { name: String },

    #[error("Installation failed for {name}: {reason}")]
    InstallFailed { name: String, reason: String },

    #[error("Removal failed for {name}: {reason}")]
    RemoveFailed { name: String, reason: String },

    #[error("Dependency conflict: {0}")]
    DependencyConflict(String),

    #[error("Invalid package specification: {0}")]
    InvalidSpec(String),

    #[error("Version constraint not satisfied: {0}")]
    VersionMismatch(String),

    #[error("Transaction failed: {0}")]
    TransactionFailed(String),
}

/// Network-related errors
#[derive(Error, Debug)]
pub enum NetworkError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Request timeout after {timeout_secs}s")]
    Timeout { timeout_secs: u64 },

    #[error("HTTP error {status}: {message}")]
    HttpError { status: u16, message: String },

    #[error("DNS resolution failed: {0}")]
    DnsError(String),

    #[error("SSL/TLS error: {0}")]
    TlsError(String),

    #[error("Download failed: {0}")]
    DownloadFailed(String),

    #[error("All mirrors failed")]
    AllMirrorsFailed,

    #[error("Rate limited, retry after {retry_after_secs}s")]
    RateLimited { retry_after_secs: u64 },
}

/// Cache-related errors
#[derive(Error, Debug)]
pub enum CacheError {
    #[error("Cache initialization failed: {0}")]
    InitFailed(String),

    #[error("Cache read error: {0}")]
    ReadError(String),

    #[error("Cache write error: {0}")]
    WriteError(String),

    #[error("Cache corrupted: {0}")]
    Corrupted(String),

    #[error("Cache entry expired: {key}")]
    Expired { key: String },

    #[error("Cache full, max size: {max_size_mb}MB")]
    Full { max_size_mb: u64 },

    #[error("Database error: {0}")]
    DatabaseError(String),
}

/// System/Nix-related errors
#[derive(Error, Debug)]
pub enum SystemError {
    #[error("Nix command failed: {command}")]
    NixCommandFailed { command: String, stderr: String },

    #[error("Nix not found in PATH")]
    NixNotFound,

    #[error("Insufficient permissions: {0}")]
    PermissionDenied(String),

    #[error("Nix store verification failed: {0}")]
    StoreVerificationFailed(String),

    #[error("Generation not found: {generation}")]
    GenerationNotFound { generation: u64 },

    #[error("Rollback failed: {0}")]
    RollbackFailed(String),

    #[error("Health check failed: {0}")]
    HealthCheckFailed(String),

    #[error("Garbage collection failed: {0}")]
    GarbageCollectionFailed(String),

    #[error("Architecture detection failed")]
    ArchDetectionFailed,
}

/// Search-related errors
#[derive(Error, Debug)]
pub enum SearchError {
    #[error("Search query too short (min {min_length} chars)")]
    QueryTooShort { min_length: usize },

    #[error("Search query too long (max {max_length} chars)")]
    QueryTooLong { max_length: usize },

    #[error("Invalid search query: {0}")]
    InvalidQuery(String),

    #[error("Search index not available")]
    IndexNotAvailable,

    #[error("Search timeout")]
    Timeout,
}

/// NUR-related errors
#[derive(Error, Debug)]
pub enum NurError {
    #[error("NUR index not available")]
    IndexNotAvailable,

    #[error("NUR package not found: {name}")]
    PackageNotFound { name: String },

    #[error("Invalid NUR attribute path: {path}")]
    InvalidAttributePath { path: String },

    #[error("NUR repository not found: {repo}")]
    RepositoryNotFound { repo: String },

    #[error("NUR index update failed: {0}")]
    IndexUpdateFailed(String),
}

impl NixBoostError {
    /// Get an error code for scripting purposes
    pub fn code(&self) -> &'static str {
        match self {
            NixBoostError::Config(_) => "E001",
            NixBoostError::Package(_) => "E010",
            NixBoostError::Network(_) => "E020",
            NixBoostError::Cache(_) => "E030",
            NixBoostError::System(_) => "E040",
            NixBoostError::Search(_) => "E050",
            NixBoostError::Nur(_) => "E060",
            NixBoostError::Io(_) => "E070",
            NixBoostError::Serialization(_) => "E080",
            NixBoostError::Other(_) => "E999",
        }
    }

    /// Get a recovery suggestion for this error
    pub fn suggestion(&self) -> Option<&'static str> {
        match self {
            NixBoostError::Config(_) => {
                Some("Check your config file at ~/.config/nixboost/config.toml")
            }
            NixBoostError::Package(PackageError::NotFound { .. }) => {
                Some("Try searching with 'nixboost -Ss <query>' or check NUR with 'nixboost -A <query>'")
            }
            NixBoostError::Package(PackageError::AlreadyInstalled { .. }) => {
                Some("The package is already installed. Use 'nixboost -l' to list installed packages")
            }
            NixBoostError::Network(NetworkError::Timeout { .. }) => {
                Some("Check your internet connection or increase timeout in config")
            }
            NixBoostError::Network(NetworkError::AllMirrorsFailed) => {
                Some("All download sources failed. Check internet connection or try again later")
            }
            NixBoostError::Cache(CacheError::Corrupted(_)) => {
                Some("Clear cache with 'rm -rf ~/.cache/nixboost' and retry")
            }
            NixBoostError::System(SystemError::NixNotFound) => {
                Some("Ensure Nix is installed and in your PATH")
            }
            NixBoostError::System(SystemError::PermissionDenied(_)) => {
                Some("Try running with sudo or check file permissions")
            }
            NixBoostError::Nur(NurError::PackageNotFound { .. }) => {
                Some("Search NUR packages with 'nixboost -A <query>'")
            }
            _ => None,
        }
    }

    /// Check if this error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            NixBoostError::Network(NetworkError::Timeout { .. })
                | NixBoostError::Network(NetworkError::RateLimited { .. })
                | NixBoostError::Network(NetworkError::ConnectionFailed(_))
                | NixBoostError::Cache(CacheError::ReadError(_))
        )
    }
}

/// Extension trait for adding context to results
pub trait ResultExt<T> {
    /// Add context to an error
    fn context<C: fmt::Display>(self, context: C) -> Result<T>;
    
    /// Add context lazily
    fn with_context<C: fmt::Display, F: FnOnce() -> C>(self, f: F) -> Result<T>;
}

impl<T, E: Into<NixBoostError>> ResultExt<T> for std::result::Result<T, E> {
    fn context<C: fmt::Display>(self, context: C) -> Result<T> {
        self.map_err(|e| {
            let err = e.into();
            NixBoostError::Other(anyhow::anyhow!("{}: {}", context, err))
        })
    }

    fn with_context<C: fmt::Display, F: FnOnce() -> C>(self, f: F) -> Result<T> {
        self.map_err(|e| {
            let err = e.into();
            NixBoostError::Other(anyhow::anyhow!("{}: {}", f(), err))
        })
    }
}

/// Helper macro for creating package not found errors
#[macro_export]
macro_rules! pkg_not_found {
    ($name:expr) => {
        $crate::core::error::NixBoostError::Package(
            $crate::core::error::PackageError::NotFound { name: $name.to_string() }
        )
    };
}

/// Helper macro for creating network errors
#[macro_export]
macro_rules! network_error {
    ($msg:expr) => {
        $crate::core::error::NixBoostError::Network(
            $crate::core::error::NetworkError::ConnectionFailed($msg.to_string())
        )
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_codes() {
        let err = NixBoostError::Config("test".to_string());
        assert_eq!(err.code(), "E001");

        let err = NixBoostError::Package(PackageError::NotFound { name: "test".to_string() });
        assert_eq!(err.code(), "E010");
    }

    #[test]
    fn test_error_suggestions() {
        let err = NixBoostError::System(SystemError::NixNotFound);
        assert!(err.suggestion().is_some());
        assert!(err.suggestion().unwrap().contains("PATH"));
    }

    #[test]
    fn test_error_retryable() {
        let timeout_err = NixBoostError::Network(NetworkError::Timeout { timeout_secs: 30 });
        assert!(timeout_err.is_retryable());

        let not_found_err = NixBoostError::Package(PackageError::NotFound { name: "test".to_string() });
        assert!(!not_found_err.is_retryable());
    }

    #[test]
    fn test_error_display() {
        let err = PackageError::NotFound { name: "firefox".to_string() };
        assert_eq!(err.to_string(), "Package not found: firefox");

        let err = NetworkError::Timeout { timeout_secs: 30 };
        assert_eq!(err.to_string(), "Request timeout after 30s");
    }
}
