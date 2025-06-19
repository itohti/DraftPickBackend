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

#[derive(Serialize)]
pub struct TeamsUpdate {
    pub r#type: String,
    pub teams: Vec<Team>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateTeam {
    pub name: String,
    pub selections: Vec<String>,
    pub created_by: String
}