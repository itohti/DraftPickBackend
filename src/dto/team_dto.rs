use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct Team {
    pub id: i64,
    pub name: String,
    pub selections: Option<String>,      
    pub team_size: i64,
    pub team_money: i64,
    pub is_picking: bool,
    pub created_by: String
}

#[derive(Serialize)]
pub struct TeamsUpdate {
    pub r#type: String,
    pub teams: Vec<Team>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateTeam {
    pub name: String,
    pub selections: Vec<String>
}