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

//! Cache module for NixBoost - persistent SQLite cache and in-memory LRU cache.

pub mod disk_cache;
pub mod memory_cache;
pub mod invalidation;

pub use disk_cache::DiskCache;
pub use memory_cache::MemoryCache;
pub use invalidation::CacheInvalidator;

use crate::core::error::Result;
use std::sync::Arc;
use parking_lot::RwLock;

/// Combined cache manager with memory and disk caching
pub struct CacheManager {
    /// In-memory LRU cache for hot data
    pub memory: Arc<RwLock<MemoryCache>>,
    /// Persistent SQLite cache
    pub disk: Arc<DiskCache>,
    /// Cache invalidator
    pub invalidator: Arc<CacheInvalidator>,
}

impl CacheManager {
    /// Create a new cache manager
    pub fn new(memory_size: usize) -> Result<Self> {
        let memory = Arc::new(RwLock::new(MemoryCache::new(memory_size)));
        let disk = Arc::new(DiskCache::new()?);
        let invalidator = Arc::new(CacheInvalidator::new());

        Ok(Self {
            memory,
            disk,
            invalidator,
        })
    }

    /// Get a value, checking memory first, then disk
    pub fn get<T: serde::de::DeserializeOwned + serde::Serialize + Clone>(&self, key: &str) -> Option<T> {
        // Try memory cache first
        if let Some(value) = self.memory.read().get::<T>(key) {
            return Some(value);
        }

        // Try disk cache
        if let Ok(Some(value)) = self.disk.get::<T>(key) {
            // Promote to memory cache
            if let Ok(serialized) = serde_json::to_string(&value) {
                self.memory.write().set(key, serialized);
            }
            return Some(value);
        }

        None
    }

    /// Set a value in both caches
    pub fn set<T: serde::Serialize>(&self, key: &str, value: &T, ttl_secs: u64) -> Result<()> {
        let serialized = serde_json::to_string(value)
            .map_err(|e| crate::core::error::CacheError::WriteError(e.to_string()))?;

        // Store in memory
        self.memory.write().set(key, serialized.clone());

        // Store on disk
        self.disk.set(key, &serialized, ttl_secs)?;

        Ok(())
    }

    /// Clear all caches
    pub fn clear(&self) -> Result<()> {
        self.memory.write().clear();
        self.disk.clear()?;
        Ok(())
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        let memory_stats = self.memory.read().stats();
        let disk_stats = self.disk.stats().unwrap_or_default();

        CacheStats {
            memory_entries: memory_stats.entries,
            memory_hits: memory_stats.hits,
            memory_misses: memory_stats.misses,
            disk_entries: disk_stats.entries,
            disk_size_bytes: disk_stats.size_bytes,
            disk_hits: disk_stats.hits,
            disk_misses: disk_stats.misses,
        }
    }
}

/// Cache statistics
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    pub memory_entries: usize,
    pub memory_hits: u64,
    pub memory_misses: u64,
    pub disk_entries: usize,
    pub disk_size_bytes: u64,
    pub disk_hits: u64,
    pub disk_misses: u64,
}

impl CacheStats {
    pub fn total_entries(&self) -> usize {
        self.memory_entries + self.disk_entries
    }

    pub fn hit_rate(&self) -> f64 {
        let total_hits = self.memory_hits + self.disk_hits;
        let total_misses = self.memory_misses + self.disk_misses;
        let total = total_hits + total_misses;
        if total == 0 {
            0.0
        } else {
            total_hits as f64 / total as f64
        }
    }

    pub fn size_human(&self) -> String {
        let bytes = self.disk_size_bytes;
        if bytes < 1024 {
            format!("{} B", bytes)
        } else if bytes < 1024 * 1024 {
            format!("{:.1} KB", bytes as f64 / 1024.0)
        } else {
            format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
        }
    }
}
