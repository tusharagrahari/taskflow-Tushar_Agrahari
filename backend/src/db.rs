use sqlx::postgres::PgPoolOptions;

pub fn create_pool(database_url: &str) -> sqlx::PgPool {
    PgPoolOptions::new()
        .max_connections(5)
        .connect_lazy(database_url)
        .expect("Invalid DATABASE_URL")
}
