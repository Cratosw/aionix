// API 密钥实体定义

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// API 密钥状态枚举
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize, ToSchema)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "api_key_status")]
pub enum ApiKeyStatus {
    #[sea_orm(string_value = "active")]
    Active,
    #[sea_orm(string_value = "revoked")]
    Revoked,
    #[sea_orm(string_value = "expired")]
    Expired,
}

/// API 密钥实体
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "api_keys")]
pub struct Model {
    /// API 密钥 ID
    #[sea_orm(primary_key)]
    pub id: Uuid,
    
    /// 租户 ID
    pub tenant_id: Uuid,
    
    /// 密钥名称
    #[sea_orm(column_type = "String(Some(255))")]
    pub name: String,
    
    /// 密钥描述
    #[sea_orm(column_type = "Text", nullable)]
    pub description: Option<String>,
    
    /// 密钥哈希值（存储加密后的密钥）
    #[sea_orm(column_type = "String(Some(255))", unique)]
    pub key_hash: String,
    
    /// 密钥前缀（用于显示，如 ak_xxx...）
    #[sea_orm(column_type = "String(Some(20))")]
    pub key_prefix: String,
    
    /// 权限列表（JSON 格式）
    #[sea_orm(column_type = "Json")]
    pub permissions: Json,
    
    /// API 密钥状态
    pub status: ApiKeyStatus,
    
    /// 过期时间
    #[sea_orm(nullable)]
    pub expires_at: Option<DateTimeWithTimeZone>,
    
    /// 最后使用时间
    #[sea_orm(nullable)]
    pub last_used_at: Option<DateTimeWithTimeZone>,
    
    /// 使用次数
    #[sea_orm(default_value = "0")]
    pub usage_count: i64,
    
    /// 创建时间
    pub created_at: DateTimeWithTimeZone,
    
    /// 更新时间
    pub updated_at: DateTimeWithTimeZone,
}

/// API 密钥关联关系
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 多对一：API 密钥 -> 租户
    #[sea_orm(
        belongs_to = "super::tenant::Entity",
        from = "Column::TenantId",
        to = "super::tenant::Column::Id"
    )]
    Tenant,
}

/// 实现与租户的关联
impl Related<super::tenant::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Tenant.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

/// API 密钥权限结构
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ApiKeyPermissions {
    /// 基础权限
    pub scopes: Vec<String>,
    /// 资源访问权限
    pub resources: Vec<String>,
    /// 操作权限
    pub actions: Vec<String>,
    /// IP 白名单
    pub allowed_ips: Option<Vec<String>>,
    /// 速率限制
    pub rate_limit: Option<ApiKeyRateLimit>,
}

/// API 密钥速率限制
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ApiKeyRateLimit {
    /// 每分钟请求数限制
    pub requests_per_minute: u32,
    /// 每小时请求数限制
    pub requests_per_hour: u32,
    /// 每日请求数限制
    pub requests_per_day: u32,
}

impl Default for ApiKeyPermissions {
    fn default() -> Self {
        Self {
            scopes: vec!["api_access".to_string()],
            resources: vec!["*".to_string()],
            actions: vec!["read".to_string()],
            allowed_ips: None,
            rate_limit: Some(ApiKeyRateLimit::default()),
        }
    }
}

impl Default for ApiKeyRateLimit {
    fn default() -> Self {
        Self {
            requests_per_minute: 60,
            requests_per_hour: 1000,
            requests_per_day: 10000,
        }
    }
}

/// API 密钥实用方法
impl Model {
    /// 检查 API 密钥是否活跃
    pub fn is_active(&self) -> bool {
        self.status == ApiKeyStatus::Active
    }
    
    /// 检查 API 密钥是否已过期
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            chrono::Utc::now().naive_utc() > expires_at.naive_utc()
        } else {
            false
        }
    }
    
    /// 检查 API 密钥是否可用
    pub fn is_usable(&self) -> bool {
        self.is_active() && !self.is_expired()
    }
    
    /// 获取权限配置
    pub fn get_permissions(&self) -> Result<ApiKeyPermissions, serde_json::Error> {
        serde_json::from_value(self.permissions.clone())
    }
    
    /// 检查是否有指定权限
    pub fn has_permission(&self, scope: &str) -> Result<bool, serde_json::Error> {
        let permissions = self.get_permissions()?;
        Ok(permissions.scopes.contains(&scope.to_string()) || 
           permissions.scopes.contains(&"*".to_string()))
    }
    
    /// 检查是否可以访问指定资源
    pub fn can_access_resource(&self, resource: &str) -> Result<bool, serde_json::Error> {
        let permissions = self.get_permissions()?;
        Ok(permissions.resources.contains(&resource.to_string()) || 
           permissions.resources.contains(&"*".to_string()))
    }
    
    /// 检查是否可以执行指定操作
    pub fn can_perform_action(&self, action: &str) -> Result<bool, serde_json::Error> {
        let permissions = self.get_permissions()?;
        Ok(permissions.actions.contains(&action.to_string()) || 
           permissions.actions.contains(&"*".to_string()))
    }
    
    /// 检查 IP 是否在白名单中
    pub fn is_ip_allowed(&self, ip: &str) -> Result<bool, serde_json::Error> {
        let permissions = self.get_permissions()?;
        if let Some(allowed_ips) = &permissions.allowed_ips {
            Ok(allowed_ips.contains(&ip.to_string()))
        } else {
            Ok(true) // 没有 IP 限制
        }
    }
    
    /// 生成显示用的密钥字符串
    pub fn display_key(&self) -> String {
        format!("{}...{}", self.key_prefix, "*".repeat(8))
    }
}

/// API 密钥工具函数
pub struct ApiKeyUtils;

impl ApiKeyUtils {
    /// 生成新的 API 密钥
    pub fn generate_key() -> String {
        use rand::Rng;
        const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
        const KEY_LEN: usize = 32;
        
        let mut rng = rand::thread_rng();
        let key: String = (0..KEY_LEN)
            .map(|_| {
                let idx = rng.gen_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect();
        
        format!("ak_{}", key)
    }
    
    /// 计算密钥哈希值
    pub fn hash_key(key: &str) -> Result<String, bcrypt::BcryptError> {
        bcrypt::hash(key, bcrypt::DEFAULT_COST)
    }
    
    /// 验证密钥
    pub fn verify_key(key: &str, hash: &str) -> Result<bool, bcrypt::BcryptError> {
        bcrypt::verify(key, hash)
    }
    
    /// 提取密钥前缀
    pub fn extract_prefix(key: &str) -> String {
        if key.len() >= 8 {
            key[..8].to_string()
        } else {
            key.to_string()
        }
    }
}