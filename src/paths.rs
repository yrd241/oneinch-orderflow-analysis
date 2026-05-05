use std::path::PathBuf;

/// Default SQLite cache: `~/.cache/oneinch-orderflow/orderflow.db`.
pub fn default_cache_db() -> PathBuf {
    let base = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    base.join(".cache").join("oneinch-orderflow").join("orderflow.db")
}
