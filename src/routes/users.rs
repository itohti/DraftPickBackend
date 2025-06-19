use axum::{
    extract::{Extension},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use sqlx::{SqlitePool};
use tracing::{error};
use jsonwebtoken::{encode, EncodingKey, Header};
use chrono::{Utc};

use crate::dto::user_dto::{User, CreateUser, LoginUser};
use crate::dto::claims_dto::Claims;

use crate::services::auth_user::AuthUser;

pub async fn create_user(
    Extension(pool): Extension<SqlitePool>,
    Json(payload): Json<CreateUser>
) -> impl IntoResponse {
    /* First check if the user with that user name already exists */
    let user_result = sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = ?")
        .bind(&payload.username)
        .fetch_all(&pool)
        .await;

    match user_result {
        Ok(result) => {
            if result.len() > 0 {
                return (StatusCode::CONFLICT, format!("That username already exists"));
            }
            else {
                /* Now insert user inside database */
                let create_result = sqlx::query!(
                    r#"
                    INSERT INTO users (team_id, name, username, ign, password)
                    VALUES (?, ?, ?, ?, ?)
                    "#,
                    None::<i64>,
                    payload.name,
                    payload.username,
                    payload.ign,
                    payload.password
                )
                .execute(&pool)
                .await;

                match create_result {
                    Ok(result) => {
                        return (StatusCode::OK, format!("Successfully created user \"{}\"", payload.username));
                    }
                    Err(e) => {
                        return (StatusCode::INTERNAL_SERVER_ERROR, format!("Could not create user in database: {}", e));
                    }
                }
            }
        }
        Err(e) => {
            error!("There was an error with the database {:?}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("There was a database issue."))
        }
    }
}

pub async fn remove_user(
    AuthUser(claims): AuthUser,
    Extension(pool): Extension<SqlitePool>,
) -> impl IntoResponse {
    let username = claims.sub;

    let remove_result = sqlx::query!(
        r#"
        DELETE FROM users WHERE username = ?
        "#,
        username
    )
    .execute(&pool)
    .await;

    match remove_result {
        Ok(result) => {
            return (StatusCode::OK, format!("Successfully removed {}", username));
        }
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("Could not remove {} for this reason {}.", username, e));
        }
    }
}

/* POST to login the user */
pub async fn login_user(
    Extension(pool): Extension<SqlitePool>,
    Json(payload): Json<LoginUser>,
) -> impl IntoResponse {
    let user_result = sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = ?")
        .bind(&payload.username)
        .fetch_all(&pool)
        .await;

    match user_result {
        Ok(result) => {
            if result.len() > 0 {
                /* Should only be one account */
                let user = &result[0];
                if payload.password == user.password {
                    let claims = Claims {
                        sub: user.username.clone(),
                        exp: (Utc::now() + chrono::Duration::hours(24)).timestamp() as usize
                    };

                    let token = encode(
                        &Header::default(),
                        &claims,
                        &EncodingKey::from_secret("sunnycup".as_ref())
                    ).expect("Token encoding failed");


                    return (StatusCode::OK, Json(token));
                }
                else {
                    return (StatusCode::UNAUTHORIZED, Json("Incorrect username or password.".to_string()));
                }
            }
            else {
                return (StatusCode::NOT_FOUND, Json("User was not found.".to_string()));
            }
        }
        Err(e) => {
            error!("There was an error with the database {:?}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, Json("There was a database issue.".to_string()))
        }
    }

}