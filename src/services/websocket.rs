
use axum::{
    extract::{Extension, ws::{WebSocket, WebSocketUpgrade, Message}},
    response::IntoResponse,
};
use sqlx::{SqlitePool};
use tokio::sync::broadcast;
use tracing::{info, error};
use crate::dto::team_dto::{Team, TeamsUpdate};
use futures_util::{StreamExt, SinkExt};

pub async fn send_update(pool: &SqlitePool, tx: &broadcast::Sender<String>) {
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