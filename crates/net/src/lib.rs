//! Layer 6: Asynchronous Network Engine with Dual-Tier (Memory + Disk) Caching.
//! Offloads network I/O from the UI thread and implements Stale-While-Revalidate caching.

pub mod cache;

use cache::CacheStore;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RequestPolicy {
    CacheOnly,
    NetworkOnly,
    CacheFirst,
    StaleWhileRevalidate,
}

/// Pluggable HTTP transport backend (e.g. reqwest, ureq, or native TcpStream).
pub trait HttpTransport: Send + Sync {
    fn fetch(&self, url: &str) -> Result<Vec<u8>, String>;
}

/// Default built-in transport providing simulated responses for demo URLs
/// and extensible for real HTTP adapters.
pub struct DefaultTransport;

impl HttpTransport for DefaultTransport {
    fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
        // Provide mock data for sample and testing URLs
        if url.contains("httpbin.org") || url.starts_with("mock://") {
            let mock_json = serde_json::json!({
                "origin": "127.0.0.1",
                "url": url,
                "timestamp": std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0),
                "headers": {
                    "User-Agent": "TemplateApp/0.1.0",
                    "Accept": "application/json"
                },
                "status": "success",
                "message": "Asynchronous response fetched via worker thread"
            });
            return serde_json::to_vec_pretty(&mock_json).map_err(|e| e.to_string());
        }

        Err(format!(
            "Transport for `{url}`: plug in reqwest or ureq for custom domains"
        ))
    }
}

#[derive(Clone)]
pub struct NetworkClient {
    pub cache: Arc<CacheStore>,
    transport: Arc<dyn HttpTransport>,
}

impl Default for NetworkClient {
    fn default() -> Self {
        Self::new()
    }
}

impl NetworkClient {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(CacheStore::default()),
            transport: Arc::new(DefaultTransport),
        }
    }

    pub fn with_transport(transport: Arc<dyn HttpTransport>) -> Self {
        Self {
            cache: Arc::new(CacheStore::default()),
            transport,
        }
    }

    /// Asynchronously fetches a URL off the UI thread according to the specified caching policy.
    /// `on_response` is called with `(result, is_from_cache)`.
    /// Under `StaleWhileRevalidate`, `on_response` is called immediately with cached data,
    /// and then called again once the fresh network data is fetched.
    pub fn fetch<F>(&self, url: &str, policy: RequestPolicy, ttl_secs: u64, on_response: F)
    where
        F: Fn(Result<Vec<u8>, String>, bool) + Send + Sync + 'static,
    {
        let url = url.to_string();
        let cache = self.cache.clone();
        let transport = self.transport.clone();
        let on_response = Arc::new(on_response);

        // Check cache tier first
        let cached = cache.get(&url);

        match policy {
            RequestPolicy::CacheOnly => {
                let res = cached.ok_or_else(|| "not found in cache".to_string());
                on_response(res, true);
                return;
            }
            RequestPolicy::CacheFirst => {
                if let Some(bytes) = cached {
                    on_response(Ok(bytes), true);
                    return;
                }
            }
            RequestPolicy::StaleWhileRevalidate => {
                if let Some(bytes) = cached {
                    // Yield cached data immediately (is_from_cache = true)
                    on_response(Ok(bytes), true);
                }
            }
            RequestPolicy::NetworkOnly => {}
        }

        // Spawn dedicated worker thread for non-blocking network I/O
        let cb = on_response.clone();
        std::thread::Builder::new()
            .name("net-worker".into())
            .spawn(move || {
                let res = transport.fetch(&url);
                match res {
                    Ok(bytes) => {
                        cache.put(&url, bytes.clone(), ttl_secs);
                        cb(Ok(bytes), false);
                    }
                    Err(e) => cb(Err(e), false),
                }
            })
            .ok();
    }
}
