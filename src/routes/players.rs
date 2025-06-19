use axum::{
    http::StatusCode,
    response::IntoResponse,
    Json,
};

use yup_oauth2::{ServiceAccountAuthenticator, read_service_account_key};
use reqwest::Client;
use serde_json::{Value};

use crate::services::draft_player_formatter;
/**
 * GET the players that signed up for the tournament.
 */
pub async fn get_players() -> impl IntoResponse {
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
