use axum::{
    extract::{Extension, Path},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use sqlx::{SqlitePool};
use tokio::sync::broadcast;
use tracing::{info, error, warn};
use crate::{dto::{player_dto::Player, team_dto::{CreateTeam, Team}}, services::websocket::send_player_update};
use crate::services::websocket::{send_team_update};
use crate::services::auth_user::AuthUser;
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
    AuthUser(claims): AuthUser,
    Json(payload): Json<CreateTeam>,
) -> impl IntoResponse {
    info!("Creating a team {}", payload.name);

    let selections_json = match serde_json::to_string(&payload.selections) {
        Ok(json) => json,
        Err(e) => {
            error!("Failed to serialize selections: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Could not read selection input correctly: {}", e),
            );
        }
    };

    // Insert the team initially
    if let Err(e) = sqlx::query!(
        r#"
        INSERT INTO teams (name, selections, team_size, team_money, is_picking, created_by)
        VALUES (?, ?, ?, ?, ?, ?)
        "#,
        payload.name,
        selections_json,
        0,
        0,
        false,
        claims.sub
    )
    .execute(&pool)
    .await
    {
        error!("Failed to create team: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Could not create the team {}", payload.name),
        );
    }

    let username = claims.sub;
    let like_pattern = format!("{}%", username);

    // Try to find a matching player
    match sqlx::query_as!(
        Player,
        r#"SELECT * FROM players WHERE ign LIKE ? LIMIT 1"#,
        like_pattern
    )
    .fetch_optional(&pool)
    .await
    {
        Ok(Some(player)) => {
            // Fetch the team just created
            match sqlx::query_as!(
                Team,
                r#"SELECT * FROM teams WHERE created_by = ? ORDER BY id DESC LIMIT 1"#,
                username
            )
            .fetch_one(&pool)
            .await
            {
                Ok(team) => {
                    let mut selections: Vec<Player> = match team.selections {
                        Some(s) => serde_json::from_str(&s).unwrap_or_else(|_| vec![]),
                        None => vec![],
                    };

                    let mut updated_player = player.clone();
                    updated_player.drafted = true;
                    selections.push(updated_player);

                    let updated_selections_json = match serde_json::to_string(&selections) {
                        Ok(json) => json,
                        Err(e) => {
                            error!("Failed to re-serialize selections: {}", e);
                            return (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                format!("Failed to serialize selections: {}", e),
                            );
                        }
                    };

                    // Update the team's selections
                    if let Err(e) = sqlx::query!(
                        r#"
                        UPDATE teams
                        SET selections = ?
                        WHERE id = ?
                        "#,
                        updated_selections_json,
                        team.id
                    )
                    .execute(&pool)
                    .await
                    {
                        error!("Failed to update team with new selection: {}", e);
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            "Failed to update team with new selection".to_string(),
                        );
                    }

                    let _ = sqlx::query!(
                        r#"UPDATE players SET drafted = 1 WHERE ign = ?"#,
                        player.ign
                    )
                    .execute(&pool)
                    .await;
                }
                Err(e) => {
                    error!("Could not fetch created team: {}", e);
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Could not fetch the created team".to_string(),
                    );
                }
            }
        }
        Ok(None) => {
            warn!("No matching player found for '{}'", username);
        }
        Err(e) => {
            error!("Error fetching player: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to find matching player".to_string(),
            );
        }
    }

    send_team_update(&pool, &tx).await;
    send_player_update(&pool, &tx).await;
    (
        StatusCode::OK,
        format!("Successfully created the team!"),
    )
}

/**
 * DELETE request to delete a team by their name.
 */
pub async fn delete_teams(
    Extension(pool): Extension<SqlitePool>,
    Extension(tx): Extension<broadcast::Sender<String>>,
    AuthUser(claims): AuthUser,
    Path(team_id): Path<i64>
) -> impl IntoResponse {
    info!("Deleting the team {}", team_id);

    let delete_result = sqlx::query!(
        "DELETE FROM teams WHERE id = ? AND created_by = ?", team_id, claims.sub
    )
    .execute(&pool)
    .await;

    match delete_result {
        Ok(res) => {
            if res.rows_affected() == 0 {
                (StatusCode::NOT_FOUND, format!("Team was not found."))
            }
            else {
                send_team_update(&pool, &tx).await;
                (StatusCode::OK, format!("Team was successfully removed."))
            }
        }
        Err(e) => {
            error!("Failed to delete team: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to delete team: {}", e))
        }
    }
    
}