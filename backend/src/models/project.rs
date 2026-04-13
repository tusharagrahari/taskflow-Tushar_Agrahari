use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use super::task::Task;

#[derive(Debug, FromRow, Serialize)]
pub struct Project {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub owner_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateProjectRequest {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProjectRequest {
    pub name: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ProjectWithTasks {
    #[serde(flatten)]
    pub project: Project,
    pub tasks: Vec<Task>,
}

#[derive(Debug, Deserialize)]
pub struct ProjectFilters {
    pub page: Option<i64>,
    pub limit: Option<i64>,
}
