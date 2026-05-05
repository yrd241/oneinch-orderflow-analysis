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
    }
    Ok(())
}
