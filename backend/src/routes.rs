use axum::{
    Router,
    routing::{delete, get, patch, post},
};

use crate::{
    handlers::{auth, project, task},
    state::AppState,
};

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .nest("/auth", auth_routes())
        .nest("/projects", project_routes())
        .nest("/tasks", task_routes())
        .with_state(state)
}

fn auth_routes() -> Router<AppState> {
    Router::new()
        .route("/register", post(auth::register))
        .route("/login", post(auth::login))
}

fn project_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/",
            get(project::list_projects).post(project::create_project),
        )
        .route(
            "/{id}",
            get(project::get_project)
                .patch(project::update_project)
                .delete(project::delete_project),
        )
        .route(
            "/{project_id}/tasks",
            get(task::list_tasks).post(task::create_task),
        )
}

fn task_routes() -> Router<AppState> {
    Router::new().route(
        "/{id}",
        patch(task::update_task).delete(task::delete_task),
    )
}
