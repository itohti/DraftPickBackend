use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RawPlayer {
    #[serde(rename = "Your Name")]
    pub name: String,

    #[serde(rename = "Your highest rank achieved on your main")]
    pub peak_rank: String,

    #[serde(rename = "Your current rank on your main")]
    pub current_rank: String,

    #[serde(rename = "Do you have any teammate preferences? While we can't guarantee you'll be placed with them, listing preferences will increase your chances.")]
    pub teammate_preferences: String,

    #[serde(rename = "Role preferences")]
    pub roles: String,

    #[serde(rename = "In game name (including #)")]
    pub ign: String,
}

#[derive(Debug, Serialize)]
pub struct PlayerCard {
    pub name: String,
    pub peak_rank: String,
    pub current_rank: String,
    pub teammate_preferences: String,
    pub roles: String,
    pub ign: String,
    pub current_rank_order: u8,
    pub peak_rank_order: u8,
}