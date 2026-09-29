use ai_review_ledger_api::{app, default_db_path};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let db_path = std::env::var("LEDGER_DB")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| default_db_path());
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".into());
    let addr = format!("127.0.0.1:{port}");

    tracing::info!("database {}", db_path.display());
    tracing::info!("listening on http://{addr}");

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app(&db_path)?)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}
