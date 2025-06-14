use axum::{
    routing::{get, post},
    http::StatusCode,
    Json, Router,
};
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use serde::{Deserialize, Serialize};
use tracing::{info, error};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let db_url = "sqlite://sunny.db";
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(db_url)
        .await
        .expect("Could not connect to SQLite");

    info!("Connected to sqlite database.");

    let app = Router::new()
        .route("/teams", get(get_teams));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    info!("Started server.");
    axum::serve(listener, app).await.unwrap();
}


async fn get_teams() -> (StatusCode, String) {
    info!("Fetching teams.");
    (StatusCode::OK, "teams lol".to_string())
}