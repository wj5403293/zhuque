use crate::api::AppState;
use crate::models::config::NotificationConfig;
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use std::sync::Arc;

pub async fn get_config(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let config = state
        .notification_service
        .get_config()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(config))
}

pub async fn update_config(
    State(state): State<Arc<AppState>>,
    Json(config): Json<NotificationConfig>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    state
        .notification_service
        .save_config(&config)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(config))
}

#[derive(Debug, Deserialize)]
pub struct TestChannelRequest {
    pub channel_type: String,
    pub config: serde_json::Value,
}

pub async fn test_channel(
    State(state): State<Arc<AppState>>,
    Json(req): Json<TestChannelRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    state
        .notification_service
        .test_channel(&req.channel_type, req.config)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    Ok(Json(serde_json::json!({
        "success": true,
        "message": "测试通知发送成功"
    })))
}

#[derive(Debug, Deserialize)]
pub struct ScriptNotifyRequest {
    pub title: String,
    pub content: String,
}

/// 供脚本内部调用的通知端点（公开路由，通过 Bearer token 校验）
pub async fn script_notify(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<ScriptNotifyRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let token = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or((
            StatusCode::UNAUTHORIZED,
            "Missing Authorization header".to_string(),
        ))?;

    state
        .notification_service
        .script_notify(token, &req.title, &req.content)
        .await
        .map_err(|e| (StatusCode::UNAUTHORIZED, e.to_string()))?;

    Ok(Json(serde_json::json!({ "ok": true })))
}
