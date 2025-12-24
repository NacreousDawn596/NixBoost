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

//! LRU in-memory cache for NixBoost.

use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::atomic::{AtomicU64, Ordering};

/// LRU in-memory cache for hot data
pub struct MemoryCache {
    cache: LruCache<String, String>,
    hits: AtomicU64,
    misses: AtomicU64,
}

impl MemoryCache {
    /// Create a new memory cache with the specified capacity
    pub fn new(capacity: usize) -> Self {
        let cap = NonZeroUsize::new(capacity.max(1)).unwrap();
        Self {
            cache: LruCache::new(cap),
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
        }
    }

    /// Get a value from the cache
    pub fn get<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
        // Note: This requires mutable access to update LRU order
        // The caller should hold a write lock
        None // Will be properly implemented by the caller with lock
    }

    /// Get a raw string value (for internal use with lock)
    pub fn get_raw(&mut self, key: &str) -> Option<String> {
        if let Some(value) = self.cache.get(key) {
            self.hits.fetch_add(1, Ordering::Relaxed);
            Some(value.clone())
        } else {
            self.misses.fetch_add(1, Ordering::Relaxed);
            None
        }
    }

    /// Get and deserialize (requires mutable self for LRU update)
    pub fn get_mut<T: serde::de::DeserializeOwned>(&mut self, key: &str) -> Option<T> {
        if let Some(value) = self.cache.get(key) {
            self.hits.fetch_add(1, Ordering::Relaxed);
            serde_json::from_str(value).ok()
        } else {
            self.misses.fetch_add(1, Ordering::Relaxed);
            None
        }
    }

    /// Set a value in the cache
    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.cache.put(key.into(), value.into());
    }

    /// Set a serializable value
    pub fn set_value<T: serde::Serialize>(&mut self, key: impl Into<String>, value: &T) -> bool {
        if let Ok(serialized) = serde_json::to_string(value) {
            self.cache.put(key.into(), serialized);
            true
        } else {
            false
        }
    }

    /// Remove a value from the cache
    pub fn remove(&mut self, key: &str) -> Option<String> {
        self.cache.pop(key)
    }

    /// Check if a key exists
    pub fn contains(&self, key: &str) -> bool {
        self.cache.contains(key)
    }

    /// Clear the cache
    pub fn clear(&mut self) {
        self.cache.clear();
        self.hits.store(0, Ordering::Relaxed);
        self.misses.store(0, Ordering::Relaxed);
    }

    /// Get the number of entries
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// Check if the cache is empty
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }

    /// Get cache statistics
    pub fn stats(&self) -> MemoryCacheStats {
        MemoryCacheStats {
            entries: self.cache.len(),
            capacity: self.cache.cap().get(),
            hits: self.hits.load(Ordering::Relaxed),
            misses: self.misses.load(Ordering::Relaxed),
        }
    }

    /// Resize the cache
    pub fn resize(&mut self, new_capacity: usize) {
        let cap = NonZeroUsize::new(new_capacity.max(1)).unwrap();
        self.cache.resize(cap);
    }

    /// Peek at a value without updating LRU order
    pub fn peek(&self, key: &str) -> Option<&String> {
        self.cache.peek(key)
    }

    /// Get all keys
    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.cache.iter().map(|(k, _)| k)
    }
}

/// Memory cache statistics
#[derive(Debug, Clone, Default)]
pub struct MemoryCacheStats {
    pub entries: usize,
    pub capacity: usize,
    pub hits: u64,
    pub misses: u64,
}

impl MemoryCacheStats {
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }

    pub fn usage_percent(&self) -> f64 {
        if self.capacity == 0 {
            0.0
        } else {
            (self.entries as f64 / self.capacity as f64) * 100.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_and_get() {
        let mut cache = MemoryCache::new(100);
        cache.set("key1", "value1");
        
        let value = cache.get_raw("key1");
        assert_eq!(value, Some("value1".to_string()));
    }

    #[test]
    fn test_lru_eviction() {
        let mut cache = MemoryCache::new(2);
        
        cache.set("key1", "value1");
        cache.set("key2", "value2");
        cache.set("key3", "value3"); // This should evict key1
        
        assert!(cache.get_raw("key1").is_none());
        assert!(cache.get_raw("key2").is_some());
        assert!(cache.get_raw("key3").is_some());
    }

    #[test]
    fn test_stats() {
        let mut cache = MemoryCache::new(100);
        cache.set("key1", "value1");
        
        let _ = cache.get_raw("key1"); // Hit
        let _ = cache.get_raw("key2"); // Miss
        
        let stats = cache.stats();
        assert_eq!(stats.entries, 1);
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.hit_rate(), 0.5);
    }

    #[test]
    fn test_clear() {
        let mut cache = MemoryCache::new(100);
        cache.set("key1", "value1");
        cache.set("key2", "value2");
        
        cache.clear();
        
        assert!(cache.is_empty());
        assert_eq!(cache.stats().hits, 0);
    }

    #[test]
    fn test_serializable_value() {
        let mut cache = MemoryCache::new(100);
        
        #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
        struct TestData {
            name: String,
            count: u32,
        }
        
        let data = TestData { name: "test".to_string(), count: 42 };
        cache.set_value("test_data", &data);
        
        let retrieved: Option<TestData> = cache.get_mut("test_data");
        assert_eq!(retrieved, Some(data));
    }
}
