use serde::{Serialize, Deserialize};
use sqlx::FromRow;
use sqlx::types::Json;
use crate::dto::{player_dto::Player, team_dto::Team};
use tokio::sync::RwLock;
use std::sync::Arc;

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct DraftState {
    pub phase: String,
    pub teams: Json<Vec<Team>>,
    pub current_turn: i64,         
    pub drafted_players: Json<Vec<Player>>,
    pub direction: i64
}

impl Default for DraftState {
    fn default() -> Self {
        Self {
            phase: "Waiting".to_string(),
            teams: Json(vec![]),
            current_turn: 0,
            drafted_players: Json(vec![]),
            direction: 1
        }
    }
}

#[derive(Serialize)]
pub struct UpdateDraft {
    pub r#type: String,
    pub draft_state: DraftState
}

pub type SharedDraftState = Arc<RwLock<DraftState>>;
