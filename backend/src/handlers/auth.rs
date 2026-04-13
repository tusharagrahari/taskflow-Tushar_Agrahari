use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use jsonwebtoken::{EncodingKey, Header, encode};
use std::collections::HashMap;
use uuid::Uuid;

use crate::{
    error::{AppError, Result},
    middleware::auth::Claims,
    models::user::{LoginRequest, LoginResponse, RegisterRequest, User, UserResponse},
    state::AppState,
};

fn make_token(user_id: Uuid, email: &str, secret: &str) -> Result<String> {
    let exp = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::hours(24))
        .ok_or_else(|| AppError::Internal("time overflow".to_string()))?
        .timestamp() as usize;

    let claims = Claims {
        sub: user_id.to_string(),
        email: email.to_string(),
        exp,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| AppError::Internal(e.to_string()))
}

pub async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>,
) -> Result<impl IntoResponse> {
    // Step 1: Validate
    let mut fields = HashMap::new();

    if payload.name.trim().is_empty() {
        fields.insert("name".to_string(), "is required".to_string());
    }
    if payload.email.trim().is_empty() {
        fields.insert("email".to_string(), "is required".to_string());
    } else if !payload.email.contains('@') {
        fields.insert("email".to_string(), "must be a valid email".to_string());
    }
    if payload.password.len() < 8 {
        fields.insert(
            "password".to_string(),
            "must be at least 8 characters".to_string(),
        );
    }
    if !fields.is_empty() {
        return Err(AppError::Validation(fields));
    }

    // Step 2: Hash password
    let password = payload.password.clone();
    let password_hash = tokio::task::spawn_blocking(move || bcrypt::hash(password, 12))
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .map_err(|e| AppError::Internal(e.to_string()))?;

    // Step 3: Insert user
    let user = sqlx::query_as!(
        User,
        "INSERT INTO users (name, email, password_hash) VALUES ($1, $2, $3) RETURNING *",
        payload.name.trim(),
        payload.email.trim().to_lowercase(),
        password_hash
    )
    .fetch_one(&state.pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::Database(ref db_err) if db_err.constraint().is_some() => {
            AppError::Conflict("email already exists".to_string())
        }
        _ => AppError::from(e),
    })?;

    let token = make_token(user.id, &user.email, &state.jwt_secret)?;

    Ok((
        StatusCode::CREATED,
        Json(LoginResponse {
            token,
            user: UserResponse::from(user),
        }),
    ))
}

pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> Result<impl IntoResponse> {
    // Step 1: Validate
    let mut fields = HashMap::new();

    if payload.email.trim().is_empty() {
        fields.insert("email".to_string(), "is required".to_string());
    }
    if payload.password.is_empty() {
        fields.insert("password".to_string(), "is required".to_string());
    }
    if !fields.is_empty() {
        return Err(AppError::Validation(fields));
    }

    // Step 2: Fetch user
    let user = sqlx::query_as!(
        User,
        "SELECT * FROM users WHERE email = $1",
        payload.email.trim().to_lowercase()
    )
    .fetch_one(&state.pool)
    .await
    .map_err(|_| AppError::Unauthorized)?;

    // Step 3: Verify password
    let password = payload.password.clone();
    let hash = user.password_hash.clone();
    let valid = tokio::task::spawn_blocking(move || bcrypt::verify(password, &hash))
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .map_err(|e| AppError::Internal(e.to_string()))?;

    if !valid {
        return Err(AppError::Unauthorized);
    }

    // Step 4: Generate JWT
    let token = make_token(user.id, &user.email, &state.jwt_secret)?;

    Ok(Json(LoginResponse {
        token,
        user: UserResponse::from(user),
    }))
}
