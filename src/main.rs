mod cli;
mod dune;
mod model;
mod paths;
mod snapshot;
mod web;

use anyhow::Result;
use clap::Parser;
use tracing_subscriber::EnvFilter;

use crate::cli::{Cli, Commands};
use crate::paths::default_cache_db;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();
    match cli.command {
        Commands::Fetch(args) => {
            dune::run_fetch(args).await?;
        }
        Commands::Serve(args) => {
            web::run_serve(args).await?;
        }
        Commands::Export(args) => {
            let db = args.db.unwrap_or_else(default_cache_db);
            let snap = snapshot::load_snapshot(&db, true)?;
            std::fs::write(&args.out, serde_json::to_string_pretty(&snap)?)?;
        }
        Commands::ExportWeb(args) => {
            let db = args.db.unwrap_or_else(default_cache_db);
            std::fs::create_dir_all(&args.out_dir)?;
            let snap = snapshot::load_snapshot(&db, true)?;
            let summary = serde_json::json!({ "ok": true, "data": snap });
            let summary_path = args.out_dir.join("summary.json");
            std::fs::write(&summary_path, serde_json::to_string(&summary)?)?;
            tracing::info!("wrote {}", summary_path.display());

            let index = snapshot::export_addresses_index(&db)?;
            let key_count = index.len();
            let addresses = serde_json::json!({ "ok": true, "index": index });
            let addresses_path = args.out_dir.join("addresses.json");
            std::fs::write(&addresses_path, serde_json::to_string(&addresses)?)?;
            tracing::info!(
                "wrote {} ({key_count} keys)",
                addresses_path.display()
            );
        }
    }
    Ok(())
}
