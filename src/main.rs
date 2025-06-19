use axum::{
    extract::{Extension}, http::{HeaderValue, Method}, routing::{get, post, delete}, Router
};
use tower_http::cors::{CorsLayer};
use sqlx::{sqlite::SqlitePoolOptions};
use tracing::{info, error};
use tokio::sync::{broadcast};

mod dto;
mod services;
mod routes;

use routes::teams::{get_teams, create_teams, delete_teams};
use routes::users::{create_user, login_user, remove_user};
use routes::players::get_players;


#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let allowed_origins = [
        "http://localhost:3001",
        "https://valorant-draft-pick.vercel.app",
    ];

    let cors = CorsLayer::new()
        .allow_origin(
            allowed_origins
                .iter()
                .map(|origin| origin.parse::<HeaderValue>().unwrap())
                .collect::<Vec<_>>(),
        )
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers(tower_http::cors::Any);

    let db_url = "sqlite://./data/sunny.db";
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(db_url)
        .await
        .expect("Could not connect to SQLite");

    info!("Connected to sqlite database.");

    let (tx, _) = broadcast::channel::<String>(32);
    
    let app = Router::new()
        .route("/ws", get(services::websocket::websocket_handler))
        .route("/teams", get(get_teams))
        .route("/teams", post(create_teams))
        .route("/teams/{team_id}", delete(delete_teams))
        .route("/players", get(get_players))
        .route("/login", post(login_user))
        .route("/users", post(create_user))
        .route("/users", delete(remove_user))
        .layer(Extension(pool))
        .layer(cors)
        .layer(Extension(tx));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    info!("Started server.");
    axum::serve(listener, app).await.unwrap();
}