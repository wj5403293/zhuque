use crate::api::AppState;
use crate::models::{BatchImportRequest, CreateEnvVar, UpdateEnvVar};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub struct ListEnvQuery {
    /// 关键字搜索（按 key/value/remark 模糊匹配）
    search: Option<String>,
}

/// 获取环境变量列表
pub async fn list_env_vars(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ListEnvQuery>,
) -> Result<impl IntoResponse, StatusCode> {
    let vars = state
        .env_service
        .list_with_search(query.search.as_deref())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(vars))
}

/// 获取单个环境变量
pub async fn get_env_var(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, StatusCode> {
    let var = state
        .env_service
        .get(id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(var))
}

/// 创建环境变量
pub async fn create_env_var(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateEnvVar>,
) -> Result<impl IntoResponse, StatusCode> {
    let var = state
        .env_service
        .create(payload)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok((StatusCode::CREATED, Json(var)))
}

/// 更新环境变量
pub async fn update_env_var(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Json(payload): Json<UpdateEnvVar>,
) -> Result<impl IntoResponse, StatusCode> {
    let var = state
        .env_service
        .update(id, payload)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(var))
}

/// 删除环境变量
pub async fn delete_env_var(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, StatusCode> {
    let deleted = state
        .env_service
        .delete(id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if !deleted {
        return Err(StatusCode::NOT_FOUND);
    }

    Ok(StatusCode::NO_CONTENT)
}

/// 批量导入环境变量
pub async fn batch_import_env_vars(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<BatchImportRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let result = state
        .env_service
        .batch_import(payload.vars, payload.overwrite)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
        })?;

    if !result.conflicts.is_empty() {
        return Err((
            StatusCode::CONFLICT,
            Json(serde_json::json!({
                "error": format!("以下变量已存在：{}", result.conflicts.join("、")),
                "conflicts": result.conflicts,
            })),
        ));
    }

    Ok(Json(serde_json::json!({
        "created": result.created,
        "updated": result.updated,
    })))
}
