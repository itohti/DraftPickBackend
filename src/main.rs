use std::{fs::File};
use axum::{
    extract::{Extension, Path}, http::StatusCode, response::IntoResponse, routing::{get, post, delete}, Json, Router
};
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use serde::{Deserialize, Serialize};
use serde_json::{Value};
use tracing::{info, error};

use yup_oauth2::{ServiceAccountAuthenticator, read_service_account_key};
use reqwest::Client;

pub mod dto {
    pub mod team_dto;
    pub mod request_team_dto;
    pub mod player_dto;
}

pub mod services {
    pub mod draft_player_formatter;
}

use dto::team_dto::Team;
use dto::request_team_dto::CreateTeam;
use dto::player_dto::PlayerCard;
use services::draft_player_formatter;

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
        .route("/teams", post(create_teams))
        .route("/teams/{team_id}", delete(delete_teams))
        .route("/players", get(get_players))
        .layer(Extension(pool));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    info!("Started server.");
    axum::serve(listener, app).await.unwrap();
}

/**
 * GET request to get all the teams.
 */
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

/**
 * POST request to create a new team.
 */
async fn create_teams(Extension(pool): Extension<SqlitePool>, Json(payload): Json<CreateTeam>,) -> impl IntoResponse {
    info!("Creating a team {}", payload.name);

    let selections_json = match serde_json::to_string(&payload.selections) {
        Ok(json) => json,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("Could not read selection input correctly {}", e))
        }
    };

    let create_result = sqlx::query!(
        r#"
        INSERT INTO teams (name, selections, team_size, team_money, is_picking)
        VALUES (?, ?, ?, ?, ?)
        "#,
        payload.name,
        selections_json,
        0,
        0,
        false
    )
    .execute(&pool)
    .await;

    match create_result {
        Ok(result) => {
            (StatusCode::OK, format!("Successfully created the team!"))
        }
        Err(e) => {
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Could not create the team {}", payload.name))
        }
    }
}

/**
 * DELETE request to delete a team by their name.
 */
async fn delete_teams(Extension(pool): Extension<SqlitePool>, Path(team_id): Path<i64>) -> impl IntoResponse {
    info!("Deleting the team {}", team_id);

    let delete_result = sqlx::query!(
        "DELETE FROM teams WHERE id = ?", team_id
    )
    .execute(&pool)
    .await;

    match delete_result {
        Ok(res) => {
            if res.rows_affected() == 0 {
                (StatusCode::NOT_FOUND, format!("Team was not found."))
            }
            else {
                (StatusCode::OK, format!("Team was successfully removed."))
            }
        }
        Err(e) => {
            error!("Failed to delete team: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to delete team: {}", e))
        }
    }
    
}

/**
 * GET the players that signed up for the tournament.
 */
async fn get_players() -> impl IntoResponse {
    let service_account_key = read_service_account_key("credentials.json").await.expect("Could not read google credentials");
    
    let auth = ServiceAccountAuthenticator::builder(service_account_key)
        .build()
        .await
        .expect("Failed to create auth");

    let scope = &["https://www.googleapis.com/auth/spreadsheets.readonly"];
    let token = auth.token(scope).await.expect("Google api error.");

    let sheet_id = "1_57KqAux4swU4QAdQXeEd--eDDSFZzF_FXVosagzAQU";
    let range = "Form Responses 1";

    let url = format!(
        "https://sheets.googleapis.com/v4/spreadsheets/{}/values/{}",
        sheet_id, range
    );

    let client = Client::new();

    let response: Value = client
        .get(&url)
        .bearer_auth(token.token().expect("No token string"))
        .send()
        .await
        .expect("Could not send request to Google Sheets API")
        .json()
        .await
        .expect("Could not convert response to json");

    let values = response.get("values")
        .and_then(|v| v.as_array())
        .ok_or("No values array found in response").expect("Could not fetch data.");

    let players = draft_player_formatter::format_responses(values);

    (StatusCode::OK, Json(players))
}