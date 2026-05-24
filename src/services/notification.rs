use crate::models::config::{
    BarkConfig, ChannelConfig, DingTalkConfig, FeishuConfig, NotificationConfig, NtfyConfig,
    PushPlusConfig, ResendConfig, SmtpConfig, TaskNotificationConfig, TelegramConfig, WeComConfig,
    WebhookConfig,
};
use crate::models::{CreateSystemConfig, UpdateSystemConfig};
use crate::services::ConfigService;
use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use lettre::{
    message::header::ContentType,
    transport::smtp::{
        authentication::Credentials,
        client::{Tls, TlsParameters},
    },
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info};
use uuid;
use chrono;
use urlencoding;

/// 正在运行的 execution_id -> task_id 映射，用于校验脚本通知 token
pub type NotifyTokenRegistry = Arc<RwLock<HashMap<String, i64>>>;

pub struct NotificationService {
    config_service: Arc<ConfigService>,
    http_client: reqwest::Client,
    pub token_registry: NotifyTokenRegistry,
}

impl NotificationService {
    const CONFIG_KEY: &'static str = "notification_config";

    pub fn new(
        config_service: Arc<ConfigService>,
        token_registry: NotifyTokenRegistry,
    ) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_default();

        Self {
            config_service,
            http_client,
            token_registry,
        }
    }

    // ─── 配置读写 ─────────────────────────────────────────────────────────────

    pub async fn get_config(&self) -> Result<NotificationConfig> {
        match self.config_service.get_by_key(Self::CONFIG_KEY).await? {
            Some(cfg) => Ok(serde_json::from_str(&cfg.value)?),
            None => Ok(NotificationConfig::default()),
        }
    }

    pub async fn save_config(&self, config: &NotificationConfig) -> Result<()> {
        let json_value = serde_json::to_string(config)?;
        if self
            .config_service
            .get_by_key(Self::CONFIG_KEY)
            .await?
            .is_some()
        {
            self.config_service
                .update(
                    Self::CONFIG_KEY,
                    UpdateSystemConfig {
                        value: json_value,
                        description: Some("通知配置".to_string()),
                    },
                )
                .await?;
        } else {
            self.config_service
                .create(CreateSystemConfig {
                    key: Self::CONFIG_KEY.to_string(),
                    value: json_value,
                    description: Some("通知配置".to_string()),
                })
                .await?;
        }
        Ok(())
    }

    // ─── 平台级任务通知 ───────────────────────────────────────────────────────

    pub async fn notify_task_result(
        &self,
        task_name: &str,
        status: &str,
        duration_ms: i64,
        output_tail: &str,
        task_notification: Option<TaskNotificationConfig>,
    ) {
        let config = match self.get_config().await {
            Ok(c) => c,
            Err(e) => {
                error!("Failed to get notification config: {}", e);
                return;
            }
        };

        if !config.enabled {
            return;
        }

        // 判断是否应触发通知（任务级别覆盖全局触发条件）
        let should_notify = if let Some(ref task_notif) = task_notification {
            if task_notif.enabled {
                match status {
                    "success" => task_notif.on_success.unwrap_or(config.on_success),
                    "failed"  => task_notif.on_failure.unwrap_or(config.on_failure),
                    "killed"  => task_notif.on_killed.unwrap_or(config.on_killed),
                    _ => false,
                }
            } else {
                match status {
                    "success" => config.on_success,
                    "failed"  => config.on_failure,
                    "killed"  => config.on_killed,
                    _ => false,
                }
            }
        } else {
            match status {
                "success" => config.on_success,
                "failed"  => config.on_failure,
                "killed"  => config.on_killed,
                _ => false,
            }
        };

        if !should_notify {
            return;
        }

        let (status_emoji, status_label) = match status {
            "success" => ("✅", "成功"),
            "failed"  => ("❌", "失败"),
            "killed"  => ("⚠️", "已终止"),
            _ => ("ℹ️", status),
        };

        let duration_s = duration_ms / 1000;
        let title = format!("{} 朱雀任务{}", status_emoji, status_label);

        let content = if !output_tail.is_empty() && status != "success" {
            format!(
                "任务名称：{}\n执行状态：{} {}\n执行耗时：{}s\n\n最后输出：\n{}",
                task_name, status_emoji, status_label, duration_s, output_tail
            )
        } else {
            format!(
                "任务名称：{}\n执行状态：{} {}\n执行耗时：{}s",
                task_name, status_emoji, status_label, duration_s
            )
        };

        // 确定生效的通知渠道（任务级别覆盖全局渠道列表）
        let effective_channel_ids: Option<&Vec<String>> = task_notification
            .as_ref()
            .filter(|n| n.enabled)
            .and_then(|n| n.channel_ids.as_ref())
            .filter(|ids| !ids.is_empty());

        for channel in &config.channels {
            if !channel.enabled {
                continue;
            }
            // 如果任务级别指定了渠道 ID 列表，只使用其中的渠道
            if let Some(ids) = effective_channel_ids {
                if !ids.contains(&channel.id) {
                    continue;
                }
            }
            if let Err(e) = self.send_to_channel(channel, &title, &content).await {
                error!(
                    "Notification failed [{}]: {}",
                    channel.channel_type, e
                );
            } else {
                info!(
                    "Notification sent [{}] for task '{}'",
                    channel.channel_type, task_name
                );
            }
        }
    }

    /// 登录成功后发送通知
    pub async fn notify_login(&self, username: &str, ip: &str) {
        let config = match self.get_config().await {
            Ok(c) => c,
            Err(e) => {
                error!("获取通知配置失败: {}", e);
                return;
            }
        };
        if !config.enabled || !config.on_login {
            return;
        }

        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let title = "🔐 朱雀登录通知".to_string();
        let content = format!("用户 {} 登录成功\nIP：{}\n时间：{}", username, ip, now);

        for channel in &config.channels {
            if !channel.enabled {
                continue;
            }
            if let Err(e) = self.send_to_channel(channel, &title, &content).await {
                error!("登录通知发送失败 [{}]: {}", channel.channel_type, e);
            }
        }
    }

    // ─── 脚本主动触发通知 ─────────────────────────────────────────────────────

    pub async fn script_notify(&self, token: &str, title: &str, content: &str) -> Result<()> {
        {
            let registry = self.token_registry.read().await;
            if !registry.contains_key(token) {
                return Err(anyhow!("Invalid or expired notify token"));
            }
        }

        let config = self.get_config().await?;
        if !config.enabled {
            return Ok(());
        }

        for channel in &config.channels {
            if !channel.enabled {
                continue;
            }
            if let Err(e) = self.send_to_channel(channel, title, content).await {
                error!(
                    "Script notify failed [{}]: {}",
                    channel.channel_type, e
                );
            }
        }
        Ok(())
    }

    // ─── 测试渠道 ─────────────────────────────────────────────────────────────

    pub async fn test_channel(
        &self,
        channel_type: &str,
        config_value: serde_json::Value,
    ) -> Result<()> {
        let channel = ChannelConfig {
            id: uuid::Uuid::new_v4().to_string(),
            name: "test".to_string(),
            channel_type: channel_type.to_string(),
            enabled: true,
            config: config_value,
        };
        self.send_to_channel(
            &channel,
            "🔔 朱雀测试通知",
            "如果您看到此消息，说明通知配置正确！",
        )
        .await
    }

    // ─── 渠道分发 ─────────────────────────────────────────────────────────────

    async fn send_to_channel(
        &self,
        channel: &ChannelConfig,
        title: &str,
        content: &str,
    ) -> Result<()> {
        match channel.channel_type.as_str() {
            "telegram" => {
                let cfg: TelegramConfig = serde_json::from_value(channel.config.clone())?;
                self.send_telegram(&cfg, title, content).await
            }
            "pushplus" => {
                let cfg: PushPlusConfig = serde_json::from_value(channel.config.clone())?;
                self.send_pushplus(&cfg, title, content).await
            }
            "smtp" => {
                let cfg: SmtpConfig = serde_json::from_value(channel.config.clone())?;
                self.send_smtp(&cfg, title, content).await
            }
            "resend" => {
                let cfg: ResendConfig = serde_json::from_value(channel.config.clone())?;
                self.send_resend(&cfg, title, content).await
            }
            "wecom" => {
                let cfg: WeComConfig = serde_json::from_value(channel.config.clone())?;
                self.send_wecom(&cfg, title, content).await
            }
            "webhook" => {
                let cfg: WebhookConfig = serde_json::from_value(channel.config.clone())?;
                self.send_webhook(&cfg, title, content).await
            }
            "dingtalk" => {
                let cfg: DingTalkConfig = serde_json::from_value(channel.config.clone())?;
                self.send_dingtalk(&cfg, title, content).await
            }
            "feishu" => {
                let cfg: FeishuConfig = serde_json::from_value(channel.config.clone())?;
                self.send_feishu(&cfg, title, content).await
            }
            "bark" => {
                let cfg: BarkConfig = serde_json::from_value(channel.config.clone())?;
                self.send_bark(&cfg, title, content).await
            }
            "ntfy" => {
                let cfg: NtfyConfig = serde_json::from_value(channel.config.clone())?;
                self.send_ntfy(&cfg, title, content).await
            }
            other => Err(anyhow!("Unknown channel type: {}", other)),
        }
    }

    // ─── Telegram ─────────────────────────────────────────────────────────────

    async fn send_telegram(
        &self,
        cfg: &TelegramConfig,
        title: &str,
        content: &str,
    ) -> Result<()> {
        if cfg.bot_token.is_empty() || cfg.chat_id.is_empty() {
            return Err(anyhow!("Telegram: bot_token 和 chat_id 不能为空"));
        }

        let text = format!("<b>{}</b>\n\n{}", title, html_escape(content));
        let url = format!(
            "https://api.telegram.org/bot{}/sendMessage",
            cfg.bot_token
        );

        let res = self
            .http_client
            .post(&url)
            .json(&json!({
                "chat_id": cfg.chat_id,
                "text": text,
                "parse_mode": "HTML"
            }))
            .send()
            .await?;

        if !res.status().is_success() {
            let body = res.text().await.unwrap_or_default();
            return Err(anyhow!("Telegram API 错误: {}", body));
        }
        Ok(())
    }

    // ─── PushPlus（微信）─────────────────────────────────────────────────────

    async fn send_pushplus(
        &self,
        cfg: &PushPlusConfig,
        title: &str,
        content: &str,
    ) -> Result<()> {
        if cfg.token.is_empty() {
            return Err(anyhow!("PushPlus: token 不能为空"));
        }

        let mut body = json!({
            "token": cfg.token,
            "title": title,
            "content": content,
            "template": "txt"
        });

        if let Some(topic) = &cfg.topic {
            if !topic.is_empty() {
                body["topic"] = json!(topic);
            }
        }

        let res = self
            .http_client
            .post("https://www.pushplus.plus/send")
            .json(&body)
            .send()
            .await?;

        let response: serde_json::Value = res.json().await?;
        let code = response["code"].as_i64().unwrap_or(-1);
        if code != 200 {
            return Err(anyhow!(
                "PushPlus 错误: {}",
                response["msg"].as_str().unwrap_or("unknown")
            ));
        }
        Ok(())
    }

    // ─── SMTP 邮件 ────────────────────────────────────────────────────────────

    async fn send_smtp(&self, cfg: &SmtpConfig, title: &str, content: &str) -> Result<()> {
        if cfg.host.is_empty() || cfg.username.is_empty() || cfg.from.is_empty() || cfg.to.is_empty() {
            return Err(anyhow!("SMTP: host、username、from、to 不能为空"));
        }

        let from: lettre::message::Mailbox = cfg
            .from
            .parse()
            .map_err(|_| anyhow!("SMTP: 发件人地址格式无效: {}", cfg.from))?;

        let mut builder = Message::builder()
            .from(from)
            .subject(title)
            .header(ContentType::TEXT_PLAIN);

        for addr in &cfg.to {
            let mailbox: lettre::message::Mailbox = addr
                .parse()
                .map_err(|_| anyhow!("SMTP: 收件人地址格式无效: {}", addr))?;
            builder = builder.to(mailbox);
        }

        let email = builder.body(content.to_string())?;

        let port = if cfg.port == 0 {
            if cfg.use_tls { 465 } else { 587 }
        } else {
            cfg.port
        };

        let creds = Credentials::new(cfg.username.clone(), cfg.password.clone());

        if cfg.use_tls {
            let tls = TlsParameters::new(cfg.host.clone())
                .map_err(|e| anyhow!("SMTP TLS 参数错误: {}", e))?;
            let transport = AsyncSmtpTransport::<Tokio1Executor>::relay(&cfg.host)
                .map_err(|e| anyhow!("SMTP relay 错误: {}", e))?
                .port(port)
                .tls(Tls::Wrapper(tls))
                .credentials(creds)
                .build();
            transport
                .send(email)
                .await
                .map_err(|e| anyhow!("SMTP 发送失败: {}", e))?;
        } else {
            let transport = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&cfg.host)
                .map_err(|e| anyhow!("SMTP STARTTLS 错误: {}", e))?
                .port(port)
                .credentials(creds)
                .build();
            transport
                .send(email)
                .await
                .map_err(|e| anyhow!("SMTP 发送失败: {}", e))?;
        }

        Ok(())
    }

    // ─── 企业微信机器人 ───────────────────────────────────────────────────────

    async fn send_wecom(&self, cfg: &WeComConfig, title: &str, content: &str) -> Result<()> {
        if cfg.webhook_url.is_empty() {
            return Err(anyhow!("企业微信: webhook_url 不能为空"));
        }

        let text = format!("**{}**\n{}", title, content);
        let res = self
            .http_client
            .post(&cfg.webhook_url)
            .json(&json!({
                "msgtype": "markdown",
                "markdown": { "content": text }
            }))
            .send()
            .await?;

        let response: serde_json::Value = res.json().await?;
        let errcode = response["errcode"].as_i64().unwrap_or(-1);
        if errcode != 0 {
            return Err(anyhow!(
                "企业微信错误 ({}): {}",
                errcode,
                response["errmsg"].as_str().unwrap_or("unknown")
            ));
        }
        Ok(())
    }

    // ─── Resend ───────────────────────────────────────────────────────────────

    async fn send_resend(&self, cfg: &ResendConfig, title: &str, content: &str) -> Result<()> {
        if cfg.api_key.is_empty() || cfg.from.is_empty() || cfg.to.is_empty() {
            return Err(anyhow!("Resend: api_key、from、to 不能为空"));
        }

        let res = self
            .http_client
            .post("https://api.resend.com/emails")
            .bearer_auth(&cfg.api_key)
            .json(&json!({
                "from": cfg.from,
                "to": cfg.to,
                "subject": title,
                "text": content
            }))
            .send()
            .await?;

        if !res.status().is_success() {
            let body = res.text().await.unwrap_or_default();
            return Err(anyhow!("Resend API 错误: {}", body));
        }
        Ok(())
    }

    async fn send_webhook(&self, cfg: &WebhookConfig, title: &str, content: &str) -> Result<()> {
        if cfg.url.is_empty() {
            return Err(anyhow!("Webhook: url 不能为空"));
        }

        let method = cfg.method.to_uppercase();
        let mut req = match method.as_str() {
            "GET"   => self.http_client.get(&cfg.url),
            "POST"  => self.http_client.post(&cfg.url),
            "PUT"   => self.http_client.put(&cfg.url),
            "PATCH" => self.http_client.patch(&cfg.url),
            other   => return Err(anyhow!("Webhook: 不支持的 method: {}", other)),
        };

        for (k, v) in &cfg.headers {
            req = req.header(k.as_str(), v.as_str());
        }

        if method != "GET" {
            let body = if cfg.body_template.is_empty() {
                serde_json::to_string(&json!({ "title": title, "content": content }))?
            } else {
                cfg.body_template
                    .replace("{title}", title)
                    .replace("{content}", content)
            };
            let content_type = cfg.headers.get("Content-Type")
                .map(|s| s.as_str())
                .unwrap_or("application/json");
            req = req.header("Content-Type", content_type).body(body);
        }

        let res = req.send().await?;
        if !res.status().is_success() {
            let status = res.status().as_u16();
            let body = res.text().await.unwrap_or_default();
            return Err(anyhow!("Webhook 请求失败 ({}): {}", status, &body[..body.len().min(200)]));
        }
        Ok(())
    }

    // ─── 钉钉机器人 ──────────────────────────────────────────────────────────

    async fn send_dingtalk(&self, cfg: &DingTalkConfig, title: &str, content: &str) -> Result<()> {
        if cfg.access_token.is_empty() {
            return Err(anyhow!("钉钉: access_token 不能为空"));
        }

        let timestamp = chrono::Utc::now().timestamp_millis();
        let mut url = format!(
            "https://oapi.dingtalk.com/robot/send?access_token={}",
            cfg.access_token
        );

        if let Some(secret) = &cfg.secret {
            if !secret.is_empty() {
                let string_to_sign = format!("{}\n{}", timestamp, secret);
                let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())
                    .map_err(|e| anyhow!("钉钉签名初始化失败: {}", e))?;
                mac.update(string_to_sign.as_bytes());
                let sign = BASE64.encode(mac.finalize().into_bytes());
                let sign_enc = urlencoding::encode(&sign).to_string();
                url = format!("{}&timestamp={}&sign={}", url, timestamp, sign_enc);
            }
        }

        let text = format!("{}\n{}", title, content);
        let res = self
            .http_client
            .post(&url)
            .json(&json!({
                "msgtype": "text",
                "text": { "content": text }
            }))
            .send()
            .await?;

        let response: serde_json::Value = res.json().await?;
        let errcode = response["errcode"].as_i64().unwrap_or(-1);
        if errcode != 0 {
            return Err(anyhow!(
                "钉钉错误 ({}): {}",
                errcode,
                response["errmsg"].as_str().unwrap_or("unknown")
            ));
        }
        Ok(())
    }

    // ─── 飞书机器人 ───────────────────────────────────────────────────────────

    async fn send_feishu(&self, cfg: &FeishuConfig, title: &str, content: &str) -> Result<()> {
        if cfg.webhook_url.is_empty() {
            return Err(anyhow!("飞书: webhook_url 不能为空"));
        }

        let timestamp = chrono::Utc::now().timestamp();
        let mut payload = json!({
            "msg_type": "post",
            "content": {
                "post": {
                    "zh_cn": {
                        "title": title,
                        "content": [[{ "tag": "text", "text": content }]]
                    }
                }
            }
        });

        if let Some(key) = &cfg.sign_key {
            if !key.is_empty() {
                let string_to_sign = format!("{}\n{}", timestamp, key);
                let mut mac = Hmac::<Sha256>::new_from_slice(string_to_sign.as_bytes())
                    .map_err(|e| anyhow!("飞书签名初始化失败: {}", e))?;
                mac.update(b"");
                let sign = BASE64.encode(mac.finalize().into_bytes());
                payload["timestamp"] = json!(timestamp.to_string());
                payload["sign"] = json!(sign);
            }
        }

        let res = self
            .http_client
            .post(&cfg.webhook_url)
            .json(&payload)
            .send()
            .await?;

        let response: serde_json::Value = res.json().await?;
        let code = response["code"].as_i64().unwrap_or(-1);
        if code != 0 {
            return Err(anyhow!(
                "飞书错误 ({}): {}",
                code,
                response["msg"].as_str().unwrap_or("unknown")
            ));
        }
        Ok(())
    }

    // ─── Bark ─────────────────────────────────────────────────────────────────

    async fn send_bark(&self, cfg: &BarkConfig, title: &str, content: &str) -> Result<()> {
        if cfg.server_url.is_empty() {
            return Err(anyhow!("Bark: server_url 不能为空"));
        }
        if cfg.device_key.is_empty() {
            return Err(anyhow!("Bark: device_key 不能为空"));
        }

        let url = format!("{}/push", cfg.server_url.trim_end_matches('/'));
        let mut payload = json!({
            "device_key": cfg.device_key,
            "title": title,
            "body": content
        });

        if let Some(s) = &cfg.sound  { if !s.is_empty() { payload["sound"] = json!(s); } }
        if let Some(g) = &cfg.group  { if !g.is_empty() { payload["group"] = json!(g); } }

        let res = self
            .http_client
            .post(&url)
            .json(&payload)
            .send()
            .await?;

        if !res.status().is_success() {
            let status = res.status().as_u16();
            let body = res.text().await.unwrap_or_default();
            return Err(anyhow!("Bark 错误 ({}): {}", status, &body[..body.len().min(200)]));
        }
        Ok(())
    }

    // ─── ntfy ─────────────────────────────────────────────────────────────────

    async fn send_ntfy(&self, cfg: &NtfyConfig, title: &str, content: &str) -> Result<()> {
        if cfg.server_url.is_empty() {
            return Err(anyhow!("ntfy: server_url 不能为空"));
        }
        if cfg.topic.is_empty() {
            return Err(anyhow!("ntfy: topic 不能为空"));
        }

        let url = format!("{}/{}", cfg.server_url.trim_end_matches('/'), cfg.topic);
        let mut req = self
            .http_client
            .post(&url)
            .header("Title", title)
            .header("Content-Type", "text/plain")
            .body(content.to_string());

        if let Some(token) = &cfg.token {
            if !token.is_empty() {
                req = req.header("Authorization", format!("Bearer {}", token));
            }
        }

        if cfg.priority > 0 {
            req = req.header("Priority", cfg.priority.to_string());
        }

        let res = req.send().await?;
        if !res.status().is_success() {
            let status = res.status().as_u16();
            let body = res.text().await.unwrap_or_default();
            return Err(anyhow!("ntfy 错误 ({}): {}", status, &body[..body.len().min(200)]));
        }
        Ok(())
    }
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
