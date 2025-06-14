use serde_json::{Value};

use crate::dto::player_dto::{RawPlayer, PlayerCard};


fn rank_to_number(rank: &str) -> u8 {
    match rank.trim() {
        "Iron" => 1,
        "Bronze" => 2,
        "Silver" => 3,
        "Gold" => 4,
        "Plat" | "Platinum" => 5,
        "Diamond" => 6,
        "Ascendant" => 7,
        "Immortal" => 8,
        "Radiant" => 9,
        _ => 0,
    }
}

pub fn format_responses(rows: &[Value]) -> Vec<PlayerCard> {
    let headers = rows[0].as_array().expect("Expected headers to be an array");
    let data_rows = &rows[1..];

    let header_map: Vec<String> = headers.iter().map(|v| v.as_str().unwrap().to_string()).collect();

    let mut players: Vec<RawPlayer> = vec![];

    for row in data_rows {
        let row_vals: Vec<String> = row.as_array().unwrap().iter()
            .map(|v| v.as_str().unwrap_or("").trim().to_string())
            .collect();

        let mut map = serde_json::Map::new();
        for (key, value) in header_map.iter().zip(row_vals.iter()) {
            map.insert(key.clone(), serde_json::Value::String(value.clone()));
        }

        let player: RawPlayer = serde_json::from_value(serde_json::Value::Object(map))
            .expect("Could not convert row to RawPlayer");

        players.push(player);
    }

    let mut player_cards: Vec<PlayerCard> = players
        .into_iter()
        .map(|p| PlayerCard {
            current_rank_order: rank_to_number(&p.current_rank),
            peak_rank_order: rank_to_number(&p.peak_rank),
            name: p.name,
            peak_rank: p.peak_rank,
            current_rank: p.current_rank,
            teammate_preferences: p.teammate_preferences,
            roles: p.roles,
            ign: p.ign,
        })
        .collect();

    player_cards.sort_by_key(|p| (std::cmp::Reverse(p.current_rank_order), std::cmp::Reverse(p.peak_rank_order)));

    player_cards
}