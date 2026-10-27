use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CacheEntry {
    pub data: Vec<u8>,
    pub created_at: u64,
    pub ttl_secs: u64,
}

impl CacheEntry {
    pub fn is_expired(&self, now: u64) -> bool {
        self.ttl_secs > 0 && now > self.created_at.saturating_add(self.ttl_secs)
    }
}

pub fn default_cache_dir(app_name: &str) -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    let base = std::env::var_os("HOME").map(|h| PathBuf::from(h).join("Library/Caches"));
    #[cfg(target_os = "windows")]
    let base = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);
    #[cfg(all(unix, not(target_os = "macos")))]
    let base = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")));
    base.map(|b| b.join(app_name).join("http"))
}

pub struct CacheStore {
    memory: RwLock<HashMap<String, CacheEntry>>,
    disk_dir: Option<PathBuf>,
}

impl Default for CacheStore {
    fn default() -> Self {
        Self::new("TemplateApp")
    }
}

impl CacheStore {
    pub fn new(app_name: &str) -> Self {
        let disk_dir = default_cache_dir(app_name);
        if let Some(dir) = &disk_dir {
            let _ = fs::create_dir_all(dir);
        }
        Self {
            memory: RwLock::new(HashMap::new()),
            disk_dir,
        }
    }

    fn now() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }

    fn hash_key(key: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(key.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Retrieve from in-memory cache, falling back to disk cache.
    pub fn get(&self, key: &str) -> Option<Vec<u8>> {
        let now = Self::now();

        // 1. Check in-memory tier
        if let Ok(mem) = self.memory.read()
            && let Some(entry) = mem.get(key)
            && !entry.is_expired(now)
        {
            return Some(entry.data.clone());
        }

        // 2. Check disk cache tier
        let dir = self.disk_dir.as_ref()?;
        let hash = Self::hash_key(key);
        let path = dir.join(format!("{hash}.json"));
        let bytes = fs::read(&path).ok()?;
        let entry: CacheEntry = serde_json::from_slice(&bytes).ok()?;

        if entry.is_expired(now) {
            let _ = fs::remove_file(path);
            return None;
        }

        // Promote back to in-memory tier
        if let Ok(mut mem) = self.memory.write() {
            mem.insert(key.to_string(), entry.clone());
        }

        Some(entry.data)
    }

    /// Store response in both in-memory and disk cache tiers.
    pub fn put(&self, key: &str, data: Vec<u8>, ttl_secs: u64) {
        let entry = CacheEntry {
            data,
            created_at: Self::now(),
            ttl_secs,
        };

        // 1. In-memory
        if let Ok(mut mem) = self.memory.write() {
            mem.insert(key.to_string(), entry.clone());
        }

        // 2. Disk
        if let Some(dir) = &self.disk_dir {
            let hash = Self::hash_key(key);
            let path = dir.join(format!("{hash}.json"));
            if let Ok(serialized) = serde_json::to_vec(&entry) {
                let _ = fs::write(path, serialized);
            }
        }
    }
}
