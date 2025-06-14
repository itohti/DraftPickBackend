use axum::{
    extract::Extension, http::StatusCode, response::IntoResponse, routing::{get, post}, Json, Router
};
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use serde::{Deserialize, Serialize};
use tracing::{info, error};

mod dto {
    pub mod team_dto;
}

use dto::team_dto::Team;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let db_url = "sqlite://./data/sunny.db";
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(db_url)
        .await
        .expect("Could not connect to SQLite");

    info!("Connected to sqlite database.");

    let app = Router::new()
        .route("/teams", get(get_teams))
        .layer(Extension(pool));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    info!("Started server.");
    axum::serve(listener, app).await.unwrap();
}


async fn get_teams(Extension(pool): Extension<SqlitePool>,) -> impl IntoResponse {
    info!("Fetching teams.");

    let teams_result = sqlx::query_as::<_, Team>("SELECT * FROM teams")
        .fetch_all(&pool)
        .await;

    match teams_result {
        Ok(teams) => (StatusCode::OK, Json(teams)),
        Err(e) => {
            error!("DB query error: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(Vec::<Team>::new()))
        }
    }
}