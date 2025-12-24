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

//! Core domain types for NixBoost.

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::time::SystemTime;

/// A Nix package with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Package {
    /// Package name (attribute path)
    pub name: String,
    /// Package version
    pub version: String,
    /// Package description
    pub description: String,
    /// Package source (nixpkgs, nur, flake)
    #[serde(default)]
    pub source: PackageSource,
    /// Full attribute path
    #[serde(default)]
    pub attr_path: Option<String>,
    /// Package homepage
    #[serde(default)]
    pub homepage: Option<String>,
    /// License information
    #[serde(default)]
    pub license: Option<String>,
    /// Maintainers
    #[serde(default)]
    pub maintainers: Vec<String>,
    /// Platforms supported
    #[serde(default)]
    pub platforms: Vec<String>,
}

impl Package {
    /// Create a new package with minimal info
    pub fn new(name: impl Into<String>, version: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            description: description.into(),
            source: PackageSource::Nixpkgs,
            attr_path: None,
            homepage: None,
            license: None,
            maintainers: Vec::new(),
            platforms: Vec::new(),
        }
    }

    /// Create a package from nixpkgs
    pub fn from_nixpkgs(name: impl Into<String>, version: impl Into<String>, description: impl Into<String>) -> Self {
        let mut pkg = Self::new(name, version, description);
        pkg.source = PackageSource::Nixpkgs;
        pkg
    }

    /// Create a package from NUR
    pub fn from_nur(name: impl Into<String>, version: impl Into<String>, description: impl Into<String>, repo: impl Into<String>) -> Self {
        let mut pkg = Self::new(name, version, description);
        pkg.source = PackageSource::Nur { repo: repo.into() };
        pkg
    }

    /// Get the install command for this package
    pub fn install_command(&self) -> String {
        match &self.source {
            PackageSource::Nixpkgs => format!("nix profile install nixpkgs#{}", self.name),
            PackageSource::Nur { repo } => format!("nix profile install github:nix-community/NUR#repos.{}.{}", repo, self.name),
            PackageSource::Flake { url } => format!("nix profile install {}#{}", url, self.name),
            PackageSource::Unknown => format!("nix profile install {}", self.name),
        }
    }

    /// Get display name with source prefix
    pub fn display_name(&self) -> String {
        match &self.source {
            PackageSource::Nixpkgs => format!("nixpkgs/{}", self.name),
            PackageSource::Nur { repo } => format!("nur/{}/{}", repo, self.name),
            PackageSource::Flake { url } => format!("{}#{}", url, self.name),
            PackageSource::Unknown => self.name.clone(),
        }
    }
}

impl PartialEq for Package {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.source == other.source
    }
}

impl Eq for Package {}

impl Hash for Package {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        // Source is intentionally not hashed for now
    }
}

impl fmt::Display for Package {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.name, self.version)
    }
}

/// Package source/repository
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum PackageSource {
    /// nixpkgs repository
    #[default]
    Nixpkgs,
    /// NUR (Nix User Repository)
    Nur { repo: String },
    /// A flake
    Flake { url: String },
    /// Unknown source
    Unknown,
}

impl fmt::Display for PackageSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PackageSource::Nixpkgs => write!(f, "nixpkgs"),
            PackageSource::Nur { repo } => write!(f, "nur:{}", repo),
            PackageSource::Flake { url } => write!(f, "flake:{}", url),
            PackageSource::Unknown => write!(f, "unknown"),
        }
    }
}

/// Installed package entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledPackage {
    /// Package info
    pub package: Package,
    /// When it was installed
    pub installed_at: SystemTime,
    /// Nix store path
    pub store_path: Option<String>,
    /// Profile element index
    pub profile_index: Option<u64>,
}

/// Search result with relevance score
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// The matched package
    pub package: Package,
    /// Relevance score (0.0 - 1.0)
    pub score: f64,
    /// Match type
    pub match_type: MatchType,
}

impl SearchResult {
    pub fn new(package: Package, score: f64, match_type: MatchType) -> Self {
        Self { package, score, match_type }
    }
}

impl PartialEq for SearchResult {
    fn eq(&self, other: &Self) -> bool {
        self.package == other.package
    }
}

impl Eq for SearchResult {}

impl PartialOrd for SearchResult {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SearchResult {
    fn cmp(&self, other: &Self) -> Ordering {
        // Higher score = better match
        other.score.partial_cmp(&self.score).unwrap_or(Ordering::Equal)
    }
}

/// Type of match in search results
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MatchType {
    /// Exact name match
    ExactName,
    /// Name prefix match
    NamePrefix,
    /// Name contains query
    NameContains,
    /// Description contains query
    DescriptionContains,
    /// Fuzzy match
    Fuzzy,
}

impl MatchType {
    /// Get the base score for this match type
    pub fn base_score(&self) -> f64 {
        match self {
            MatchType::ExactName => 1.0,
            MatchType::NamePrefix => 0.9,
            MatchType::NameContains => 0.7,
            MatchType::DescriptionContains => 0.5,
            MatchType::Fuzzy => 0.3,
        }
    }
}

/// Operation status for progress tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationStatus {
    Pending,
    Running,
    Success,
    Failed,
    Skipped,
    Cancelled,
}

impl fmt::Display for OperationStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OperationStatus::Pending => write!(f, "pending"),
            OperationStatus::Running => write!(f, "running"),
            OperationStatus::Success => write!(f, "success"),
            OperationStatus::Failed => write!(f, "failed"),
            OperationStatus::Skipped => write!(f, "skipped"),
            OperationStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// Operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationType {
    Install,
    Remove,
    Update,
    Search,
    GarbageCollect,
    Rollback,
}

impl fmt::Display for OperationType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OperationType::Install => write!(f, "install"),
            OperationType::Remove => write!(f, "remove"),
            OperationType::Update => write!(f, "update"),
            OperationType::Search => write!(f, "search"),
            OperationType::GarbageCollect => write!(f, "gc"),
            OperationType::Rollback => write!(f, "rollback"),
        }
    }
}

/// Result of a package operation
#[derive(Debug, Clone)]
pub struct OperationResult {
    /// Operation type
    pub operation: OperationType,
    /// Target package(s)
    pub packages: Vec<String>,
    /// Status
    pub status: OperationStatus,
    /// Duration in milliseconds
    pub duration_ms: u64,
    /// Error message if failed
    pub error: Option<String>,
    /// Detailed message
    pub message: Option<String>,
}

impl OperationResult {
    pub fn success(operation: OperationType, packages: Vec<String>, duration_ms: u64) -> Self {
        Self {
            operation,
            packages,
            status: OperationStatus::Success,
            duration_ms,
            error: None,
            message: None,
        }
    }

    pub fn failure(operation: OperationType, packages: Vec<String>, error: impl Into<String>) -> Self {
        Self {
            operation,
            packages,
            status: OperationStatus::Failed,
            duration_ms: 0,
            error: Some(error.into()),
            message: None,
        }
    }
}

/// Nix generation info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Generation {
    /// Generation number
    pub number: u64,
    /// Creation timestamp
    pub created_at: SystemTime,
    /// Whether this is the current generation
    pub is_current: bool,
    /// Path to the generation
    pub path: String,
}

/// Garbage collection preview
#[derive(Debug, Clone, Default)]
pub struct GCPreview {
    /// Paths that would be deleted
    pub paths: Vec<String>,
    /// Total size in bytes that would be freed
    pub size_bytes: u64,
    /// Generations that would be affected
    pub affected_generations: Vec<u64>,
}

impl GCPreview {
    /// Get human-readable size
    pub fn size_human(&self) -> String {
        let bytes = self.size_bytes;
        if bytes < 1024 {
            format!("{} B", bytes)
        } else if bytes < 1024 * 1024 {
            format!("{:.1} KB", bytes as f64 / 1024.0)
        } else if bytes < 1024 * 1024 * 1024 {
            format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
        } else {
            format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_package_creation() {
        let pkg = Package::new("firefox", "120.0", "Web browser");
        assert_eq!(pkg.name, "firefox");
        assert_eq!(pkg.version, "120.0");
        assert_eq!(pkg.source, PackageSource::Nixpkgs);
    }

    #[test]
    fn test_package_from_nur() {
        let pkg = Package::from_nur("somepackage", "1.0", "Description", "username");
        assert!(matches!(pkg.source, PackageSource::Nur { repo } if repo == "username"));
    }

    #[test]
    fn test_search_result_ordering() {
        let pkg1 = Package::new("test1", "1.0", "");
        let pkg2 = Package::new("test2", "1.0", "");
        
        let r1 = SearchResult::new(pkg1, 0.9, MatchType::ExactName);
        let r2 = SearchResult::new(pkg2, 0.5, MatchType::NameContains);
        
        assert!(r1 < r2); // r1 has higher score, so it should come first
    }

    #[test]
    fn test_gc_preview_size_human() {
        let preview = GCPreview {
            paths: vec![],
            size_bytes: 1024 * 1024 * 512, // 512 MB
            affected_generations: vec![],
        };
        assert!(preview.size_human().contains("MB"));
    }

    #[test]
    fn test_package_display_name() {
        let pkg = Package::from_nixpkgs("firefox", "120.0", "Browser");
        assert_eq!(pkg.display_name(), "nixpkgs/firefox");
        
        let nur_pkg = Package::from_nur("pkg", "1.0", "Desc", "user");
        assert_eq!(nur_pkg.display_name(), "nur/user/pkg");
    }
}
