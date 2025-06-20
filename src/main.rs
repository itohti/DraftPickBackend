use axum::{
    extract::{Extension}, http::{HeaderValue, Method}, routing::{get, post, delete}, Router
};
use tower_http::cors::{CorsLayer};
use sqlx::{sqlite::SqlitePoolOptions, types::Json};
use tracing::{info, error};
use tokio::sync::{broadcast};
use tokio::sync::RwLock;
use std::sync::Arc;

mod dto;
mod services;
mod routes;

use dto::draft_dto::{DraftState, SharedDraftState};

use routes::teams::{get_teams, create_teams, delete_teams};
use routes::users::{create_user, login_user, remove_user};
use routes::draft::{start_draft, get_state_internal, draft_pick, get_state, stop_draft};
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

    let draft_state = get_state_internal(&pool).await;
    
    let app = Router::new()
        .route("/ws", get(services::websocket::websocket_handler))
        .route("/teams", get(get_teams))
        .route("/teams", post(create_teams))
        .route("/teams/{team_id}", delete(delete_teams))
        .route("/players", get(get_players))
        .route("/login", post(login_user))
        .route("/users", post(create_user))
        .route("/users", delete(remove_user))
        .route("/start_draft", post(start_draft))
        .route("/draft/pick", post(draft_pick))
        .route("/stop_draft", post(stop_draft))
        .route("/draft", get(get_state))
        .layer(Extension(pool))
        .layer(Extension(tx))
        .layer(Extension(draft_state))
        .layer(cors);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    info!("Started server.");
    axum::serve(listener, app).await.unwrap();
}