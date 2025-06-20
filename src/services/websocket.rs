
use axum::{
    extract::{Extension, ws::{WebSocket, WebSocketUpgrade, Message}},
    response::IntoResponse,
};
use sqlx::{SqlitePool};
use tokio::sync::broadcast;
use tracing::{info, error};
use crate::dto::{draft_dto::{SharedDraftState, UpdateDraft}, team_dto::{Team, TeamsUpdate}, player_dto::{Player, PlayerUpdate}};
use futures_util::{StreamExt, SinkExt};

pub async fn send_team_update(pool: &SqlitePool, tx: &broadcast::Sender<String>) {
    let teams = sqlx::query_as::<_, Team>("SELECT * FROM teams")
        .fetch_all(pool)
        .await
        .unwrap_or_default();

    let update = TeamsUpdate {
        r#type: "teams_update".to_string(),
        teams
    };

    let update_msg = match serde_json::to_string(&update) {
        Ok(msg) => msg,
        Err(e) => {
            error!("Failed to serialize update: {}", e);
            return;
        }
    };
    let _ = tx.send(update_msg);
}

pub async fn send_draft_update(tx: &broadcast::Sender<String>, state: &SharedDraftState) {
    let state_guard = state.read().await;

    let update_msg = UpdateDraft {
        r#type: "draft_update".to_string(),
        draft_state: state_guard.clone(),
    };

    match serde_json::to_string(&update_msg) {
        Ok(json) => {
            let _ = tx.send(json);
        }
        Err(e) => {
            tracing::error!("Failed to serialize draft update message: {}", e);
        }
    }
}

pub async fn send_player_update(pool: &SqlitePool, tx: &broadcast::Sender<String>) {
    let player_result = sqlx::query_as::<_, Player>("SELECT * FROM players")
        .fetch_all(pool)
        .await
        .unwrap_or_default();

    let update_msg = PlayerUpdate {
        r#type: "player_update".to_string(),
        players: player_result
    };

    match serde_json::to_string(&update_msg) {
        Ok(json) => {
            let _ = tx.send(json);
        }
        Err(e) => {
            tracing::error!("Failed to serialize player update message: {}", e);
        }
    }
}

/* Web Socket stuff */
pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    Extension(tx): Extension<broadcast::Sender<String>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, tx))
}

async fn handle_socket(socket: WebSocket, tx: broadcast::Sender<String>) {
    let (mut sender, mut receiver) = socket.split();
    let mut rx = tx.subscribe();

    // Task to send messages to this client
    let send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if sender.send(Message::Text(msg.into())).await.is_err() {
                break;
            }
        }
    });

    // Receive messages from this client and broadcast them
    while let Some(Ok(Message::Text(msg))) = receiver.next().await {
        let _ = tx.send(msg.to_string());
    }

    // Clean up
    send_task.abort();
}