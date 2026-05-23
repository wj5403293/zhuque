use crate::api::AppState;
use crate::models::{CreateDependence, DependenceType, UpdateDependence};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use std::sync::Arc;

#[derive(Deserialize)]
pub struct ListQuery {
    #[serde(rename = "type")]
    dep_type: Option<String>,
}

/// 获取依赖列表
pub async fn list_dependences(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ListQuery>,
) -> Result<impl IntoResponse, StatusCode> {
    let dep_type = query.dep_type.and_then(|t| match t.as_str() {
        "nodejs" => Some(DependenceType::NodeJS),
        "python" => Some(DependenceType::Python),
        "linux" => Some(DependenceType::Linux),
        _ => None,
    });

    let deps = state
        .dependence_service
        .list(dep_type)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(deps))
}

/// 获取单个依赖
pub async fn get_dependence(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, StatusCode> {
    let dep = state
        .dependence_service
        .get(id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(dep))
}

/// 创建依赖
pub async fn create_dependence(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateDependence>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let dep = state
        .dependence_service
        .create(payload)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() }))
            )
        })?;

    Ok((StatusCode::CREATED, Json(dep)))
}

/// 批量创建依赖
pub async fn create_dependences_batch(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<Vec<CreateDependence>>,
) -> Result<impl IntoResponse, StatusCode> {
    let deps = state
        .dependence_service
        .create_batch(payload)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok((StatusCode::CREATED, Json(deps)))
}

/// 更新依赖
pub async fn update_dependence(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Json(payload): Json<UpdateDependence>,
) -> Result<impl IntoResponse, StatusCode> {
    let dep = state
        .dependence_service
        .update(id, payload)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(dep))
}

/// 删除依赖
pub async fn delete_dependence(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, StatusCode> {
    let deleted = state
        .dependence_service
        .delete(id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if !deleted {
        return Err(StatusCode::NOT_FOUND);
    }

    Ok(StatusCode::NO_CONTENT)
}

/// 软删除依赖（只删除数据库记录，不卸载系统依赖）
pub async fn soft_delete_dependence(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, StatusCode> {
    let deleted = state
        .dependence_service
        .soft_delete(id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if !deleted {
        return Err(StatusCode::NOT_FOUND);
    }

    Ok(StatusCode::NO_CONTENT)
}

/// 重新安装依赖
pub async fn reinstall_dependence(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, StatusCode> {
    state
        .dependence_service
        .reinstall(id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::ACCEPTED)
}
