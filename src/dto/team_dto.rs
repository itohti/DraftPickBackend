use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Team {
    id: i64,
    name: String,
    selections: String,      
    team_size: i32,
    team_money: i32,
    is_picking: bool,
    created_by: String
}