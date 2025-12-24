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

//! SQLite-based persistent cache for NixBoost.

use crate::core::config::Config;
use crate::core::error::{CacheError, Result};
use rusqlite::{Connection, params};
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, info, warn};

/// Persistent SQLite-based disk cache
pub struct DiskCache {
    conn: Mutex<Connection>,
    path: PathBuf,
}

impl DiskCache {
    /// Create a new disk cache
    pub fn new() -> Result<Self> {
        let path = Config::cache_dir().join("cache.db");
        Self::with_path(path)
    }

    /// Create a disk cache at a specific path
    pub fn with_path(path: PathBuf) -> Result<Self> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| CacheError::InitFailed(e.to_string()))?;
        }

        debug!("Opening cache database at {:?}", path);
        let conn = Connection::open(&path)
            .map_err(|e| CacheError::InitFailed(e.to_string()))?;

        // Initialize schema
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS cache (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                expires_at INTEGER NOT NULL,
                access_count INTEGER DEFAULT 0,
                last_accessed INTEGER
            );
            
            CREATE INDEX IF NOT EXISTS idx_expires ON cache(expires_at);
            CREATE INDEX IF NOT EXISTS idx_key_prefix ON cache(key);
            
            -- Metadata table for stats
            CREATE TABLE IF NOT EXISTS metadata (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            
            -- Initialize hit/miss counters
            INSERT OR IGNORE INTO metadata (key, value) VALUES ('hits', '0');
            INSERT OR IGNORE INTO metadata (key, value) VALUES ('misses', '0');
            "
        ).map_err(|e| CacheError::InitFailed(e.to_string()))?;

        // Enable WAL mode for better performance
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")
            .map_err(|e| CacheError::InitFailed(e.to_string()))?;

        info!("Cache database initialized");

        Ok(Self {
            conn: Mutex::new(conn),
            path,
        })
    }

    /// Get a value from the cache
    pub fn get<T: serde::de::DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        let conn = self.conn.lock().map_err(|e| CacheError::ReadError(e.to_string()))?;
        let now = current_timestamp();

        // Try to get the value
        let result: rusqlite::Result<(String, i64)> = conn.query_row(
            "SELECT value, expires_at FROM cache WHERE key = ?1",
            params![key],
            |row| Ok((row.get(0)?, row.get(1)?)),
        );

        match result {
            Ok((value, expires_at)) => {
                if expires_at < now as i64 {
                    // Expired, delete it
                    debug!("Cache entry expired: {}", key);
                    let _ = conn.execute("DELETE FROM cache WHERE key = ?1", params![key]);
                    self.increment_misses(&conn)?;
                    return Ok(None);
                }

                // Update access stats
                let _ = conn.execute(
                    "UPDATE cache SET access_count = access_count + 1, last_accessed = ?2 WHERE key = ?1",
                    params![key, now],
                );
                self.increment_hits(&conn)?;

                // Deserialize
                let parsed: T = serde_json::from_str(&value)
                    .map_err(|e| CacheError::ReadError(format!("Deserialize error: {}", e)))?;
                Ok(Some(parsed))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                self.increment_misses(&conn)?;
                Ok(None)
            }
            Err(e) => Err(CacheError::ReadError(e.to_string()).into()),
        }
    }

    /// Set a value in the cache
    pub fn set(&self, key: &str, value: &str, ttl_secs: u64) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| CacheError::WriteError(e.to_string()))?;
        let now = current_timestamp();
        let expires_at = now + ttl_secs;

        conn.execute(
            "INSERT OR REPLACE INTO cache (key, value, created_at, expires_at, access_count, last_accessed)
             VALUES (?1, ?2, ?3, ?4, 0, ?3)",
            params![key, value, now, expires_at],
        ).map_err(|e| CacheError::WriteError(e.to_string()))?;

        debug!("Cached key: {} (ttl: {}s)", key, ttl_secs);
        Ok(())
    }

    /// Delete a specific key
    pub fn delete(&self, key: &str) -> Result<bool> {
        let conn = self.conn.lock().map_err(|e| CacheError::WriteError(e.to_string()))?;
        let affected = conn.execute("DELETE FROM cache WHERE key = ?1", params![key])
            .map_err(|e| CacheError::WriteError(e.to_string()))?;
        Ok(affected > 0)
    }

    /// Delete entries matching a prefix
    pub fn delete_prefix(&self, prefix: &str) -> Result<usize> {
        let conn = self.conn.lock().map_err(|e| CacheError::WriteError(e.to_string()))?;
        let pattern = format!("{}%", prefix);
        let affected = conn.execute("DELETE FROM cache WHERE key LIKE ?1", params![pattern])
            .map_err(|e| CacheError::WriteError(e.to_string()))?;
        Ok(affected)
    }

    /// Clear all cache entries
    pub fn clear(&self) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| CacheError::WriteError(e.to_string()))?;
        conn.execute("DELETE FROM cache", [])
            .map_err(|e| CacheError::WriteError(e.to_string()))?;
        
        // Reset counters
        conn.execute("UPDATE metadata SET value = '0' WHERE key IN ('hits', 'misses')", [])
            .map_err(|e| CacheError::WriteError(e.to_string()))?;
        
        info!("Cache cleared");
        Ok(())
    }

    /// Prune expired entries
    pub fn prune(&self) -> Result<usize> {
        let conn = self.conn.lock().map_err(|e| CacheError::WriteError(e.to_string()))?;
        let now = current_timestamp();
        let affected = conn.execute("DELETE FROM cache WHERE expires_at < ?1", params![now])
            .map_err(|e| CacheError::WriteError(e.to_string()))?;
        
        if affected > 0 {
            info!("Pruned {} expired cache entries", affected);
        }
        
        Ok(affected)
    }

    /// Vacuum the database to reclaim space
    pub fn vacuum(&self) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| CacheError::WriteError(e.to_string()))?;
        conn.execute("VACUUM", [])
            .map_err(|e| CacheError::WriteError(e.to_string()))?;
        info!("Cache database vacuumed");
        Ok(())
    }

    /// Get cache statistics
    pub fn stats(&self) -> Result<DiskCacheStats> {
        let conn = self.conn.lock().map_err(|e| CacheError::ReadError(e.to_string()))?;

        let entries: usize = conn.query_row(
            "SELECT COUNT(*) FROM cache",
            [],
            |row| row.get(0),
        ).unwrap_or(0);

        let size_bytes = std::fs::metadata(&self.path)
            .map(|m| m.len())
            .unwrap_or(0);

        let hits: u64 = conn.query_row(
            "SELECT CAST(value AS INTEGER) FROM metadata WHERE key = 'hits'",
            [],
            |row| row.get(0),
        ).unwrap_or(0);

        let misses: u64 = conn.query_row(
            "SELECT CAST(value AS INTEGER) FROM metadata WHERE key = 'misses'",
            [],
            |row| row.get(0),
        ).unwrap_or(0);

        let expired: usize = conn.query_row(
            "SELECT COUNT(*) FROM cache WHERE expires_at < ?1",
            params![current_timestamp()],
            |row| row.get(0),
        ).unwrap_or(0);

        Ok(DiskCacheStats {
            entries,
            size_bytes,
            hits,
            misses,
            expired,
        })
    }

    /// Check if a key exists and is valid
    pub fn contains(&self, key: &str) -> bool {
        if let Ok(conn) = self.conn.lock() {
            let now = current_timestamp();
            conn.query_row(
                "SELECT 1 FROM cache WHERE key = ?1 AND expires_at > ?2",
                params![key, now],
                |_| Ok(()),
            ).is_ok()
        } else {
            false
        }
    }

    fn increment_hits(&self, conn: &Connection) -> Result<()> {
        conn.execute(
            "UPDATE metadata SET value = CAST(CAST(value AS INTEGER) + 1 AS TEXT) WHERE key = 'hits'",
            [],
        ).map_err(|e| CacheError::WriteError(e.to_string()))?;
        Ok(())
    }

    fn increment_misses(&self, conn: &Connection) -> Result<()> {
        conn.execute(
            "UPDATE metadata SET value = CAST(CAST(value AS INTEGER) + 1 AS TEXT) WHERE key = 'misses'",
            [],
        ).map_err(|e| CacheError::WriteError(e.to_string()))?;
        Ok(())
    }
}

/// Disk cache statistics
#[derive(Debug, Clone, Default)]
pub struct DiskCacheStats {
    pub entries: usize,
    pub size_bytes: u64,
    pub hits: u64,
    pub misses: u64,
    pub expired: usize,
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_cache() -> (DiskCache, TempDir) {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("test_cache.db");
        let cache = DiskCache::with_path(path).unwrap();
        (cache, tmp)
    }

    #[test]
    fn test_set_and_get() {
        let (cache, _tmp) = create_test_cache();
        
        cache.set("test_key", r#"{"name": "test"}"#, 3600).unwrap();
        
        let result: Option<serde_json::Value> = cache.get("test_key").unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap()["name"], "test");
    }

    #[test]
    fn test_expiration() {
        let (cache, _tmp) = create_test_cache();
        
        // Set with 1 second TTL
        cache.set("expired_key", r#""test""#, 1).unwrap();
        
        // Wait for expiration
        std::thread::sleep(std::time::Duration::from_secs(2));
        
        // Should not return expired value
        let result: Option<String> = cache.get("expired_key").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_delete() {
        let (cache, _tmp) = create_test_cache();
        
        cache.set("to_delete", r#""value""#, 3600).unwrap();
        assert!(cache.contains("to_delete"));
        
        cache.delete("to_delete").unwrap();
        assert!(!cache.contains("to_delete"));
    }

    #[test]
    fn test_delete_prefix() {
        let (cache, _tmp) = create_test_cache();
        
        cache.set("search:query1", r#""a""#, 3600).unwrap();
        cache.set("search:query2", r#""b""#, 3600).unwrap();
        cache.set("package:pkg1", r#""c""#, 3600).unwrap();
        
        let deleted = cache.delete_prefix("search:").unwrap();
        assert_eq!(deleted, 2);
        assert!(!cache.contains("search:query1"));
        assert!(cache.contains("package:pkg1"));
    }

    #[test]
    fn test_stats() {
        let (cache, _tmp) = create_test_cache();
        
        cache.set("key1", r#""value1""#, 3600).unwrap();
        cache.set("key2", r#""value2""#, 3600).unwrap();
        
        let stats = cache.stats().unwrap();
        assert_eq!(stats.entries, 2);
    }

    #[test]
    fn test_clear() {
        let (cache, _tmp) = create_test_cache();
        
        cache.set("key1", r#""value1""#, 3600).unwrap();
        cache.set("key2", r#""value2""#, 3600).unwrap();
        
        cache.clear().unwrap();
        
        let stats = cache.stats().unwrap();
        assert_eq!(stats.entries, 0);
    }
}
