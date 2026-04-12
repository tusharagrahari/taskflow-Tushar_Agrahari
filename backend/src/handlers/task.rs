use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use uuid::Uuid;

use crate::{
    error::{AppError, Result},
    middleware::auth::AuthUser,
    models::task::{CreateTaskRequest, Task, TaskFilters, TaskPriority, TaskStatus, UpdateTaskRequest},
    state::AppState,
};

pub async fn list_tasks(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(project_id): Path<Uuid>,
    Query(filters): Query<TaskFilters>,
) -> Result<impl IntoResponse> {
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
             AND ($2::task_status IS NULL OR status = $2)
             AND ($3::uuid IS NULL OR assignee_id = $3)
           ORDER BY created_at DESC"#,
        project_id,
        filters.status as Option<TaskStatus>,
        filters.assignee
    )
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(tasks))
}

pub async fn create_task(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(project_id): Path<Uuid>,
    Json(payload): Json<CreateTaskRequest>,
) -> Result<impl IntoResponse> {
    if payload.title.trim().is_empty() {
        return Err(AppError::Validation(
            std::collections::HashMap::from([("title".to_string(), "is required".to_string())]),
        ));
    }

    let task = sqlx::query_as!(
        Task,
        r#"INSERT INTO tasks (title, description, priority, assignee_id, creator_id, project_id, due_date)
           VALUES ($1, $2, $3, $4, $5, $6, $7)
           RETURNING
               id, title, description,
               status AS "status: TaskStatus",
               priority AS "priority: TaskPriority",
               project_id, assignee_id, creator_id, due_date,
               created_at, updated_at"#,
        payload.title.trim(),
        payload.description.as_deref(),
        payload.priority.unwrap_or(TaskPriority::Medium) as TaskPriority,
        payload.assignee_id,
        auth.user_id,
        project_id,
        payload.due_date
    )
    .fetch_one(&state.pool)
    .await?;

    Ok((StatusCode::CREATED, Json(task)))
}

pub async fn update_task(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateTaskRequest>,
) -> Result<impl IntoResponse> {
    let task = sqlx::query_as!(
        Task,
        r#"UPDATE tasks
           SET title       = COALESCE($1, title),
               description = COALESCE($2, description),
               status      = COALESCE($3, status),
               priority    = COALESCE($4, priority),
               assignee_id = COALESCE($5, assignee_id),
               due_date    = COALESCE($6, due_date),
               updated_at  = NOW()
           WHERE id = $7
           RETURNING
               id, title, description,
               status AS "status: TaskStatus",
               priority AS "priority: TaskPriority",
               project_id, assignee_id, creator_id, due_date,
               created_at, updated_at"#,
        payload.title.as_deref(),
        payload.description.as_deref(),
        payload.status as Option<TaskStatus>,
        payload.priority as Option<TaskPriority>,
        payload.assignee_id,
        payload.due_date,
        id
    )
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(task))
}

pub async fn delete_task(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    let row = sqlx::query!(
        r#"SELECT t.creator_id, p.owner_id AS project_owner_id
           FROM tasks t
           JOIN projects p ON t.project_id = p.id
           WHERE t.id = $1"#,
        id
    )
    .fetch_one(&state.pool)
    .await?;

    if row.creator_id != auth.user_id && row.project_owner_id != auth.user_id {
        return Err(AppError::Forbidden);
    }

    sqlx::query!("DELETE FROM tasks WHERE id = $1", id)
        .execute(&state.pool)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}
