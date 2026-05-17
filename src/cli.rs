use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "orderflow")]
#[command(
    about = "1inch orderflow dashboard — Dune → SQLite → web UI",
    version
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Execute registered Dune queries and refresh local SQLite cache
    Fetch(FetchArgs),
    /// Start web server (Sankey + charts); open http://HOST:PORT
    Serve(ServeArgs),
    /// Export current snapshot as JSON (same payload as /api/summary)
    Export(ExportArgs),
    /// Write static JSON under web/data/ for GitHub Pages
    ExportWeb(ExportWebArgs),
}

#[derive(clap::Args, Debug)]
pub struct FetchArgs {
    #[arg(long, env = "ORDERFLOW_DB")]
    pub db: Option<PathBuf>,
}

#[derive(clap::Args, Debug)]
pub struct ServeArgs {
    #[arg(long, env = "ORDERFLOW_DB")]
    pub db: Option<PathBuf>,
    #[arg(long, default_value = "127.0.0.1")]
    pub host: String,
    #[arg(long, default_value_t = 3000)]
    pub port: u16,
    /// Fall back to demo aggregates when cache is empty (same as before)
    #[arg(long, default_value_t = true)]
    pub demo: bool,
}

#[derive(clap::Args, Debug)]
pub struct ExportArgs {
    #[arg(long, env = "ORDERFLOW_DB")]
    pub db: Option<PathBuf>,
    #[arg(long, default_value = "export.json")]
    pub out: PathBuf,
}

#[derive(clap::Args, Debug)]
pub struct ExportWebArgs {
    #[arg(long, env = "ORDERFLOW_DB")]
    pub db: Option<PathBuf>,
    /// Output directory (summary.json, addresses.json)
    #[arg(long, default_value = "web/data")]
    pub out_dir: PathBuf,
}
