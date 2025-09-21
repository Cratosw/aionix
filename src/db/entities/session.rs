// 会话实体定义

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 会话状态枚举
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "session_status")]
pub enum SessionStatus {
    #[sea_orm(string_value = "active")]
    Active,
    #[sea_orm(string_value = "expired")]
    Expired,
    #[sea_orm(string_value = "revoked")]
    Revoked,
}

/// 会话类型枚举
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "session_type")]
pub enum SessionType {
    #[sea_orm(string_value = "web")]
    Web,
    #[sea_orm(string_value = "api")]
    Api,
    #[sea_orm(string_value = "mobile")]
    Mobile,
    #[sea_orm(string_value = "desktop")]
    Desktop,
}

/// 会话实体
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "sessions")]
pub struct Model {
    /// 会话 ID
    #[sea_orm(primary_key)]
    pub id: Uuid,
    
    /// 用户 ID
    pub user_id: Uuid,
    
    /// 租户 ID
    pub tenant_id: Uuid,
    
    /// 会话令牌（哈希后的）
    #[sea_orm(column_type = "String(Some(255))", unique)]
    pub token_hash: String,
    
    /// 刷新令牌（哈希后的）
    #[sea_orm(column_type = "String(Some(255))", nullable)]
    pub refresh_token_hash: Option<String>,
    
    /// 会话类型
    pub session_type: SessionType,
    
    /// 会话状态
    pub status: SessionStatus,
    
    /// 客户端 IP 地址
    #[sea_orm(column_type = "String(Some(45))", nullable)]
    pub client_ip: Option<String>,
    
    /// 用户代理字符串
    #[sea_orm(column_type = "Text", nullable)]
    pub user_agent: Option<String>,
    
    /// 设备信息（JSON 格式）
    #[sea_orm(column_type = "Json")]
    pub device_info: Json,
    
    /// 会话元数据（JSON 格式）
    #[sea_orm(column_type = "Json")]
    pub metadata: Json,
    
    /// 会话过期时间
    pub expires_at: DateTimeWithTimeZone,
    
    /// 刷新令牌过期时间
    #[sea_orm(nullable)]
    pub refresh_expires_at: Option<DateTimeWithTimeZone>,
    
    /// 最后活跃时间
    pub last_activity_at: DateTimeWithTimeZone,
    
    /// 最后访问的 URL
    #[sea_orm(column_type = "String(Some(1000))", nullable)]
    pub last_url: Option<String>,
    
    /// 创建时间
    pub created_at: DateTimeWithTimeZone,
    
    /// 更新时间
    pub updated_at: DateTimeWithTimeZone,
}

/// 会话关联关系
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 多对一：会话 -> 用户
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::UserId",
        to = "super::user::Column::Id"
    )]
    User,
    
    /// 多对一：会话 -> 租户
    #[sea_orm(
        belongs_to = "super::tenant::Entity",
        from = "Column::TenantId",
        to = "super::tenant::Column::Id"
    )]
    Tenant,
}

/// 实现与用户的关联
impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

/// 实现与租户的关联
impl Related<super::tenant::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Tenant.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

/// 设备信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    /// 设备类型
    pub device_type: String,
    /// 操作系统
    pub os: Option<String>,
    /// 操作系统版本
    pub os_version: Option<String>,
    /// 浏览器
    pub browser: Option<String>,
    /// 浏览器版本
    pub browser_version: Option<String>,
    /// 设备名称
    pub device_name: Option<String>,
    /// 屏幕分辨率
    pub screen_resolution: Option<String>,
    /// 是否为移动设备
    pub is_mobile: bool,
    /// 是否为机器人
    pub is_bot: bool,
}

/// 会话元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetadata {
    /// 登录方式
    pub login_method: String,
    /// 是否记住登录
    pub remember_me: bool,
    /// 会话标签
    pub session_tags: Vec<String>,
    /// 权限范围
    pub scopes: Vec<String>,
    /// 自定义数据
    pub custom_data: std::collections::HashMap<String, serde_json::Value>,
}

impl Default for DeviceInfo {
    fn default() -> Self {
        Self {
            device_type: "unknown".to_string(),
            os: None,
            os_version: None,
            browser: None,
            browser_version: None,
            device_name: None,
            screen_resolution: None,
            is_mobile: false,
            is_bot: false,
        }
    }
}

impl Default for SessionMetadata {
    fn default() -> Self {
        Self {
            login_method: "password".to_string(),
            remember_me: false,
            session_tags: Vec::new(),
            scopes: vec!["read".to_string(), "write".to_string()],
            custom_data: std::collections::HashMap::new(),
        }
    }
}

/// 会话实用方法
impl Model {
    /// 检查会话是否有效
    pub fn is_valid(&self) -> bool {
        self.status == SessionStatus::Active && !self.is_expired()
    }
    
    /// 检查会话是否过期
    pub fn is_expired(&self) -> bool {
        chrono::Utc::now() > self.expires_at
    }
    
    /// 检查刷新令牌是否过期
    pub fn is_refresh_token_expired(&self) -> bool {
        if let Some(refresh_expires_at) = self.refresh_expires_at {
            chrono::Utc::now() > refresh_expires_at
        } else {
            true
        }
    }
    
    /// 检查会话是否被撤销
    pub fn is_revoked(&self) -> bool {
        self.status == SessionStatus::Revoked
    }
    
    /// 获取设备信息
    pub fn get_device_info(&self) -> Result<DeviceInfo, serde_json::Error> {
        serde_json::from_value(self.device_info.clone())
    }
    
    /// 获取会话元数据
    pub fn get_metadata(&self) -> Result<SessionMetadata, serde_json::Error> {
        serde_json::from_value(self.metadata.clone())
    }
    
    /// 检查会话是否有特定权限范围
    pub fn has_scope(&self, scope: &str) -> Result<bool, serde_json::Error> {
        let metadata = self.get_metadata()?;
        Ok(metadata.scopes.contains(&scope.to_string()))
    }
    
    /// 获取会话剩余时间（秒）
    pub fn remaining_time(&self) -> i64 {
        let now = chrono::Utc::now();
        let expires_utc = self.expires_at.with_timezone(&chrono::Utc);
        if now > expires_utc {
            0
        } else {
            (expires_utc - now).num_seconds()
        }
    }
    
    /// 检查会话是否即将过期（30分钟内）
    pub fn is_expiring_soon(&self) -> bool {
        let remaining = self.remaining_time();
        remaining > 0 && remaining <= 1800 // 30 minutes
    }
    
    /// 获取会话持续时间（秒）
    pub fn duration(&self) -> i64 {
        let now = chrono::Utc::now();
        let created_utc = self.created_at.with_timezone(&chrono::Utc);
        (now - created_utc).num_seconds()
    }
    
    /// 获取空闲时间（秒）
    pub fn idle_time(&self) -> i64 {
        let now = chrono::Utc::now();
        let last_activity_utc = self.last_activity_at.with_timezone(&chrono::Utc);
        (now - last_activity_utc).num_seconds()
    }
    
    /// 检查会话是否空闲过久（超过2小时）
    pub fn is_idle_too_long(&self) -> bool {
        self.idle_time() > 7200 // 2 hours
    }
    
    /// 获取设备描述
    pub fn get_device_description(&self) -> String {
        match self.get_device_info() {
            Ok(device) => {
                let mut parts = Vec::new();
                
                if let Some(ref browser) = device.browser {
                    if let Some(ref version) = device.browser_version {
                        parts.push(format!("{} {}", browser, version));
                    } else {
                        parts.push(browser.clone());
                    }
                }
                
                if let Some(ref os) = device.os {
                    if let Some(ref version) = device.os_version {
                        parts.push(format!("{} {}", os, version));
                    } else {
                        parts.push(os.clone());
                    }
                }
                
                if parts.is_empty() {
                    device.device_type
                } else {
                    parts.join(" on ")
                }
            }
            Err(_) => "Unknown Device".to_string(),
        }
    }
    
    /// 获取位置信息（基于 IP）
    pub fn get_location_info(&self) -> Option<String> {
        // 这里可以集成 IP 地理位置服务
        // 目前返回 IP 地址
        self.client_ip.clone()
    }
}