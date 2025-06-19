use axum::{
    extract::{Extension, Path},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use sqlx::{SqlitePool};
use tokio::sync::broadcast;
use tracing::{info, error};
use crate::dto::team_dto::{Team, CreateTeam};
use crate::services::websocket::{send_update};
/**
 * GET request to get all the teams.
 */
pub async fn get_teams(Extension(pool): Extension<SqlitePool>,) -> impl IntoResponse {
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
pub async fn create_teams(
    Extension(pool): Extension<SqlitePool>,
    Extension(tx): Extension<broadcast::Sender<String>>,
    Json(payload): Json<CreateTeam>,
) -> impl IntoResponse {
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
            send_update(&pool, &tx).await;
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
pub async fn delete_teams(
    Extension(pool): Extension<SqlitePool>,
    Extension(tx): Extension<broadcast::Sender<String>>,
    Path(team_id): Path<i64>
) -> impl IntoResponse {
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
                send_update(&pool, &tx).await;
                (StatusCode::OK, format!("Team was successfully removed."))
            }
        }
        Err(e) => {
            error!("Failed to delete team: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to delete team: {}", e))
        }
    }
    
}