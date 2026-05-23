use crate::models::{CreateTaskGroup, UpdateTaskGroup};
use crate::AppState;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub struct GroupTasksQuery {
    search: Option<String>,
}

pub async fn list_groups(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, StatusCode> {
    let groups = state
        .task_group_service
        .list()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(groups))
}

pub async fn get_group(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, StatusCode> {
    let group = state
        .task_group_service
        .get(id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    Ok(Json(group))
}

pub async fn create_group(
    State(state): State<Arc<AppState>>,
    Json(create): Json<CreateTaskGroup>,
) -> Result<impl IntoResponse, StatusCode> {
    let group = state
        .task_group_service
        .create(create)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(group))
}

pub async fn update_group(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Json(update): Json<UpdateTaskGroup>,
) -> Result<impl IntoResponse, StatusCode> {
    let group = state
        .task_group_service
        .update(id, update)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(group))
}

pub async fn delete_group(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, StatusCode> {
    state
        .task_group_service
        .delete(id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(json!({"success": true})))
}

pub async fn get_group_tasks(
    State(state): State<Arc<AppState>>,
    Path(group_id): Path<i64>,
    Query(query): Query<GroupTasksQuery>,
) -> Result<impl IntoResponse, StatusCode> {
    let tasks = state
        .task_service
        .list_by_group(group_id, query.search.as_deref())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(tasks))
}

pub async fn get_group_stats(
    State(state): State<Arc<AppState>>,
    Path(group_id): Path<i64>,
) -> Result<impl IntoResponse, StatusCode> {
    let count = state
        .task_group_service
        .get_tasks_count(group_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(json!({"task_count": count})))
}
