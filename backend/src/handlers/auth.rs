use crate::{
    middleware::auth::Claims,
    models::user::{LoginRequest, LoginResponse},
};
use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use jsonwebtoken::{EncodingKey, Header, encode};
use std::collections::HashMap;

use crate::{
    error::{AppError, Result},
    models::user::{RegisterRequest, User, UserResponse},
    state::AppState,
};

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
    .await?;

    Ok((StatusCode::CREATED, Json(UserResponse::from(user))))
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
    let exp = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::hours(24))
        .unwrap()
        .timestamp() as usize;

    let claims = Claims {
        sub: user.id.to_string(),
        email: user.email.clone(),
        exp,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(state.jwt_secret.as_bytes()),
    )
    .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(Json(LoginResponse {
        token,
        user: UserResponse::from(user),
    }))
}
