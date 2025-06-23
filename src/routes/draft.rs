use axum::{
    extract::{Extension, Path, Json},
    http::StatusCode,
    response::IntoResponse,
};
use sqlx::{Pool, SqlitePool, types::Json as SqlxJson, Sqlite};
use tracing::{info, error};
use rand::seq::SliceRandom;
use rand::rng;
use tokio::sync::RwLock;
use std::sync::Arc;
use tokio::sync::broadcast;

use crate::{dto::{draft_dto::{DraftState, SharedDraftState}, player_dto::Player, team_dto::Team}};
use crate::services::{auth_user::AuthUser, websocket::{send_draft_update, send_player_update}};

pub async fn start_draft (
    Extension(state): Extension<SharedDraftState>,
    Extension(tx): Extension<broadcast::Sender<String>>,
    Extension(pool): Extension<SqlitePool>,
    AuthUser(claims): AuthUser
) -> impl IntoResponse {
    info!("Starting tournament.");
    if claims.sub != "admin" {
        return (StatusCode::UNAUTHORIZED, format!("You must be an admin to start the tournament."))
    }

    let mut teams: Vec<Team> = match sqlx::query_as::<_, Team>("SELECT * FROM teams")
        .fetch_all(&pool)
        .await {
            Ok(result) => result,
            Err(e) => {
                error!("Failed to fetch teams: {:?}", e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to load teams from database".to_string()
                )
            }
        };

    teams.shuffle(&mut rng());

    let mut players: Vec<Player> = match sqlx::query_as::<_, Player>("SELECT * FROM players WHERE drafted = true")
        .fetch_all(&pool)
        .await {
            Ok(result) => result,
            Err(e) => {
                error!("Failed to fetch teams: {:?}", e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to load teams from database".to_string()
                )
            }
        };

    {
        let mut guard = state.write().await;
        info!("Fetched write lock.");
        guard.phase = "Drafting".into();
        guard.drafted_players = SqlxJson(players);
        guard.teams = SqlxJson(teams);
        
        info!("state has been updated with tournament set up.");

        // Serialize JSON fields
        let teams_json = match serde_json::to_string(&guard.teams.0) {
            Ok(json) => json,
            Err(e) => {
                error!("Failed to serialize teams: {:?}", e);
                return (StatusCode::INTERNAL_SERVER_ERROR, "Serialization error".to_string())
            }
        };

        let drafted_players_json = match serde_json::to_string(&guard.drafted_players.0) {
            Ok(json) => json,
            Err(e) => {
                error!("Failed to serialize drafted players: {:?}", e);
                return (StatusCode::INTERNAL_SERVER_ERROR, "Serialization error".to_string())
            }
        };

        // Save draft state
        let result = sqlx::query!(
            r#"
            INSERT INTO draft_state (id, phase, teams, current_turn, drafted_players)
            VALUES (1, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                phase = excluded.phase,
                teams = excluded.teams,
                drafted_players = excluded.drafted_players
            "#,
            guard.phase,
            teams_json,
            0,
            drafted_players_json
        )
        .execute(&pool)
        .await;

        if let Err(e) = result {
            error!("Failed to save draft state: {:?}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to save draft state".to_string());
        }
    }

    info!("Saved draft to db.");

    send_draft_update(&tx, &state).await;
    (StatusCode::OK, format!("Started the tournament!"))
}

pub async fn stop_draft (
    Extension(pool): Extension<SqlitePool>,
    Extension(state): Extension<SharedDraftState>,
    Extension(tx): Extension<broadcast::Sender<String>>,
    AuthUser(claims): AuthUser
) -> impl IntoResponse {
    info!("Stopping tournament.");

    if claims.sub != "admin" {
        return (
            StatusCode::UNAUTHORIZED,
            "You must be an admin to stop the tournament.".to_string()
        );
    }

    {
        let mut guard = state.write().await;
        info!("Fetched write lock.");
        *guard = DraftState::default();
    }

    let delete_draft_result = sqlx::query!("DELETE FROM draft_state WHERE id = 1")
        .execute(&pool)
        .await;

    if let Err(e) = delete_draft_result {
        error!("Failed to delete draft: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to delete draft: {}", e)
        );
    }

    let delete_players_result = sqlx::query!("DELETE FROM players")
        .execute(&pool)
        .await;

    if let Err(e) = delete_players_result {
        error!("Failed to delete players: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to delete players: {}", e)
        );
    }

    send_draft_update(&tx, &state).await;
    send_player_update(&pool, &tx).await;

    (
        StatusCode::OK,
        "Draft and player pool were successfully reset.".to_string()
    )
}

pub async fn get_state_internal (
    pool: &Pool<Sqlite>,
) -> SharedDraftState {
    let draft_state: SharedDraftState = {
        match sqlx::query_as::<_, DraftState>(
            r#"SELECT phase, teams, current_turn, drafted_players, direction FROM draft_state WHERE id = 1"#
        )
        .fetch_optional(pool)
        .await
        {
            Ok(Some(row)) => {
                Arc::new(RwLock::new(DraftState {
                    phase: row.phase,
                    teams: row.teams,
                    current_turn: row.current_turn,
                    drafted_players: row.drafted_players,
                    direction: row.direction
                }))
            }
            Ok(None) => {
                info!("No draft state found in DB. Initializing to default.");
                Arc::new(RwLock::new(DraftState::default()))
            }
            Err(e) => {
                error!("Failed to load draft state from DB: {:?}", e);
                Arc::new(RwLock::new(DraftState::default()))
            }
        }
    };

    draft_state
}

pub async fn draft_pick(
    Extension(state): Extension<SharedDraftState>,
    Extension(tx): Extension<broadcast::Sender<String>>,
    Extension(pool): Extension<SqlitePool>,
    AuthUser(claims): AuthUser,
    Json(payload): Json<Player>
) -> impl IntoResponse {
    info!("Drafting player {}", payload.name);
    let mut state_guard = state.write().await;

    let turn = state_guard.current_turn;
    let direction = state_guard.direction;
    let teams = &mut state_guard.teams.0;

    if turn >= teams.len() as i64 {
        return (
            StatusCode::BAD_REQUEST,
            format!("Invalid current turn: {}", turn),
        )
    }

    let current_team = &mut teams[turn as usize];

    if current_team.created_by != Some(claims.sub) {
        return (StatusCode::UNAUTHORIZED, format!("You do not have permission to pick for this team."));
    }

    // Safely parse selections JSON string
    let mut selections: Vec<Player> = serde_json::from_str(
        current_team.selections.as_deref().unwrap_or("[]")
    ).unwrap_or_default();

    if selections.len() >= 5 {
        return (
            StatusCode::BAD_REQUEST,
            format!("Team '{}' is full and cannot pick.", current_team.name),
        )
    }

    let update_result = sqlx::query!(
        r#"
        UPDATE players
        SET drafted = 1
        WHERE ign = ?
        "#,
        payload.ign
    )
    .execute(&pool)
    .await;

    if let Err(e) = update_result {
        error!("Failed to mark player as drafted: {:?}", e);
        return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to update player status".to_string());
    }

    // push the selection in selections.
    selections.push(payload);
    let selections_json = match serde_json::to_string(&selections) {
        Ok(json) => json,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to serialize selections: {}", e));
        }
    };
    current_team.selections = Some(selections_json);

    // Snake draft logic
    let mut next_turn = turn as isize + direction as isize;
    if next_turn >= teams.len() as isize {
        next_turn = teams.len() as isize - 1;
        state_guard.direction = -1;
    } else if next_turn < 0 {
        next_turn = 0;
        state_guard.direction = 1;
    }

    state_guard.current_turn = next_turn as i64;

    let teams_json = match serde_json::to_string(&state_guard.teams.0) {
        Ok(json) => json,
        Err(e) => {
            error!("Failed to serialize teams: {:?}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Serialization error".to_string())
        }
    };

    let drafted_players_json = match serde_json::to_string(&state_guard.drafted_players.0) {
        Ok(json) => json,
        Err(e) => {
            error!("Failed to serialize drafted players: {:?}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Serialization error".to_string())
        }
    };

    // Save draft state
    let result = sqlx::query!(
        r#"
        INSERT INTO draft_state (id, phase, teams, current_turn, drafted_players, direction)
        VALUES (1, ?, ?, ?, ?, ?)
        ON CONFLICT(id) DO UPDATE SET
            phase = excluded.phase,
            teams = excluded.teams,
            current_turn = excluded.current_turn,
            drafted_players = excluded.drafted_players,
            direction = excluded.direction
        "#,
        state_guard.phase,
        teams_json,
        state_guard.current_turn,
        drafted_players_json,
        state_guard.direction
    )
    .execute(&pool)
    .await;

    if let Err(e) = result {
        error!("Failed to save draft state: {:?}", e);
        return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to save draft state".to_string());
    }
    
    drop(state_guard);
    send_draft_update(&tx, &state).await;
    send_player_update(&pool, &tx).await;

    (StatusCode::OK, format!("Successfully pushed selection to team."))
}

pub async fn get_state(
    Extension(state): Extension<SharedDraftState>,
) -> impl IntoResponse {
    let state_guard = state.read().await;
    let cloned_state = state_guard.clone();

    (StatusCode::OK, Json(cloned_state)).into_response()
}