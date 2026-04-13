use crate::{
    error::{AppError, Result},
    middleware::auth::AuthUser,
    models::{
        PaginatedResponse,
        project::{
            CreateProjectRequest, Project, ProjectFilters, ProjectWithTasks, UpdateProjectRequest,
        },
        task::{Task, TaskPriority, TaskStatus},
    },
    state::AppState,
};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use uuid::Uuid;

pub async fn list_projects(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(filters): Query<ProjectFilters>,
) -> Result<impl IntoResponse> {
    let page = filters.page.unwrap_or(1).max(1);
    let limit = filters.limit.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * limit;

    let total: i64 = sqlx::query_scalar!(
        r#"SELECT COUNT(DISTINCT p.id)
           FROM projects p
           LEFT JOIN tasks t ON t.project_id = p.id
           WHERE p.owner_id = $1 OR t.assignee_id = $1"#,
        auth.user_id
    )
    .fetch_one(&state.pool)
    .await?
    .unwrap_or(0);

    let projects = sqlx::query_as!(
        Project,
        r#"SELECT DISTINCT p.*
           FROM projects p
           LEFT JOIN tasks t ON t.project_id = p.id
           WHERE p.owner_id = $1 OR t.assignee_id = $1
           ORDER BY p.created_at DESC
           LIMIT $2 OFFSET $3"#,
        auth.user_id,
        limit,
        offset
    )
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(PaginatedResponse {
        data: projects,
        total,
        page,
        limit,
    }))
}

pub async fn create_project(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(payload): Json<CreateProjectRequest>,
) -> Result<impl IntoResponse> {
    if payload.name.trim().is_empty() {
        return Err(AppError::Validation(std::collections::HashMap::from([(
            "name".to_string(),
            "is required".to_string(),
        )])));
    }

    let project = sqlx::query_as!(
        Project,
        "INSERT INTO projects (name, description, owner_id) VALUES ($1, $2, $3) RETURNING *",
        payload.name.trim(),
        payload.description.as_deref(),
        auth.user_id
    )
    .fetch_one(&state.pool)
    .await?;

    Ok((StatusCode::CREATED, Json(project)))
}

pub async fn get_project(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    let project = sqlx::query_as!(Project, "SELECT * FROM projects WHERE id = $1", id)
        .fetch_one(&state.pool)
        .await?;

    let tasks = sqlx::query_as!(
        Task,
        r#"SELECT
               id, title, description,
               status AS "status: TaskStatus",
               priority AS "priority: TaskPriority",
               project_id, assignee_id, creator_id, due_date,
               created_at, updated_at
           FROM tasks
           WHERE project_id = $1
           ORDER BY created_at DESC"#,
        id
    )
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(ProjectWithTasks { project, tasks }))
}

pub async fn update_project(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateProjectRequest>,
) -> Result<impl IntoResponse> {
    let project = sqlx::query_as!(Project, "SELECT * FROM projects WHERE id = $1", id)
        .fetch_one(&state.pool)
        .await?;

    if project.owner_id != auth.user_id {
        return Err(AppError::Forbidden);
    }

    let updated = sqlx::query_as!(
        Project,
        r#"UPDATE projects
           SET name = COALESCE($1, name),
               description = COALESCE($2, description),
               updated_at = NOW()
           WHERE id = $3
           RETURNING *"#,
        payload.name.as_deref(),
        payload.description.as_deref(),
        id
    )
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(updated))
}

pub async fn delete_project(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    let project = sqlx::query_as!(Project, "SELECT * FROM projects WHERE id = $1", id)
        .fetch_one(&state.pool)
        .await?;
    if project.owner_id != auth.user_id {
        return Err(AppError::Forbidden);
    }

    sqlx::query!("DELETE FROM projects WHERE id = $1", id)
        .execute(&state.pool)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}
