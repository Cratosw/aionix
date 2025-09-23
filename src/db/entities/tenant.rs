// 租户实体定义

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// 租户状态枚举
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize, ToSchema)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "tenant_status")]
pub enum TenantStatus {
    #[sea_orm(string_value = "active")]
    Active,
    #[sea_orm(string_value = "suspended")]
    Suspended,
    #[sea_orm(string_value = "inactive")]
    Inactive,
    #[sea_orm(string_value = "pending")]
    Pending,
    #[sea_orm(string_value = "archived")]
    Archived,
}

/// 租户实体
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "tenants")]
pub struct Model {
    /// 租户 ID
    #[sea_orm(primary_key)]
    pub id: Uuid,
    
    /// 租户名称
    #[sea_orm(column_type = "String(Some(255))", unique)]
    pub name: String,
    
    /// 租户标识符（用于 URL 等）
    #[sea_orm(column_type = "String(Some(100))", unique)]
    pub slug: String,
    
    /// 租户显示名称
    #[sea_orm(column_type = "String(Some(255))")]
    pub display_name: String,
    
    /// 租户描述
    #[sea_orm(column_type = "Text", nullable)]
    pub description: Option<String>,
    
    /// 租户状态
    pub status: TenantStatus,
    
    /// 租户配置（JSON 格式）
    #[sea_orm(column_type = "Json")]
    pub config: Json,
    
    /// 配额限制（JSON 格式）
    #[sea_orm(column_type = "Json")]
    pub quota_limits: Json,
    
    /// 使用统计（JSON 格式）
    #[sea_orm(column_type = "Json")]
    pub usage_stats: Json,
    
    /// 联系邮箱
    #[sea_orm(column_type = "String(Some(255))", nullable)]
    pub contact_email: Option<String>,
    
    /// 联系电话
    #[sea_orm(column_type = "String(Some(50))", nullable)]
    pub contact_phone: Option<String>,
    
    /// 创建时间
    pub created_at: DateTimeWithTimeZone,
    
    /// 更新时间
    pub updated_at: DateTimeWithTimeZone,
    
    /// 最后活跃时间
    #[sea_orm(nullable)]
    pub last_active_at: Option<DateTimeWithTimeZone>,
}

/// 租户关联关系
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 一对多：租户 -> 用户
    #[sea_orm(has_many = "super::user::Entity")]
    Users,
}

/// 实现与用户的关联
impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Users.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

/// 租户配置结构
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TenantConfig {
    /// 时区设置
    pub timezone: String,
    /// 语言设置
    pub language: String,
    /// 主题设置
    pub theme: String,
    /// 功能开关
    pub features: TenantFeatures,
    /// 自定义设置
    pub custom_settings: serde_json::Value,
}

/// 租户功能开关
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TenantFeatures {
    /// 是否启用 AI 功能
    pub ai_enabled: bool,
    /// 是否启用知识库
    pub knowledge_base_enabled: bool,
    /// 是否启用 Agent 功能
    pub agent_enabled: bool,
    /// 是否启用 API 访问
    pub api_enabled: bool,
    /// 是否启用文件上传
    pub file_upload_enabled: bool,
}

/// 租户配额限制
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TenantQuotaLimits {
    /// 最大用户数
    pub max_users: u32,
    /// 最大知识库数
    pub max_knowledge_bases: u32,
    /// 最大文档数
    pub max_documents: u32,
    /// 最大存储空间（字节）
    pub max_storage_bytes: u64,
    /// 每月 API 调用限制
    pub monthly_api_calls: u32,
    /// 每日 AI 查询限制
    pub daily_ai_queries: u32,
}

/// 租户使用统计
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TenantUsageStats {
    /// 当前用户数
    pub current_users: u32,
    /// 当前知识库数
    pub current_knowledge_bases: u32,
    /// 当前文档数
    pub current_documents: u32,
    /// 当前存储使用量（字节）
    pub current_storage_bytes: u64,
    /// 本月 API 调用数
    pub monthly_api_calls: u32,
    /// 今日 AI 查询数
    pub daily_ai_queries: u32,
    /// 最后统计更新时间
    pub last_updated: DateTimeWithTimeZone,
}

impl Default for TenantConfig {
    fn default() -> Self {
        Self {
            timezone: "UTC".to_string(),
            language: "zh-CN".to_string(),
            theme: "default".to_string(),
            features: TenantFeatures::default(),
            custom_settings: serde_json::Value::Object(serde_json::Map::new()),
        }
    }
}

impl Default for TenantFeatures {
    fn default() -> Self {
        Self {
            ai_enabled: true,
            knowledge_base_enabled: true,
            agent_enabled: true,
            api_enabled: true,
            file_upload_enabled: true,
        }
    }
}

impl Default for TenantQuotaLimits {
    fn default() -> Self {
        Self {
            max_users: 100,
            max_knowledge_bases: 10,
            max_documents: 1000,
            max_storage_bytes: 1024 * 1024 * 1024, // 1GB
            monthly_api_calls: 10000,
            daily_ai_queries: 1000,
        }
    }
}

impl Default for TenantUsageStats {
    fn default() -> Self {
        Self {
            current_users: 0,
            current_knowledge_bases: 0,
            current_documents: 0,
            current_storage_bytes: 0,
            monthly_api_calls: 0,
            daily_ai_queries: 0,
            last_updated: chrono::Utc::now().into(),
        }
    }
}

/// 租户实用方法
impl Model {
    /// 检查租户是否活跃
    pub fn is_active(&self) -> bool {
        self.status == TenantStatus::Active
    }
    
    /// 检查租户是否被暂停
    pub fn is_suspended(&self) -> bool {
        self.status == TenantStatus::Suspended
    }
    
    /// 获取租户配置
    pub fn get_config(&self) -> Result<TenantConfig, serde_json::Error> {
        serde_json::from_value(self.config.clone())
    }
    
    /// 获取配额限制
    pub fn get_quota_limits(&self) -> Result<TenantQuotaLimits, serde_json::Error> {
        serde_json::from_value(self.quota_limits.clone())
    }
    
    /// 获取使用统计
    pub fn get_usage_stats(&self) -> Result<TenantUsageStats, serde_json::Error> {
        serde_json::from_value(self.usage_stats.clone())
    }
    
    /// 检查是否超出配额
    pub fn is_quota_exceeded(&self, quota_type: &str) -> Result<bool, serde_json::Error> {
        let limits = self.get_quota_limits()?;
        let stats = self.get_usage_stats()?;
        
        match quota_type {
            "users" => Ok(stats.current_users >= limits.max_users),
            "knowledge_bases" => Ok(stats.current_knowledge_bases >= limits.max_knowledge_bases),
            "documents" => Ok(stats.current_documents >= limits.max_documents),
            "storage" => Ok(stats.current_storage_bytes >= limits.max_storage_bytes),
            "monthly_api_calls" => Ok(stats.monthly_api_calls >= limits.monthly_api_calls),
            "daily_ai_queries" => Ok(stats.daily_ai_queries >= limits.daily_ai_queries),
            _ => Ok(false),
        }
    }
}