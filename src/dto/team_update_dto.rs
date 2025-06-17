use serde::{Serialize};
use crate::dto::team_dto::Team;
#[derive(Serialize)]
pub struct TeamsUpdate {
    pub r#type: String,
    pub teams: Vec<Team>,
}