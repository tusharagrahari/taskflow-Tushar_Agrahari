pub mod config;
pub mod db;
pub mod error;
pub mod handlers;
pub mod middleware;
pub mod models;
pub mod routes;
pub mod state;

use state::AppState;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt().with_env_filter("info").init();

    let config = config::Config::from_env();
    let pool = db::create_pool(&config.database_url);

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    tracing::info!("Migrations complete");

    let state = AppState {
        pool,
        jwt_secret: config.jwt_secret,
    };

    let app = routes::create_router(state);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", config.server_port))
        .await
        .expect("Failed to bind to port");

    tracing::info!("Server running on 0.0.0.0:{}", config.server_port);
    axum::serve(listener, app).await.unwrap();
}
