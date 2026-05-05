use std::path::PathBuf;

use anyhow::Result;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json};
use axum::routing::get;
use axum::Router;
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;
use tracing::info;

use crate::cli::ServeArgs;
use crate::paths;
use crate::snapshot::{load_snapshot, OrderflowSnapshot};
use crate::web::web_root;

#[derive(Clone)]
struct AppState {
    db: PathBuf,
    demo: bool,
}

pub async fn run(args: ServeArgs) -> Result<()> {
    let db = resolve_db(args.db);
    let state = AppState {
        db,
        demo: args.demo,
    };

    let static_dir = web_root();
    if !static_dir.join("index.html").exists() {
        tracing::warn!(
            "web/index.html not found at {} — API only",
            static_dir.display()
        );
    }

    let app = Router::new()
        .route("/api/summary", get(api_summary))
        .with_state(state)
        .fallback_service(ServeDir::new(static_dir))
        .layer(CorsLayer::permissive());

    let addr = format!("{}:{}", args.host, args.port);
    let listener = TcpListener::bind(&addr).await?;
    info!("Orderflow web UI → http://{}", addr);
    axum::serve(listener, app).await?;
    Ok(())
}

async fn api_summary(State(st): State<AppState>) -> impl IntoResponse {
    let result = tokio::task::spawn_blocking(move || load_snapshot(&st.db, st.demo))
        .await
        .map_err(|e| anyhow::anyhow!("task panic: {e}"))
        .and_then(|r| r);

    match result {
        Ok(s) => Json(ApiResponse::ok(s)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

#[derive(serde::Serialize)]
struct ApiResponse {
    ok: bool,
    data: OrderflowSnapshot,
}

impl ApiResponse {
    fn ok(data: OrderflowSnapshot) -> Self {
        Self { ok: true, data }
    }
}

fn resolve_db(override_path: Option<PathBuf>) -> PathBuf {
    override_path.unwrap_or_else(paths::default_cache_db)
}
