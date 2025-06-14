use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateTeam {
    pub name: String,
    pub selections: Vec<String>
}