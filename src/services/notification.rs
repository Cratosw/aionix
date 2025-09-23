// 通知服务
// 处理配额超限通知、告警通知和系统通知

use uuid::Uuid;
use chrono::{Utc, DateTime};
use serde::{Deserialize, Serialize};
use tracing::{info, error, instrument};
use utoipa::ToSchema;
use std::collections::HashMap;

use crate::errors::AiStudioError;
use crate::services::quota::QuotaUsage;
use crate::services::monitoring::{AlertEvent, AlertSeverity};

/// 通知类型
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
pub enum NotificationType {
    /// 配额警告
    QuotaWarning,
    /// 配额超限
    QuotaExceeded,
    /// 系统告警
    SystemAlert,
    /// 安全事件
    SecurityEvent,
    /// 系统维护
    SystemMaintenance,
    /// 账单提醒
    BillingReminder,
}

/// 通知渠道
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
pub enum NotificationChannel {
    /// 邮件
    Email,
    /// 短信
    Sms,
    /// Webhook
    Webhook,
    /// 站内消息
    InApp,
    /// Slack
    Slack,
    /// 钉钉
    DingTalk,
}

/// 通知优先级
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum NotificationPriority {
    /// 低
    Low,
    /// 正常
    Normal,
    /// 高
    High,
    /// 紧急
    Urgent,
}

/// 通知消息
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct NotificationMessage {
    /// 消息 ID
    pub id: Uuid,
    /// 租户 ID
    pub tenant_id: Uuid,
    /// 通知类型
    pub notification_type: NotificationType,
    /// 标题
    pub title: String,
    /// 内容
    pub content: String,
    /// 优先级
    pub priority: NotificationPriority,
    /// 目标渠道
    pub channels: Vec<NotificationChannel>,
    /// 接收者
    pub recipients: Vec<String>,
    /// 元数据
    pub metadata: HashMap<String, serde_json::Value>,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 发送时间
    pub sent_at: Option<DateTime<Utc>>,
    /// 发送状态
    pub status: NotificationStatus,
    /// 重试次数
    pub retry_count: u32,
    /// 最大重试次数
    pub max_retries: u32,
}

/// 通知状态
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum NotificationStatus {
    /// 待发送
    Pending,
    /// 发送中
    Sending,
    /// 已发送
    Sent,
    /// 发送失败
    Failed,
    /// 已取消
    Cancelled,
}

/// 通知模板
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct NotificationTemplate {
    /// 模板 ID
    pub id: Uuid,
    /// 模板名称
    pub name: String,
    /// 通知类型
    pub notification_type: NotificationType,
    /// 标题模板
    pub title_template: String,
    /// 内容模板
    pub content_template: String,
    /// 支持的渠道
    pub supported_channels: Vec<NotificationChannel>,
    /// 默认优先级
    pub default_priority: NotificationPriority,
    /// 是否启用
    pub enabled: bool,
}

/// 通知配置
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct NotificationConfig {
    /// 租户 ID
    pub tenant_id: Uuid,
    /// 通知类型配置
    pub type_configs: HashMap<NotificationType, TypeConfig>,
    /// 渠道配置
    pub channel_configs: HashMap<NotificationChannel, ChannelConfig>,
    /// 全局设置
    pub global_settings: GlobalSettings,
}

/// 类型配置
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TypeConfig {
    /// 是否启用
    pub enabled: bool,
    /// 默认渠道
    pub default_channels: Vec<NotificationChannel>,
    /// 默认接收者
    pub default_recipients: Vec<String>,
    /// 静默时间（小时）
    pub silence_hours: Option<u32>,
}

/// 渠道配置
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ChannelConfig {
    /// 是否启用
    pub enabled: bool,
    /// 配置参数
    pub settings: HashMap<String, serde_json::Value>,
    /// 重试配置
    pub retry_config: RetryConfig,
}

/// 重试配置
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RetryConfig {
    /// 最大重试次数
    pub max_retries: u32,
    /// 重试间隔（秒）
    pub retry_interval_seconds: u64,
    /// 退避策略
    pub backoff_strategy: BackoffStrategy,
}

/// 退避策略
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum BackoffStrategy {
    /// 固定间隔
    Fixed,
    /// 指数退避
    Exponential,
    /// 线性退避
    Linear,
}

/// 全局设置
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GlobalSettings {
    /// 默认语言
    pub default_language: String,
    /// 时区
    pub timezone: String,
    /// 批量发送大小
    pub batch_size: u32,
    /// 发送频率限制（每分钟）
    pub rate_limit_per_minute: u32,
}

/// 通知服务
pub struct NotificationService {
    templates: HashMap<NotificationType, NotificationTemplate>,
    configs: HashMap<Uuid, NotificationConfig>,
}

impl NotificationService {
    /// 创建新的通知服务实例
    pub fn new() -> Self {
        Self {
            templates: Self::create_default_templates(),
            configs: HashMap::new(),
        }
    }

    /// 发送配额警告通知
    #[instrument(skip(self))]
    pub async fn send_quota_warning(
        &self,
        tenant_id: Uuid,
        quota_usage: &QuotaUsage,
    ) -> Result<Uuid, AiStudioError> {
        let message = self.create_quota_warning_message(tenant_id, quota_usage)?;
        self.send_notification(message).await
    }

    /// 发送配额超限通知
    #[instrument(skip(self))]
    pub async fn send_quota_exceeded(
        &self,
        tenant_id: Uuid,
        quota_usage: &QuotaUsage,
    ) -> Result<Uuid, AiStudioError> {
        let message = self.create_quota_exceeded_message(tenant_id, quota_usage)?;
        self.send_notification(message).await
    }

    /// 发送系统告警通知
    #[instrument(skip(self))]
    pub async fn send_system_alert(
        &self,
        tenant_id: Uuid,
        alert_event: &AlertEvent,
    ) -> Result<Uuid, AiStudioError> {
        let message = self.create_system_alert_message(tenant_id, alert_event)?;
        self.send_notification(message).await
    }

    /// 发送安全事件通知
    #[instrument(skip(self))]
    pub async fn send_security_event(
        &self,
        tenant_id: Uuid,
        event_type: &str,
        details: &str,
    ) -> Result<Uuid, AiStudioError> {
        let message = self.create_security_event_message(tenant_id, event_type, details)?;
        self.send_notification(message).await
    }

    /// 发送通知
    #[instrument(skip(self))]
    pub async fn send_notification(
        &self,
        mut message: NotificationMessage,
    ) -> Result<Uuid, AiStudioError> {
        info!(
            message_id = %message.id,
            tenant_id = %message.tenant_id,
            notification_type = ?message.notification_type,
            "发送通知"
        );

        message.status = NotificationStatus::Sending;
        message.sent_at = Some(Utc::now());

        // 根据渠道发送通知
        let mut send_results = Vec::new();
        for channel in &message.channels {
            let result = self.send_to_channel(&message, channel).await;
            send_results.push(result);
        }

        // 检查发送结果
        let all_failed = send_results.iter().all(|r| r.is_err());
        let any_success = send_results.iter().any(|r| r.is_ok());

        if all_failed {
            message.status = NotificationStatus::Failed;
            error!(
                message_id = %message.id,
                "所有渠道发送失败"
            );
        } else if any_success {
            message.status = NotificationStatus::Sent;
            info!(
                message_id = %message.id,
                "通知发送成功"
            );
        }

        Ok(message.id)
    }

    /// 批量发送通知
    #[instrument(skip(self))]
    pub async fn batch_send_notifications(
        &self,
        messages: Vec<NotificationMessage>,
    ) -> Result<Vec<Result<Uuid, AiStudioError>>, AiStudioError> {
        let mut results = Vec::new();
        
        for message in messages {
            let result = self.send_notification(message).await;
            results.push(result);
        }

        Ok(results)
    }

    /// 获取通知状态
    #[instrument(skip(self))]
    pub async fn get_notification_status(
        &self,
        message_id: Uuid,
    ) -> Result<NotificationStatus, AiStudioError> {
        // 这里应该从数据库查询通知状态
        // 为了简化，返回已发送状态
        Ok(NotificationStatus::Sent)
    }

    /// 取消通知
    #[instrument(skip(self))]
    pub async fn cancel_notification(
        &self,
        message_id: Uuid,
    ) -> Result<(), AiStudioError> {
        info!(message_id = %message_id, "取消通知");
        
        // 这里应该更新数据库中的通知状态
        Ok(())
    }

    /// 重试失败的通知
    #[instrument(skip(self))]
    pub async fn retry_failed_notifications(&self) -> Result<u32, AiStudioError> {
        // 这里应该查询失败的通知并重试
        // 为了简化，返回 0
        Ok(0)
    }

    /// 设置通知配置
    #[instrument(skip(self))]
    pub async fn set_notification_config(
        &mut self,
        config: NotificationConfig,
    ) -> Result<(), AiStudioError> {
        info!(tenant_id = %config.tenant_id, "设置通知配置");
        self.configs.insert(config.tenant_id, config);
        Ok(())
    }

    /// 获取通知配置
    #[instrument(skip(self))]
    pub async fn get_notification_config(
        &self,
        tenant_id: Uuid,
    ) -> Result<Option<NotificationConfig>, AiStudioError> {
        Ok(self.configs.get(&tenant_id).cloned())
    }

    // 私有辅助方法

    /// 创建配额警告消息
    fn create_quota_warning_message(
        &self,
        tenant_id: Uuid,
        quota_usage: &QuotaUsage,
    ) -> Result<NotificationMessage, AiStudioError> {
        let template = self.templates.get(&NotificationType::QuotaWarning)
            .ok_or_else(|| AiStudioError::internal("配额警告模板不存在".to_string()))?;

        let title = template.title_template
            .replace("{quota_type}", &format!("{:?}", quota_usage.quota_type))
            .replace("{usage_percentage}", &format!("{:.1}", quota_usage.usage_percentage));

        let content = template.content_template
            .replace("{quota_type}", &format!("{:?}", quota_usage.quota_type))
            .replace("{current_usage}", &quota_usage.current_usage.to_string())
            .replace("{limit}", &quota_usage.limit.to_string())
            .replace("{usage_percentage}", &format!("{:.1}", quota_usage.usage_percentage))
            .replace("{remaining}", &quota_usage.remaining.to_string());

        let mut metadata = HashMap::new();
        metadata.insert("quota_type".to_string(), serde_json::json!(quota_usage.quota_type));
        metadata.insert("usage_percentage".to_string(), serde_json::json!(quota_usage.usage_percentage));

        Ok(NotificationMessage {
            id: Uuid::new_v4(),
            tenant_id,
            notification_type: NotificationType::QuotaWarning,
            title,
            content,
            priority: NotificationPriority::High,
            channels: template.supported_channels.clone(),
            recipients: self.get_default_recipients(tenant_id, &NotificationType::QuotaWarning),
            metadata,
            created_at: Utc::now(),
            sent_at: None,
            status: NotificationStatus::Pending,
            retry_count: 0,
            max_retries: 3,
        })
    }

    /// 创建配额超限消息
    fn create_quota_exceeded_message(
        &self,
        tenant_id: Uuid,
        quota_usage: &QuotaUsage,
    ) -> Result<NotificationMessage, AiStudioError> {
        let template = self.templates.get(&NotificationType::QuotaExceeded)
            .ok_or_else(|| AiStudioError::internal("配额超限模板不存在".to_string()))?;

        let title = template.title_template
            .replace("{quota_type}", &format!("{:?}", quota_usage.quota_type));

        let content = template.content_template
            .replace("{quota_type}", &format!("{:?}", quota_usage.quota_type))
            .replace("{current_usage}", &quota_usage.current_usage.to_string())
            .replace("{limit}", &quota_usage.limit.to_string());

        let mut metadata = HashMap::new();
        metadata.insert("quota_type".to_string(), serde_json::json!(quota_usage.quota_type));
        metadata.insert("exceeded_by".to_string(), serde_json::json!(quota_usage.current_usage - quota_usage.limit));

        Ok(NotificationMessage {
            id: Uuid::new_v4(),
            tenant_id,
            notification_type: NotificationType::QuotaExceeded,
            title,
            content,
            priority: NotificationPriority::Urgent,
            channels: template.supported_channels.clone(),
            recipients: self.get_default_recipients(tenant_id, &NotificationType::QuotaExceeded),
            metadata,
            created_at: Utc::now(),
            sent_at: None,
            status: NotificationStatus::Pending,
            retry_count: 0,
            max_retries: 5,
        })
    }

    /// 创建系统告警消息
    fn create_system_alert_message(
        &self,
        tenant_id: Uuid,
        alert_event: &AlertEvent,
    ) -> Result<NotificationMessage, AiStudioError> {
        let template = self.templates.get(&NotificationType::SystemAlert)
            .ok_or_else(|| AiStudioError::internal("系统告警模板不存在".to_string()))?;

        let priority = match alert_event.severity {
            AlertSeverity::Info => NotificationPriority::Low,
            AlertSeverity::Warning => NotificationPriority::Normal,
            AlertSeverity::Error => NotificationPriority::High,
            AlertSeverity::Critical => NotificationPriority::Urgent,
        };

        let title = template.title_template
            .replace("{severity}", &format!("{:?}", alert_event.severity));

        let content = template.content_template
            .replace("{message}", &alert_event.message)
            .replace("{current_value}", &alert_event.current_value.to_string())
            .replace("{threshold}", &alert_event.threshold.to_string());

        let mut metadata = HashMap::new();
        metadata.insert("alert_id".to_string(), serde_json::json!(alert_event.id));
        metadata.insert("rule_id".to_string(), serde_json::json!(alert_event.rule_id));
        metadata.insert("severity".to_string(), serde_json::json!(alert_event.severity));

        Ok(NotificationMessage {
            id: Uuid::new_v4(),
            tenant_id,
            notification_type: NotificationType::SystemAlert,
            title,
            content,
            priority,
            channels: template.supported_channels.clone(),
            recipients: self.get_default_recipients(tenant_id, &NotificationType::SystemAlert),
            metadata,
            created_at: Utc::now(),
            sent_at: None,
            status: NotificationStatus::Pending,
            retry_count: 0,
            max_retries: 3,
        })
    }

    /// 创建安全事件消息
    fn create_security_event_message(
        &self,
        tenant_id: Uuid,
        event_type: &str,
        details: &str,
    ) -> Result<NotificationMessage, AiStudioError> {
        let template = self.templates.get(&NotificationType::SecurityEvent)
            .ok_or_else(|| AiStudioError::internal("安全事件模板不存在".to_string()))?;

        let title = template.title_template
            .replace("{event_type}", event_type);

        let content = template.content_template
            .replace("{event_type}", event_type)
            .replace("{details}", details)
            .replace("{timestamp}", &Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string());

        let mut metadata = HashMap::new();
        metadata.insert("event_type".to_string(), serde_json::json!(event_type));
        metadata.insert("details".to_string(), serde_json::json!(details));

        Ok(NotificationMessage {
            id: Uuid::new_v4(),
            tenant_id,
            notification_type: NotificationType::SecurityEvent,
            title,
            content,
            priority: NotificationPriority::Urgent,
            channels: template.supported_channels.clone(),
            recipients: self.get_default_recipients(tenant_id, &NotificationType::SecurityEvent),
            metadata,
            created_at: Utc::now(),
            sent_at: None,
            status: NotificationStatus::Pending,
            retry_count: 0,
            max_retries: 5,
        })
    }

    /// 发送到指定渠道
    async fn send_to_channel(
        &self,
        message: &NotificationMessage,
        channel: &NotificationChannel,
    ) -> Result<(), AiStudioError> {
        match channel {
            NotificationChannel::Email => self.send_email(message).await,
            NotificationChannel::Sms => self.send_sms(message).await,
            NotificationChannel::Webhook => self.send_webhook(message).await,
            NotificationChannel::InApp => self.send_in_app(message).await,
            NotificationChannel::Slack => self.send_slack(message).await,
            NotificationChannel::DingTalk => self.send_dingtalk(message).await,
        }
    }

    /// 发送邮件
    async fn send_email(&self, message: &NotificationMessage) -> Result<(), AiStudioError> {
        info!(
            message_id = %message.id,
            recipients = ?message.recipients,
            "发送邮件通知"
        );
        
        // 这里应该实现实际的邮件发送逻辑
        Ok(())
    }

    /// 发送短信
    async fn send_sms(&self, message: &NotificationMessage) -> Result<(), AiStudioError> {
        info!(
            message_id = %message.id,
            recipients = ?message.recipients,
            "发送短信通知"
        );
        
        // 这里应该实现实际的短信发送逻辑
        Ok(())
    }

    /// 发送 Webhook
    async fn send_webhook(&self, message: &NotificationMessage) -> Result<(), AiStudioError> {
        info!(
            message_id = %message.id,
            "发送 Webhook 通知"
        );
        
        // 这里应该实现实际的 Webhook 发送逻辑
        Ok(())
    }

    /// 发送站内消息
    async fn send_in_app(&self, message: &NotificationMessage) -> Result<(), AiStudioError> {
        info!(
            message_id = %message.id,
            "发送站内消息"
        );
        
        // 这里应该实现实际的站内消息发送逻辑
        Ok(())
    }

    /// 发送 Slack 消息
    async fn send_slack(&self, message: &NotificationMessage) -> Result<(), AiStudioError> {
        info!(
            message_id = %message.id,
            "发送 Slack 通知"
        );
        
        // 这里应该实现实际的 Slack 发送逻辑
        Ok(())
    }

    /// 发送钉钉消息
    async fn send_dingtalk(&self, message: &NotificationMessage) -> Result<(), AiStudioError> {
        info!(
            message_id = %message.id,
            "发送钉钉通知"
        );
        
        // 这里应该实现实际的钉钉发送逻辑
        Ok(())
    }

    /// 获取默认接收者
    fn get_default_recipients(&self, tenant_id: Uuid, notification_type: &NotificationType) -> Vec<String> {
        // 这里应该从配置或数据库获取默认接收者
        // 为了简化，返回默认邮箱
        vec!["admin@example.com".to_string()]
    }

    /// 创建默认模板
    fn create_default_templates() -> HashMap<NotificationType, NotificationTemplate> {
        let mut templates = HashMap::new();

        // 配额警告模板
        templates.insert(
            NotificationType::QuotaWarning,
            NotificationTemplate {
                id: Uuid::new_v4(),
                name: "配额警告".to_string(),
                notification_type: NotificationType::QuotaWarning,
                title_template: "配额警告：{quota_type} 使用率已达 {usage_percentage}%".to_string(),
                content_template: "您的 {quota_type} 配额使用率已达 {usage_percentage}%（{current_usage}/{limit}），剩余 {remaining}。请及时处理以避免服务中断。".to_string(),
                supported_channels: vec![
                    NotificationChannel::Email,
                    NotificationChannel::InApp,
                    NotificationChannel::Webhook,
                ],
                default_priority: NotificationPriority::High,
                enabled: true,
            },
        );

        // 配额超限模板
        templates.insert(
            NotificationType::QuotaExceeded,
            NotificationTemplate {
                id: Uuid::new_v4(),
                name: "配额超限".to_string(),
                notification_type: NotificationType::QuotaExceeded,
                title_template: "配额超限：{quota_type} 已超出限制".to_string(),
                content_template: "您的 {quota_type} 配额已超出限制（{current_usage}/{limit}），相关服务可能受到影响。请立即联系管理员处理。".to_string(),
                supported_channels: vec![
                    NotificationChannel::Email,
                    NotificationChannel::Sms,
                    NotificationChannel::InApp,
                    NotificationChannel::Webhook,
                ],
                default_priority: NotificationPriority::Urgent,
                enabled: true,
            },
        );

        // 系统告警模板
        templates.insert(
            NotificationType::SystemAlert,
            NotificationTemplate {
                id: Uuid::new_v4(),
                name: "系统告警".to_string(),
                notification_type: NotificationType::SystemAlert,
                title_template: "系统告警：{severity} 级别".to_string(),
                content_template: "系统告警：{message}。当前值：{current_value}，阈值：{threshold}。".to_string(),
                supported_channels: vec![
                    NotificationChannel::Email,
                    NotificationChannel::InApp,
                    NotificationChannel::Webhook,
                    NotificationChannel::Slack,
                ],
                default_priority: NotificationPriority::High,
                enabled: true,
            },
        );

        // 安全事件模板
        templates.insert(
            NotificationType::SecurityEvent,
            NotificationTemplate {
                id: Uuid::new_v4(),
                name: "安全事件".to_string(),
                notification_type: NotificationType::SecurityEvent,
                title_template: "安全事件：{event_type}".to_string(),
                content_template: "检测到安全事件：{event_type}。详情：{details}。时间：{timestamp}。".to_string(),
                supported_channels: vec![
                    NotificationChannel::Email,
                    NotificationChannel::Sms,
                    NotificationChannel::InApp,
                    NotificationChannel::Webhook,
                ],
                default_priority: NotificationPriority::Urgent,
                enabled: true,
            },
        );

        templates
    }
}

impl Default for NotificationService {
    fn default() -> Self {
        Self::new()
    }
}

/// 通知服务工厂
pub struct NotificationServiceFactory;

impl NotificationServiceFactory {
    /// 创建通知服务实例
    pub fn create() -> NotificationService {
        NotificationService::new()
    }
}