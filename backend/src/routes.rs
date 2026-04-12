use axum::{Router, routing::post};

use crate::{handlers::auth, state::AppState};

pub fn create_router(state: AppState) -> Router {
    Router::new().nest("/auth", auth_routes()).with_state(state)
}

fn auth_routes() -> Router<AppState> {
    Router::new()
        .route("/register", post(auth::register))
        .route("/login", post(auth::login))
}
