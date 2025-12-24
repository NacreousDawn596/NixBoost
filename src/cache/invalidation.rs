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

//! Cache invalidation strategies for NixBoost.

use std::time::{Duration, Instant};
use std::sync::atomic::{AtomicU64, Ordering};
use tracing::debug;

/// Cache invalidation manager
pub struct CacheInvalidator {
    /// Last global invalidation time
    last_invalidation: AtomicU64,
}

impl CacheInvalidator {
    /// Create a new cache invalidator
    pub fn new() -> Self {
        Self {
            last_invalidation: AtomicU64::new(0),
        }
    }

    /// Trigger a global cache invalidation
    pub fn invalidate_all(&self) {
        let now = current_epoch_ms();
        self.last_invalidation.store(now, Ordering::SeqCst);
        debug!("Global cache invalidation triggered");
    }

    /// Check if a cached entry is still valid
    pub fn is_valid(&self, cached_at_ms: u64) -> bool {
        let last_invalidation = self.last_invalidation.load(Ordering::SeqCst);
        cached_at_ms > last_invalidation
    }

    /// Get time since last invalidation
    pub fn time_since_invalidation(&self) -> Duration {
        let last = self.last_invalidation.load(Ordering::SeqCst);
        let now = current_epoch_ms();
        Duration::from_millis(now.saturating_sub(last))
    }
}

impl Default for CacheInvalidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Cache key builder for consistent key generation
pub struct CacheKey;

impl CacheKey {
    /// Create a search results cache key
    pub fn search(query: &str) -> String {
        format!("search:{}", query.to_lowercase())
    }

    /// Create a package metadata cache key
    pub fn package(name: &str) -> String {
        format!("pkg:{}", name)
    }

    /// Create a NUR index cache key
    pub fn nur_index() -> String {
        "nur:index".to_string()
    }

    /// Create a NUR package cache key
    pub fn nur_package(name: &str) -> String {
        format!("nur:pkg:{}", name)
    }

    /// Create a dependency tree cache key
    pub fn dependencies(package: &str) -> String {
        format!("deps:{}", package)
    }

    /// Create an installed packages cache key
    pub fn installed() -> String {
        "installed".to_string()
    }

    /// Create a generations cache key
    pub fn generations() -> String {
        "generations".to_string()
    }
}

/// TTL (Time-To-Live) constants
pub struct TTL;

impl TTL {
    /// Search results TTL (5 minutes)
    pub const SEARCH: u64 = 300;
    
    /// Package metadata TTL (1 hour)
    pub const PACKAGE: u64 = 3600;
    
    /// NUR index TTL (24 hours)
    pub const NUR_INDEX: u64 = 86400;
    
    /// NUR package TTL (1 hour)
    pub const NUR_PACKAGE: u64 = 3600;
    
    /// Installed packages TTL (1 minute - changes frequently)
    pub const INSTALLED: u64 = 60;
    
    /// Generations TTL (5 minutes)
    pub const GENERATIONS: u64 = 300;
    
    /// Dependencies TTL (1 hour)
    pub const DEPENDENCIES: u64 = 3600;
    
    /// Short TTL for temporary data (30 seconds)
    pub const SHORT: u64 = 30;
    
    /// Long TTL for stable data (1 week)
    pub const LONG: u64 = 604800;
}

fn current_epoch_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_key_search() {
        let key = CacheKey::search("Firefox");
        assert_eq!(key, "search:firefox");
    }

    #[test]
    fn test_cache_key_package() {
        let key = CacheKey::package("firefox");
        assert_eq!(key, "pkg:firefox");
    }

    #[test]
    fn test_invalidator() {
        let invalidator = CacheInvalidator::new();
        
        let cached_at = current_epoch_ms();
        assert!(invalidator.is_valid(cached_at));
        
        // Wait longer and invalidate
        std::thread::sleep(std::time::Duration::from_millis(50));
        invalidator.invalidate_all();
        
        // Old cache should be invalid
        assert!(!invalidator.is_valid(cached_at));
        
        // Wait a bit then new cache should be valid
        std::thread::sleep(std::time::Duration::from_millis(10));
        let new_cached_at = current_epoch_ms();
        assert!(invalidator.is_valid(new_cached_at));
    }
}
