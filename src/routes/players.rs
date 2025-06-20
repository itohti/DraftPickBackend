use axum::{
    extract::{Extension},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use sqlx::{pool, SqlitePool};
use yup_oauth2::{ServiceAccountAuthenticator, read_service_account_key};
use reqwest::Client;
use serde_json::{Value};

use crate::{dto::player_dto::{PlayerCard, Player}, services::draft_player_formatter};
/**
 * GET the players that signed up for the tournament.
 */
pub async fn get_players(
    Extension(pool): Extension<SqlitePool>
) -> Result<impl IntoResponse, (StatusCode, &'static str)> {
    let service_account_key = read_service_account_key("credentials.json")
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Could not read Google credentials"))?;

    let auth = ServiceAccountAuthenticator::builder(service_account_key)
        .build()
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to create Google auth"))?;

    let token = auth.token(&["https://www.googleapis.com/auth/spreadsheets.readonly"])
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Google API token error"))?;

    let url = format!(
        "https://sheets.googleapis.com/v4/spreadsheets/{}/values/{}",
        "1_57KqAux4swU4QAdQXeEd--eDDSFZzF_FXVosagzAQU", "Form Responses 1"
    );

    let client = Client::new();
    let response: Value = client
        .get(&url)
        .bearer_auth(token.token().ok_or((StatusCode::INTERNAL_SERVER_ERROR, "Missing token string"))?)
        .send()
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to send request"))?
        .json()
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to parse JSON response"))?;

    let values = response.get("values")
        .and_then(|v| v.as_array())
        .ok_or((StatusCode::INTERNAL_SERVER_ERROR, "No values array found in response"))?;

    let players = draft_player_formatter::format_responses(values);

    save_players(&pool, &players).await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to save players to DB"))?;

    let saved_players = sqlx::query_as!(
        Player,
        r#"
        SELECT
            name,
            peak_rank,
            current_rank,
            teammate_preferences,
            roles,
            ign,
            current_rank_order,
            peak_rank_order,
            drafted
        FROM players
        ORDER BY current_rank_order DESC
        "#
    )
    .fetch_all(&pool)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to fetch players from DB"))?;

    Ok((StatusCode::OK, Json(saved_players)))
}

pub async fn save_players(
    pool: &SqlitePool,
    players: &[PlayerCard],
) -> Result<(), sqlx::Error> {
    for player in players {
        sqlx::query!(
            r#"
            INSERT INTO players (
                name, peak_rank, current_rank, teammate_preferences,
                roles, ign, current_rank_order, peak_rank_order, drafted
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(ign) DO UPDATE SET
                name = excluded.name,
                peak_rank = excluded.peak_rank,
                current_rank = excluded.current_rank,
                teammate_preferences = excluded.teammate_preferences,
                roles = excluded.roles,
                current_rank_order = excluded.current_rank_order,
                peak_rank_order = excluded.peak_rank_order
            "#,
            player.name,
            player.peak_rank,
            player.current_rank,
            player.teammate_preferences,
            player.roles,
            player.ign,
            player.current_rank_order,
            player.peak_rank_order,
            false,
        )
        .execute(pool)
        .await?;
    }

    Ok(())
}
