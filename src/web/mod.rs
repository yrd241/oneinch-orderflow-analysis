mod server;

use std::path::PathBuf;

use anyhow::Result;

use crate::cli::ServeArgs;

pub async fn run_serve(args: ServeArgs) -> Result<()> {
    server::run(args).await
}

pub fn web_root() -> PathBuf {
    if let Ok(p) = std::env::var("ORDERFLOW_WEB_ROOT") {
        return PathBuf::from(p);
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("web")
}
