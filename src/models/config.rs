use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SystemConfig {
    pub id: i64,
    pub key: String,
    pub value: String, // JSON格式
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateSystemConfig {
    pub key: String,
    pub value: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateSystemConfig {
    pub value: String,
    pub description: Option<String>,
}

// 镜像源配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirrorConfig {
    pub linux: Option<LinuxMirror>,
    pub nodejs: Option<NodejsMirror>,
    pub python: Option<PythonMirror>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinuxMirror {
    pub enabled: bool,
    pub apt_source: Option<String>, // Debian/Ubuntu
    pub yum_source: Option<String>, // CentOS/RHEL
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodejsMirror {
    pub enabled: bool,
    pub registry: Option<String>, // npm registry
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PythonMirror {
    pub enabled: bool,
    pub index_url: Option<String>, // pip index
}

// 自动备份配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoBackupConfig {
    pub enabled: bool,
    pub webdav_url: String,
    pub webdav_username: String,
    pub webdav_password: String,
    pub cron: String,
    pub remote_path: Option<String>,        // WebDAV 远程路径，默认为根目录
    pub max_backups: Option<u32>,           // 最大保留备份数量，None 表示不限制
}

impl Default for AutoBackupConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            webdav_url: String::new(),
            webdav_username: String::new(),
            webdav_password: String::new(),
            cron: "0 2 * * *".to_string(), // 默认每天凌晨2点（5字段格式）
            remote_path: None,
            max_backups: Some(10),         // 默认保留10个备份
        }
    }
}

// ─── 通知配置 ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationConfig {
    pub enabled: bool,
    pub on_success: bool,
    pub on_failure: bool,
    pub on_killed: bool,
    pub channels: Vec<ChannelConfig>,
}

impl Default for NotificationConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            on_success: false,
            on_failure: true,
            on_killed: true,
            channels: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelConfig {
    #[serde(rename = "type")]
    pub channel_type: String,
    pub enabled: bool,
    pub config: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TelegramConfig {
    pub bot_token: String,
    pub chat_id: String,
    pub proxy: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PushPlusConfig {
    pub token: String,
    pub topic: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SmtpConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub from: String,
    pub to: Vec<String>,
    pub use_tls: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResendConfig {
    pub api_key: String,
    pub from: String,
    pub to: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WeComConfig {
    pub webhook_url: String,
}