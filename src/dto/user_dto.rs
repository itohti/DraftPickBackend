use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Deserialize)]
pub struct LoginUser {
    pub username: String,
    pub password: String
}

#[derive(Debug, Deserialize)]
pub struct CreateUser {
    pub name: String,
    pub username: String,
    pub ign: String,
    pub password: String
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: i64,
    pub team_id: i64,
    pub name: String,
    pub username: String,
    pub ign: String,
    pub password: String
}