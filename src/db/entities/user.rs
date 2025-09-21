// 用户实体定义

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 用户状态枚举
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "user_status")]
pub enum UserStatus {
    #[sea_orm(string_value = "active")]
    Active,
    #[sea_orm(string_value = "inactive")]
    Inactive,
    #[sea_orm(string_value = "suspended")]
    Suspended,
    #[sea_orm(string_value = "pending")]
    Pending,
}

/// 用户角色枚举
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "user_role")]
pub enum UserRole {
    #[sea_orm(string_value = "admin")]
    Admin,
    #[sea_orm(string_value = "manager")]
    Manager,
    #[sea_orm(string_value = "user")]
    User,
    #[sea_orm(string_value = "viewer")]
    Viewer,
}

/// 用户实体
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "users")]
pub struct Model {
    /// 用户 ID
    #[sea_orm(primary_key)]
    pub id: Uuid,
    
    /// 租户 ID
    pub tenant_id: Uuid,
    
    /// 用户名（租户内唯一）
    #[sea_orm(column_type = "String(Some(100))")]
    pub username: String,
    
    /// 邮箱地址（全局唯一）
    #[sea_orm(column_type = "String(Some(255))", unique)]
    pub email: String,
    
    /// 密码哈希
    #[sea_orm(column_type = "String(Some(255))")]
    pub password_hash: String,
    
    /// 显示名称
    #[sea_orm(column_type = "String(Some(255))")]
    pub display_name: String,
    
    /// 头像 URL
    #[sea_orm(column_type = "String(Some(500))", nullable)]
    pub avatar_url: Option<String>,
    
    /// 用户角色
    pub role: UserRole,
    
    /// 用户状态
    pub status: UserStatus,
    
    /// 用户偏好设置（JSON 格式）
    #[sea_orm(column_type = "Json")]
    pub preferences: Json,
    
    /// 用户权限（JSON 格式）
    #[sea_orm(column_type = "Json")]
    pub permissions: Json,
    
    /// 用户元数据（JSON 格式）
    #[sea_orm(column_type = "Json")]
    pub metadata: Json,
    
    /// 手机号码
    #[sea_orm(column_type = "String(Some(50))", nullable)]
    pub phone: Option<String>,
    
    /// 邮箱验证状态
    pub email_verified: bool,
    
    /// 邮箱验证时间
    #[sea_orm(nullable)]
    pub email_verified_at: Option<DateTimeWithTimeZone>,
    
    /// 手机验证状态
    pub phone_verified: bool,
    
    /// 手机验证时间
    #[sea_orm(nullable)]
    pub phone_verified_at: Option<DateTimeWithTimeZone>,
    
    /// 两步验证启用状态
    pub two_factor_enabled: bool,
    
    /// 两步验证密钥
    #[sea_orm(column_type = "String(Some(255))", nullable)]
    pub two_factor_secret: Option<String>,
    
    /// 最后登录时间
    #[sea_orm(nullable)]
    pub last_login_at: Option<DateTimeWithTimeZone>,
    
    /// 最后登录 IP
    #[sea_orm(column_type = "String(Some(45))", nullable)]
    pub last_login_ip: Option<String>,
    
    /// 登录失败次数
    pub failed_login_attempts: i32,
    
    /// 账户锁定时间
    #[sea_orm(nullable)]
    pub locked_until: Option<DateTimeWithTimeZone>,
    
    /// 创建时间
    pub created_at: DateTimeWithTimeZone,
    
    /// 更新时间
    pub updated_at: DateTimeWithTimeZone,
}

/// 用户关联关系
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 多对一：用户 -> 租户
    #[sea_orm(
        belongs_to = "super::tenant::Entity",
        from = "Column::TenantId",
        to = "super::tenant::Column::Id"
    )]
    Tenant,
    
    /// 一对多：用户 -> 会话
    #[sea_orm(has_many = "super::session::Entity")]
    Sessions,
}

/// 实现与租户的关联
impl Related<super::tenant::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Tenant.def()
    }
}

/// 实现与会话的关联
impl Related<super::session::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Sessions.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

/// 用户偏好设置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreferences {
    /// 语言设置
    pub language: String,
    /// 时区设置
    pub timezone: String,
    /// 主题设置
    pub theme: String,
    /// 通知设置
    pub notifications: NotificationSettings,
    /// 界面设置
    pub ui_settings: UiSettings,
}

/// 通知设置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationSettings {
    /// 邮件通知
    pub email_notifications: bool,
    /// 短信通知
    pub sms_notifications: bool,
    /// 浏览器通知
    pub browser_notifications: bool,
    /// 通知类型设置
    pub notification_types: Vec<String>,
}

/// 界面设置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiSettings {
    /// 侧边栏折叠状态
    pub sidebar_collapsed: bool,
    /// 每页显示数量
    pub items_per_page: u32,
    /// 默认视图模式
    pub default_view: String,
}

/// 用户权限
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPermissions {
    /// 系统权限
    pub system_permissions: Vec<String>,
    /// 功能权限
    pub feature_permissions: Vec<String>,
    /// 资源权限
    pub resource_permissions: std::collections::HashMap<String, Vec<String>>,
}

/// 用户元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMetadata {
    /// 部门
    pub department: Option<String>,
    /// 职位
    pub position: Option<String>,
    /// 员工编号
    pub employee_id: Option<String>,
    /// 入职时间
    pub hire_date: Option<chrono::NaiveDate>,
    /// 自定义字段
    pub custom_fields: std::collections::HashMap<String, serde_json::Value>,
}

impl Default for UserPreferences {
    fn default() -> Self {
        Self {
            language: "zh-CN".to_string(),
            timezone: "Asia/Shanghai".to_string(),
            theme: "default".to_string(),
            notifications: NotificationSettings::default(),
            ui_settings: UiSettings::default(),
        }
    }
}

impl Default for NotificationSettings {
    fn default() -> Self {
        Self {
            email_notifications: true,
            sms_notifications: false,
            browser_notifications: true,
            notification_types: vec![
                "system".to_string(),
                "security".to_string(),
                "updates".to_string(),
            ],
        }
    }
}

impl Default for UiSettings {
    fn default() -> Self {
        Self {
            sidebar_collapsed: false,
            items_per_page: 20,
            default_view: "list".to_string(),
        }
    }
}

impl Default for UserPermissions {
    fn default() -> Self {
        Self {
            system_permissions: vec!["read".to_string()],
            feature_permissions: vec!["basic".to_string()],
            resource_permissions: std::collections::HashMap::new(),
        }
    }
}

impl Default for UserMetadata {
    fn default() -> Self {
        Self {
            department: None,
            position: None,
            employee_id: None,
            hire_date: None,
            custom_fields: std::collections::HashMap::new(),
        }
    }
}

/// 用户实用方法
impl Model {
    /// 检查用户是否活跃
    pub fn is_active(&self) -> bool {
        self.status == UserStatus::Active
    }
    
    /// 检查用户是否被锁定
    pub fn is_locked(&self) -> bool {
        if let Some(locked_until) = self.locked_until {
            chrono::Utc::now().naive_utc() < locked_until.naive_utc()
        } else {
            false
        }
    }
    
    /// 检查用户是否为管理员
    pub fn is_admin(&self) -> bool {
        self.role == UserRole::Admin
    }
    
    /// 检查用户是否为管理者
    pub fn is_manager(&self) -> bool {
        matches!(self.role, UserRole::Admin | UserRole::Manager)
    }
    
    /// 获取用户偏好设置
    pub fn get_preferences(&self) -> Result<UserPreferences, serde_json::Error> {
        serde_json::from_value(self.preferences.clone())
    }
    
    /// 获取用户权限
    pub fn get_permissions(&self) -> Result<UserPermissions, serde_json::Error> {
        serde_json::from_value(self.permissions.clone())
    }
    
    /// 获取用户元数据
    pub fn get_metadata(&self) -> Result<UserMetadata, serde_json::Error> {
        serde_json::from_value(self.metadata.clone())
    }
    
    /// 检查用户是否有特定权限
    pub fn has_permission(&self, permission: &str) -> Result<bool, serde_json::Error> {
        let permissions = self.get_permissions()?;
        Ok(permissions.system_permissions.contains(&permission.to_string()) ||
           permissions.feature_permissions.contains(&permission.to_string()))
    }
    
    /// 检查用户是否有资源权限
    pub fn has_resource_permission(&self, resource: &str, action: &str) -> Result<bool, serde_json::Error> {
        let permissions = self.get_permissions()?;
        if let Some(resource_perms) = permissions.resource_permissions.get(resource) {
            Ok(resource_perms.contains(&action.to_string()))
        } else {
            Ok(false)
        }
    }
    
    /// 获取显示名称或用户名
    pub fn get_display_name(&self) -> &str {
        if self.display_name.is_empty() {
            &self.username
        } else {
            &self.display_name
        }
    }
    
    /// 检查是否需要重置密码
    pub fn needs_password_reset(&self) -> bool {
        // 可以根据业务需求实现密码过期逻辑
        false
    }
    
    /// 检查登录失败次数是否过多
    pub fn has_too_many_failed_attempts(&self) -> bool {
        self.failed_login_attempts >= 5
    }
}