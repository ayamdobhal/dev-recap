use crate::ai::Summary;
use crate::config::Config;
use crate::error::Result;
use chrono::{DateTime, Duration, Utc};
use sled::Db;
use std::path::Path;

/// Cache for AI-generated summaries
pub struct SummaryCache {
    db: Db,
    ttl_hours: u32,
}

impl SummaryCache {
    /// Create or open a cache
    pub fn new(cache_dir: &Path, ttl_hours: u32) -> Result<Self> {
        // Ensure cache directory exists
        std::fs::create_dir_all(cache_dir)?;

        let db_path = cache_dir.join("summaries.sled");
        let db = sled::open(db_path)?;

        Ok(Self { db, ttl_hours })
    }

    /// Create cache from config
    pub fn from_config(config: &Config) -> Result<Self> {
        let cache_dir = Config::default_cache_dir()?;
        Self::new(&cache_dir, config.cache_ttl_hours)
    }

    /// Generate a cache key from repository path and commit hashes
    pub fn generate_key(repo_path: &str, commit_hashes: &[String]) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        repo_path.hash(&mut hasher);
        for hash in commit_hashes {
            hash.hash(&mut hasher);
        }

        format!("summary_{:x}", hasher.finish())
    }

    /// Get a summary from cache if it exists and is not expired
    pub fn get(&self, key: &str) -> Result<Option<Summary>> {
        if let Some(data) = self.db.get(key)? {
            let cached: CachedSummary = serde_json::from_slice(&data)?;

            // Check if expired
            if self.is_expired(&cached.cached_at) {
                // Remove expired entry
                self.db.remove(key)?;
                return Ok(None);
            }

            Ok(Some(cached.summary))
        } else {
            Ok(None)
        }
    }

    /// Store a summary in cache
    pub fn set(&self, key: &str, summary: Summary) -> Result<()> {
        let cached = CachedSummary {
            summary,
            cached_at: Utc::now(),
        };

        let data = serde_json::to_vec(&cached)?;
        self.db.insert(key, data)?;
        self.db.flush()?;

        Ok(())
    }

    /// Check if a cache entry is expired
    fn is_expired(&self, cached_at: &DateTime<Utc>) -> bool {
        let now = Utc::now();
        let ttl = Duration::hours(self.ttl_hours as i64);
        now - *cached_at > ttl
    }

    /// Clear all cache entries
    pub fn clear(&self) -> Result<()> {
        self.db.clear()?;
        self.db.flush()?;
        Ok(())
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        let total_entries = self.db.len();
        let db_size = self.db.size_on_disk().unwrap_or(0);

        CacheStats {
            total_entries,
            db_size_bytes: db_size,
        }
    }

    /// Remove expired entries
    pub fn cleanup_expired(&self) -> Result<usize> {
        let mut removed = 0;

        for item in self.db.iter() {
            let (key, value) = item?;

            if let Ok(cached) = serde_json::from_slice::<CachedSummary>(&value) {
                if self.is_expired(&cached.cached_at) {
                    self.db.remove(key)?;
                    removed += 1;
                }
            }
        }

        self.db.flush()?;
        Ok(removed)
    }
}

/// Cached summary with metadata
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct CachedSummary {
    summary: Summary,
    cached_at: DateTime<Utc>,
}

/// Cache statistics
#[derive(Debug)]
pub struct CacheStats {
    pub total_entries: usize,
    pub db_size_bytes: u64,
}

impl CacheStats {
    /// Format size in human-readable format
    pub fn format_size(&self) -> String {
        let bytes = self.db_size_bytes;
        if bytes < 1024 {
            format!("{} B", bytes)
        } else if bytes < 1024 * 1024 {
            format!("{:.2} KB", bytes as f64 / 1024.0)
        } else {
            format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::Summary;
    use tempfile::TempDir;

    #[test]
    fn test_cache_creation() {
        let temp_dir = TempDir::new().unwrap();
        let cache = SummaryCache::new(temp_dir.path(), 24).unwrap();
        assert_eq!(cache.ttl_hours, 24);
    }

    #[test]
    fn test_cache_key_generation() {
        let key1 = SummaryCache::generate_key("/path/to/repo", &vec!["abc123".to_string()]);
        let key2 = SummaryCache::generate_key("/path/to/repo", &vec!["abc123".to_string()]);
        let key3 = SummaryCache::generate_key("/path/to/repo", &vec!["def456".to_string()]);

        // Same inputs should produce same key
        assert_eq!(key1, key2);
        // Different inputs should produce different keys
        assert_ne!(key1, key3);
    }

    #[test]
    fn test_cache_set_and_get() {
        let temp_dir = TempDir::new().unwrap();
        let cache = SummaryCache::new(temp_dir.path(), 24).unwrap();

        let summary = Summary::new(
            "test-repo".to_string(),
            "Test summary".to_string(),
            vec!["Achievement".to_string()],
            vec!["Tip".to_string()],
        );

        let key = "test_key";
        cache.set(key, summary.clone()).unwrap();

        let retrieved = cache.get(key).unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().repository, "test-repo");
    }

    #[test]
    fn test_cache_expiration() {
        let temp_dir = TempDir::new().unwrap();
        let cache = SummaryCache::new(temp_dir.path(), 0).unwrap(); // 0 hour TTL

        let summary = Summary::new(
            "test-repo".to_string(),
            "Test summary".to_string(),
            vec![],
            vec![],
        );

        let key = "test_key";
        cache.set(key, summary).unwrap();

        // Should be expired immediately with 0 TTL
        // Sleep a bit to ensure time has passed
        std::thread::sleep(std::time::Duration::from_millis(100));
        let retrieved = cache.get(key).unwrap();
        assert!(retrieved.is_none());
    }

    #[test]
    fn test_cache_clear() {
        let temp_dir = TempDir::new().unwrap();
        let cache = SummaryCache::new(temp_dir.path(), 24).unwrap();

        let summary = Summary::new(
            "test-repo".to_string(),
            "Test".to_string(),
            vec![],
            vec![],
        );

        cache.set("key1", summary.clone()).unwrap();
        cache.set("key2", summary).unwrap();

        let stats = cache.stats();
        assert_eq!(stats.total_entries, 2);

        cache.clear().unwrap();

        let stats = cache.stats();
        assert_eq!(stats.total_entries, 0);
    }

    #[test]
    fn test_cache_stats() {
        let temp_dir = TempDir::new().unwrap();
        let cache = SummaryCache::new(temp_dir.path(), 24).unwrap();

        let stats = cache.stats();
        assert_eq!(stats.total_entries, 0);

        let summary = Summary::new(
            "test-repo".to_string(),
            "Test".to_string(),
            vec![],
            vec![],
        );
        cache.set("key", summary).unwrap();

        let stats = cache.stats();
        assert_eq!(stats.total_entries, 1);
        assert!(stats.db_size_bytes > 0);
    }
}
