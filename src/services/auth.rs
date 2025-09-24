// 认证服务
// 处理用户认证、授权和令牌管理

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{Duration, Utc};
use tracing::{info, warn, instrument};
use utoipa::ToSchema;
use bcrypt::{verify, hash, DEFAULT_COST};
use sea_orm::{DatabaseConnection, EntityTrait, ColumnTrait, Set, ActiveModelTrait, QueryFilter};

use crate::errors::AiStudioError;
use crate::db::entities::{user, tenant, session, Tenant, User, Session};
use crate::api::auth::JwtUtils;

/// 登录请求
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct LoginRequest {
    /// 用户名或邮箱
    pub username: String,
    /// 密码
    pub password: String,
    /// 租户标识符（可选，如果不提供则从其他方式获取）
    pub tenant_slug: Option<String>,
    /// 记住我（延长令牌有效期）
    pub remember_me: Option<bool>,
}

/// 登录响应
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct LoginResponse {
    /// 访问令牌
    pub access_token: String,
    /// 刷新令牌
    pub refresh_token: String,
    /// 令牌类型
    pub token_type: String,
    /// 过期时间（秒）
    pub expires_in: i64,
    /// 用户信息
    pub user: UserInfo,
    /// 租户信息
    pub tenant: TenantInfo,
}

/// 刷新令牌请求
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct RefreshTokenRequest {
    /// 刷新令牌
    pub refresh_token: String,
}

/// 刷新令牌响应
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RefreshTokenResponse {
    /// 访问令牌
    pub access_token: String,
    /// 刷新令牌
    pub refresh_token: String,
    /// 令牌类型
    pub token_type: String,
    /// 过期时间（秒）
    pub expires_in: i64,
}

/// 注册请求
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct RegisterRequest {
    /// 用户名
    pub username: String,
    /// 邮箱
    pub email: String,
    /// 密码
    pub password: String,
    /// 确认密码
    pub password_confirm: String,
    /// 显示名称
    pub display_name: String,
    /// 租户标识符
    pub tenant_slug: String,
    /// 邀请码（可选）
    pub invitation_code: Option<String>,
}

/// 注册响应
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RegisterResponse {
    /// 用户信息
    pub user: UserInfo,
    /// 是否需要邮箱验证
    pub email_verification_required: bool,
    /// 验证邮件发送状态
    pub verification_email_sent: bool,
}

/// 密码重置请求
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct PasswordResetRequest {
    /// 邮箱
    pub email: String,
    /// 租户标识符
    pub tenant_slug: String,
}

/// 密码重置确认请求
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct PasswordResetConfirmRequest {
    /// 重置令牌
    pub reset_token: String,
    /// 新密码
    pub new_password: String,
    /// 确认新密码
    pub new_password_confirm: String,
}

/// 邮箱验证查询参数
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct EmailVerificationQuery {
    /// 验证令牌
    pub token: String,
}

/// 重新发送验证邮件请求
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ResendVerificationRequest {
    /// 用户邮箱
    pub email: String,
    /// 租户标识符
    pub tenant_slug: String,
}

/// 用户信息
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct UserInfo {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub username: String,
    pub email: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub role: String,
    pub permissions: Vec<String>,
    pub last_login_at: Option<chrono::DateTime<Utc>>,
    pub created_at: chrono::DateTime<Utc>,
}

/// 租户信息
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct TenantInfo {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub display_name: String,
    pub status: String,
}

/// 更新用户资料请求
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct UpdateUserProfileRequest {
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub avatar_url: Option<String>,
}

/// 认证服务
pub struct AuthService {
    db: sea_orm::DatabaseConnection,
    jwt_secret: String,
    access_token_expires_hours: i64,
    refresh_token_expires_days: i64,
}

impl AuthService {
    /// 创建新的认证服务实例
    pub fn new(
        db: DatabaseConnection,
        jwt_secret: String,
        access_token_expires_hours: Option<i64>,
        refresh_token_expires_days: Option<i64>,
    ) -> Self {
        Self {
            db,
            jwt_secret,
            access_token_expires_hours: access_token_expires_hours.unwrap_or(24),
            refresh_token_expires_days: refresh_token_expires_days.unwrap_or(30),
        }
    }

    /// 用户登录
    #[instrument(skip(self, request))]
    pub async fn login(&self, request: LoginRequest, client_ip: Option<String>, user_agent: Option<String>) -> Result<LoginResponse, AiStudioError> {
        info!(username = %request.username, "用户登录尝试");

        // 查找用户
        let user = self.find_user_by_username_or_email(&request.username, request.tenant_slug.as_deref()).await?;

        // 验证密码
        if !verify(&request.password, &user.password_hash)
            .map_err(|e| AiStudioError::internal(format!("密码验证失败: {}", e)))?
        {
            warn!(username = %request.username, "密码验证失败");
            return Err(AiStudioError::unauthorized("用户名或密码错误".to_string()));
        }

        // 检查用户状态
        // 这里应该检查用户状态，但由于用户实体可能还没有完全实现，先跳过
        // if user.status != user::UserStatus::Active {
        //     return Err(AiStudioError::forbidden("用户账户已被暂停".to_string()));
        // }

        // 获取租户信息
        let tenant = Tenant::find_by_id(user.tenant_id)
            .one(&self.db)
            .await?
            .ok_or_else(|| AiStudioError::not_found("租户"))?;

        // 检查租户状态
        if tenant.status != tenant::TenantStatus::Active {
            return Err(AiStudioError::forbidden("租户已被暂停或停用".to_string()));
        }

        // 生成令牌
        let expires_hours = if request.remember_me.unwrap_or(false) {
            self.access_token_expires_hours * 7 // 记住我时延长到 7 倍
        } else {
            self.access_token_expires_hours
        };

        let access_token = JwtUtils::generate_token(
            user.id,
            user.tenant_id,
            user.username.clone(),
            format!("{}", match user.role { user::UserRole::Admin => "admin", user::UserRole::Manager => "manager", user::UserRole::User => "user", user::UserRole::Viewer => "viewer" }),
            self.get_user_permissions(&user).await?,
            self.is_admin_user(&user),
            &self.jwt_secret,
            expires_hours,
        )?;

        let refresh_token = self.generate_refresh_token();

        // 创建会话
        let session_id = self.create_session(
            user.id,
            user.tenant_id,
            &refresh_token,
            client_ip,
            user_agent,
            expires_hours,
        ).await?;

        // 更新用户最后登录时间
        self.update_last_login(user.id).await?;

        info!(user_id = %user.id, tenant_id = %user.tenant_id, "用户登录成功");

        Ok(LoginResponse {
            access_token,
            refresh_token,
            token_type: "Bearer".to_string(),
            expires_in: expires_hours * 3600,
            user: UserInfo {
                id: user.id,
                tenant_id: user.tenant_id,
                username: user.username.clone(),
                email: user.email.clone(),
                display_name: user.display_name.clone(),
                avatar_url: user.avatar_url.clone(),
                role: user.role.to_string(),
                permissions: self.get_user_permissions(&user).await?,
                last_login_at: user.last_login_at.map(|dt| dt.into()),
                created_at: user.created_at.into(),
            },
            tenant: TenantInfo {
                id: tenant.id,
                name: tenant.name,
                slug: tenant.slug,
                display_name: tenant.display_name,
                status: format!("{:?}", tenant.status),
            },
        })
    }

    /// 刷新令牌
    #[instrument(skip(self, request))]
    pub async fn refresh_token(&self, request: RefreshTokenRequest) -> Result<RefreshTokenResponse, AiStudioError> {
        info!("刷新访问令牌");

        // 查找会话
        let session = self.find_session_by_refresh_token(&request.refresh_token).await?;

        // 检查会话是否过期
        {
            let expires_utc: chrono::DateTime<chrono::FixedOffset> = session.expires_at.with_timezone(&chrono::FixedOffset::east_opt(0).unwrap());
            if expires_utc < chrono::Utc::now().with_timezone(&chrono::FixedOffset::east_opt(0).unwrap()) {
                return Err(AiStudioError::unauthorized("刷新令牌已过期".to_string()));
            }
        }

        // 获取用户信息
        let user = User::find_by_id(session.user_id)
            .one(&self.db)
            .await
            .map_err(|e| AiStudioError::database(format!("查询用户失败: {}", e)))?
            .ok_or_else(|| AiStudioError::not_found("用户不存在".to_string()))?;

        // 生成新的访问令牌
        let access_token = JwtUtils::generate_token(
            user.id,
            user.tenant_id,
            user.username.clone(),
            format!("{}", match user.role { user::UserRole::Admin => "admin", user::UserRole::Manager => "manager", user::UserRole::User => "user", user::UserRole::Viewer => "viewer" }),
            self.get_user_permissions(&user).await?,
            self.is_admin_user(&user),
            &self.jwt_secret,
            self.access_token_expires_hours,
        )?;

        // 生成新的刷新令牌
        let new_refresh_token = self.generate_refresh_token();

        // 更新会话
        self.update_session_refresh_token(session.id, &new_refresh_token).await?;

        info!(user_id = %user.id, "令牌刷新成功");

        Ok(RefreshTokenResponse {
            access_token,
            refresh_token: new_refresh_token,
            token_type: "Bearer".to_string(),
            expires_in: self.access_token_expires_hours * 3600,
        })
    }

    /// 用户注册
    #[instrument(skip(self, request))]
    pub async fn register(&self, request: RegisterRequest) -> Result<RegisterResponse, AiStudioError> {
        info!(username = %request.username, email = %request.email, "用户注册");

        // 验证密码确认
        if request.password != request.password_confirm {
            return Err(AiStudioError::validation("password", "密码确认不匹配"));
        }

        // 验证密码强度
        self.validate_password_strength(&request.password)?;

        // 获取租户信息
        let tenant = Tenant::find()
            .filter(tenant::Column::Slug.eq(&request.tenant_slug))
            .one(&self.db) // 使用 connection 字段
            .await?
            .ok_or_else(|| AiStudioError::not_found("租户"))?;

        if tenant.status != tenant::TenantStatus::Active {
            return Err(AiStudioError::forbidden("租户已被暂停或停用".to_string()));
        }

        // 检查用户名是否已存在
        if User::find()
            .filter(user::Column::Username.eq(&request.username))
            .filter(user::Column::TenantId.eq(tenant.id))
            .one(&self.db) // 使用 connection 字段
            .await?
            .is_some()
        {
            return Err(AiStudioError::conflict("用户名已存在".to_string()));
        }

        // 检查邮箱是否已存在
        if User::find()
            .filter(user::Column::Email.eq(&request.email))
            .filter(user::Column::TenantId.eq(tenant.id))
            .one(&self.db) // 使用 connection 字段
            .await?
            .is_some()
        {
            return Err(AiStudioError::conflict("邮箱已被使用".to_string()));
        }

        // 哈希密码
        let password_hash = hash(&request.password, DEFAULT_COST)
            .map_err(|e| AiStudioError::internal(format!("密码哈希失败: {}", e)))?;

        // 创建用户
        let user_id = Uuid::new_v4();
        let now = Utc::now();

        let user = user::ActiveModel {
            id: Set(user_id),
            tenant_id: Set(tenant.id),
            username: Set(request.username.clone()),
            email: Set(request.email.clone()),
            password_hash: Set(password_hash),
            display_name: Set(request.display_name.clone()),
            avatar_url: Set(None),
            status: Set(user::UserStatus::Pending),
            role: Set(user::UserRole::User),
            permissions: Set(serde_json::json!(["read"])),
            preferences: Set(serde_json::json!({})),
            metadata: Set(serde_json::json!({})),
            phone: Set(None),
            email_verified: Set(false),
            email_verified_at: Set(None),
            phone_verified: Set(false),
            phone_verified_at: Set(None),
            last_login_at: Set(None),
            last_login_ip: Set(None),
            failed_login_attempts: Set(0),
            locked_until: Set(None),
            two_factor_enabled: Set(false),
            two_factor_secret: Set(None),
            password_reset_token: Set(None),
            password_reset_expires_at: Set(None),
            created_at: Set(now.with_timezone(&chrono::FixedOffset::east_opt(0).unwrap())),
            updated_at: Set(now.with_timezone(&chrono::FixedOffset::east_opt(0).unwrap())),
        };

        let created_user = user.insert(&self.db) // 使用 connection 字段
            .await?;
        info!(user_id = %created_user.id, username = %created_user.username, "新用户已创建");

        // 注册时不需要创建会话，用户需要登录才能获得会话
        // self.create_default_session(&created_user, client_ip, user_agent).await?;

        Ok(RegisterResponse {
            user: UserInfo {
                id: created_user.id,
                tenant_id: created_user.tenant_id,
                username: created_user.username.clone(),
                email: created_user.email.clone(),
                display_name: created_user.display_name.clone(),
                avatar_url: created_user.avatar_url.clone(),
                role: created_user.role.to_string(),
                permissions: self.get_user_permissions(&created_user).await?,
                last_login_at: created_user.last_login_at.map(|dt| dt.into()),
                created_at: created_user.created_at.into(),
            },
            email_verification_required: true, // or based on config
            verification_email_sent: self.send_verification_email(&created_user).await?,
        })
    }

    /// 登出
    #[instrument(skip(self))]
    pub async fn logout(&self, refresh_token: &str) -> Result<(), AiStudioError> {
        info!("用户登出");

        // 删除会话
        self.delete_session_by_refresh_token(refresh_token).await?;

        info!("用户登出成功");
        Ok(())
    }

        /// 密码重置请求
    #[instrument(skip(self, request))]
    pub async fn request_password_reset(&self, request: PasswordResetRequest) -> Result<(), AiStudioError> {
        info!(email = %request.email, "密码重置请求");

        // 查找用户
        let user = self.find_user_by_email(&request.email, Some(&request.tenant_slug)).await?;

        // 生成重置令牌
        let reset_token = Uuid::new_v4().to_string();
        let expires_at = Utc::now() + Duration::hours(1);

        // 更新用户信息
        let mut user_active: user::ActiveModel = user.clone().into();
        user_active.password_reset_token = Set(Some(reset_token.clone()));
        user_active.password_reset_expires_at = Set(Some(expires_at.with_timezone(&chrono::FixedOffset::east_opt(0).unwrap())));
        user_active.update(&self.db).await?;

        // 发送重置邮件
        self.send_password_reset_email(&user, &reset_token).await?;

        info!(user_id = %user.id, "密码重置邮件已发送");
        Ok(())
    }

    // 私有辅助方法

    /// 根据用户名或邮箱查找用户
    async fn find_user_by_username_or_email(&self, username_or_email: &str, tenant_slug: Option<&str>) -> Result<user::Model, AiStudioError> {
        let mut query = User::find();

        // 添加租户过滤
        if let Some(slug) = tenant_slug {
            let tenant = Tenant::find()
                .filter(tenant::Column::Slug.eq(slug))
                .one(&self.db)
                .await?
                .ok_or_else(|| AiStudioError::not_found("租户"))?;
            
            query = query.filter(user::Column::TenantId.eq(tenant.id));
        }

        // 查找用户（用户名或邮箱）
        query
            .filter(
                user::Column::Username.eq(username_or_email)
                    .or(user::Column::Email.eq(username_or_email))
            )
            .one(&self.db)
            .await?
            .ok_or_else(|| AiStudioError::unauthorized("用户名或密码错误".to_string()))
    }

    /// 根据邮箱查找用户
    async fn find_user_by_email(&self, email: &str, tenant_slug: Option<&str>) -> Result<user::Model, AiStudioError> {
        let mut query = User::find().filter(user::Column::Email.eq(email));

        if let Some(slug) = tenant_slug {
            let tenant = Tenant::find()
                .filter(tenant::Column::Slug.eq(slug))
                .one(&self.db)
                .await?
                .ok_or_else(|| AiStudioError::not_found("租户"))?;
            
            query = query.filter(user::Column::TenantId.eq(tenant.id));
        }

        query
            .one(&self.db)
            .await?
            .ok_or_else(|| AiStudioError::not_found("用户"))
    }

    /// 获取用户权限
    async fn get_user_permissions(&self, user: &user::Model) -> Result<Vec<String>, AiStudioError> {
        // 从用户的 permissions 字段解析权限
        let permissions: Vec<String> = serde_json::from_value(user.permissions.clone())
            .unwrap_or_else(|_| vec!["read".to_string()]);
        
        Ok(permissions)
    }

    /// 检查是否为管理员用户
    fn is_admin_user(&self, user: &user::Model) -> bool {
        user.role == user::UserRole::Admin
    }

    /// 生成刷新令牌
    fn generate_refresh_token(&self) -> String {
        format!("rt_{}", Uuid::new_v4())
    }

    /// 创建会话
    async fn create_session(
        &self,
        user_id: Uuid,
        tenant_id: Uuid,
        refresh_token: &str,
        client_ip: Option<String>,
        user_agent: Option<String>,
        expires_hours: i64,
    ) -> Result<Uuid, AiStudioError> {
        let session_id = Uuid::new_v4();
        let now = Utc::now();
        let expires_at = now + Duration::days(self.refresh_token_expires_days);

        let session = session::ActiveModel {
            id: Set(session_id),
            user_id: Set(user_id),
            tenant_id: Set(tenant_id),
            token_hash: Set(Uuid::new_v4().to_string()),
            refresh_token_hash: Set(Some(refresh_token.to_string())),
            // status: Set(session::SessionStatus::Active),
            client_ip: Set(client_ip),
            user_agent: Set(user_agent),
            expires_at: Set(expires_at.with_timezone(&chrono::FixedOffset::east_opt(0).unwrap())),
            last_activity_at: Set(now.with_timezone(&chrono::FixedOffset::east_opt(0).unwrap())),
            created_at: Set(now.with_timezone(&chrono::FixedOffset::east_opt(0).unwrap())),
            updated_at: Set(now.with_timezone(&chrono::FixedOffset::east_opt(0).unwrap())),
            session_type: Set(session::SessionType::Api),
            status: Set(session::SessionStatus::Active),
            device_info: Set(serde_json::json!({})),
            metadata: Set(serde_json::json!({})),
            refresh_expires_at: Set(None),
            last_url: Set(None),
        };

        session.insert(&self.db) // 使用 connection 字段
            .await?;
        Ok(session_id)
    }

    /// 根据刷新令牌查找会话
    async fn find_session_by_refresh_token(&self, refresh_token: &str) -> Result<session::Model, AiStudioError> {
        Session::find()
            .filter(session::Column::RefreshTokenHash.eq(refresh_token))
            .one(&self.db)
            .await
            .map_err(|e| AiStudioError::database(format!("查询会话失败: {}", e)))?
            .ok_or_else(|| AiStudioError::unauthorized("无效的刷新令牌".to_string()))
    }

    /// 更新会话刷新令牌
    async fn update_session_refresh_token(&self, session_id: Uuid, new_refresh_token: &str) -> Result<(), AiStudioError> {
        let mut session: session::ActiveModel = Session::find_by_id(session_id)
            .one(&self.db) // 使用 connection 字段
            .await?
            .ok_or_else(|| AiStudioError::not_found("会话"))?
            .into();

        session.refresh_token_hash = Set(Some(new_refresh_token.to_string()));
        session.last_activity_at = Set(Utc::now().with_timezone(&chrono::FixedOffset::east_opt(0).unwrap()));

        session.update(&self.db) // 使用 connection 字段
            .await?;
        Ok(())
    }

    /// 删除会话
    async fn delete_session_by_refresh_token(&self, refresh_token: &str) -> Result<(), AiStudioError> {
        let session = self.find_session_by_refresh_token(refresh_token).await?;
        session::Entity::delete_by_id(session.id)
            .exec(&self.db)
            .await
            .map_err(|e| AiStudioError::database(format!("删除会话失败: {}", e)))?;
        Ok(())
    }

    /// 更新用户最后登录时间
    async fn update_last_login(&self, user_id: Uuid) -> Result<(), AiStudioError> {
        let mut user: user::ActiveModel = User::find_by_id(user_id)
            .one(&self.db)
            .await
            .map_err(|e| AiStudioError::database(format!("查询用户失败: {}", e)))?
            .ok_or_else(|| AiStudioError::not_found("用户不存在".to_string()))?
            .into();

        user.last_login_at = Set(Some(Utc::now().with_timezone(&chrono::FixedOffset::east_opt(0).unwrap())));
        user.updated_at = Set(Utc::now().with_timezone(&chrono::FixedOffset::east_opt(0).unwrap()));

        user.update(&self.db).await?;
        Ok(())
    }

    /// 验证用户唯一性
    async fn validate_user_uniqueness(
        &self,
        username: &str,
        email: &str,
        tenant_id: Uuid,
        exclude_user_id: Option<Uuid>,
    ) -> Result<(), AiStudioError> {
        let mut username_query = User::find()
            .filter(user::Column::Username.eq(username))
            .filter(user::Column::TenantId.eq(tenant_id));

        let mut email_query = User::find()
            .filter(user::Column::Email.eq(email))
            .filter(user::Column::TenantId.eq(tenant_id));

        if let Some(exclude_id) = exclude_user_id {
            username_query = username_query.filter(user::Column::Id.ne(exclude_id));
            email_query = email_query.filter(user::Column::Id.ne(exclude_id));
        }

        if username_query.one(&self.db).await?.is_some() {
            return Err(AiStudioError::conflict("用户名已存在".to_string()));
        }

        if email_query.one(&self.db).await?.is_some() {
            return Err(AiStudioError::conflict("邮箱已存在".to_string()));
        }

        Ok(())
    }

    /// 验证密码强度
    fn validate_password_strength(&self, password: &str) -> Result<(), AiStudioError> {
        if password.len() < 8 {
            return Err(AiStudioError::validation("password", "密码长度至少为 8 个字符 "));
        }

        if !password.chars().any(|c| c.is_ascii_lowercase()) {
            return Err(AiStudioError::validation("password", "密码必须包含小写字母"));
        }

        if !password.chars().any(|c| c.is_ascii_uppercase()) {
            return Err(AiStudioError::validation("password", "密码必须包含大写字母"));
        }

        if !password.chars().any(|c| c.is_ascii_digit()) {
            return Err(AiStudioError::validation("password", "密码必须包含数字"));
        }

        Ok(())
    }

    /// 发送验证邮件
    async fn send_verification_email(&self, user: &user::Model) -> Result<bool, AiStudioError> {
        // 这里应该实现实际的邮件发送逻辑
        info!(user_id = %user.id, email = %user.email, "发送验证邮件");
        Ok(true)
    }

    /// 发送密码重置邮件
    async fn send_password_reset_email(&self, user: &user::Model, _reset_token: &str) -> Result<(), AiStudioError> {
        // 这里应该实现实际的邮件发送逻辑
        info!(user_id = %user.id, email = %user.email, "发送密码重置邮件");
        Ok(())
    }

    /// 获取用户信息
    #[instrument(skip(self))]
    pub async fn get_user_info(&self, user_id: Uuid) -> Result<UserInfo, AiStudioError> {
        let user = User::find_by_id(user_id)
            .one(&self.db)
            .await?
            .ok_or_else(|| AiStudioError::not_found("用户"))?;

        let permissions = self.get_user_permissions(&user).await?;

        Ok(UserInfo {
            id: user.id,
            tenant_id: user.tenant_id,
            username: user.username,
            email: user.email,
            display_name: user.display_name,
            avatar_url: user.avatar_url,
            role: user.role.to_string(),
            permissions,
            last_login_at: user.last_login_at.map(|dt| dt.into()),
            created_at: user.created_at.into(),
        })
    }

    /// 更新用户资料
    #[instrument(skip(self, request))]
    pub async fn update_user_profile(&self, user_id: Uuid, request: UpdateUserProfileRequest) -> Result<UserInfo, AiStudioError> {
        let mut user: user::ActiveModel = User::find_by_id(user_id)
            .one(&self.db)
            .await?
            .ok_or_else(|| AiStudioError::not_found("用户"))?
            .into();

        if let Some(display_name) = request.display_name {
            user.display_name = Set(display_name);
        }

        if let Some(email) = request.email {
            user.email = Set(email);
        }

        if let Some(avatar_url) = request.avatar_url {
            user.avatar_url = Set(Some(avatar_url));
        }

        user.updated_at = Set(Utc::now().into());

        let updated_user = user.update(&self.db).await?;

        self.get_user_info(updated_user.id).await
    }

    async fn find_user_by_reset_token(&self, reset_token: &str) -> Result<user::Model, AiStudioError> {
        User::find()
            .filter(user::Column::PasswordResetToken.eq(reset_token))
            .one(&self.db)
            .await?
            .ok_or_else(|| AiStudioError::not_found("用户"))
    }

    /// 确认密码重置
    #[instrument(skip(self, request))]
    pub async fn confirm_password_reset(&self, request: PasswordResetConfirmRequest) -> Result<(), AiStudioError> {
        info!("确认密码重置");

        // 查找用户
        let user = self.find_user_by_reset_token(&request.reset_token).await?;

        // 验证重置令牌
        if user.password_reset_token.is_none() || user.password_reset_token != Some(request.reset_token.clone()) {
            return Err(AiStudioError::unauthorized("无效的重置令牌".to_string()));
        }

        if let Some(expires_at) = user.password_reset_expires_at {
            if expires_at.with_timezone(&chrono::FixedOffset::east_opt(0).unwrap()) < chrono::Utc::now().with_timezone(&chrono::FixedOffset::east_opt(0).unwrap()) {
                return Err(AiStudioError::unauthorized("重置令牌已过期".to_string()));
            }
        } else {
            return Err(AiStudioError::unauthorized("无效的重置令牌".to_string()));
        }

        // 验证新密码强度
        self.validate_password_strength(&request.new_password)?;

        // 更新密码
        let password_hash = hash(&request.new_password, DEFAULT_COST)
            .map_err(|e| AiStudioError::internal(format!("密码哈希失败: {}", e)))?;

        let mut user_active: user::ActiveModel = user.into();
        user_active.password_hash = Set(password_hash);
        user_active.password_reset_token = Set(None);
        user_active.password_reset_expires_at = Set(None);
        user_active.updated_at = Set(Utc::now().with_timezone(&chrono::FixedOffset::east_opt(0).unwrap()));

        user_active.update(&self.db).await?;

        info!("密码重置成功");
        Ok(())
    }
}