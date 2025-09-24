// 用户仓储实现

use crate::db::entities::{user, prelude::*};
use crate::errors::AiStudioError;
use sea_orm::{prelude::*, *};
use uuid::Uuid;
use tracing::{info, instrument};

/// 用户仓储
pub struct UserRepository;

impl UserRepository {
    /// 创建新用户
    #[instrument(skip(db, password_hash))]
    pub async fn create(
        db: &DatabaseConnection,
        tenant_id: Uuid,
        username: String,
        email: String,
        password_hash: String,
        display_name: String,
        role: user::UserRole,
    ) -> Result<user::Model, AiStudioError> {
        info!(tenant_id = %tenant_id, username = %username, email = %email, "创建新用户");

        // 检查邮箱是否已存在
        if Self::exists_by_email(db, &email).await? {
            return Err(AiStudioError::conflict(format!("邮箱 '{}' 已存在", email)));
        }

        // 检查用户名在租户内是否已存在
        if Self::exists_by_username_in_tenant(db, tenant_id, &username).await? {
            return Err(AiStudioError::conflict(format!("用户名 '{}' 在该租户内已存在", username)));
        }

        let user = user::ActiveModel {
            id: Set(Uuid::new_v4()),
            tenant_id: Set(tenant_id),
            username: Set(username),
            email: Set(email),
            password_hash: Set(password_hash),
            display_name: Set(display_name),
            avatar_url: Set(None),
            role: Set(role),
            status: Set(user::UserStatus::Active),
            preferences: Set(serde_json::to_value(user::UserPreferences::default())?),
            permissions: Set(serde_json::to_value(user::UserPermissions::default())?),
            metadata: Set(serde_json::to_value(user::UserMetadata::default())?),
            phone: Set(None),
            email_verified: Set(false),
            email_verified_at: Set(None),
            phone_verified: Set(false),
            phone_verified_at: Set(None),
            two_factor_enabled: Set(false),
            two_factor_secret: Set(None),
            last_login_at: Set(None),
            last_login_ip: Set(None),
            failed_login_attempts: Set(0),
            locked_until: Set(None),
            password_reset_token: Set(None),
            password_reset_expires_at: Set(None),
            created_at: Set(chrono::Utc::now().into()),
            updated_at: Set(chrono::Utc::now().into()),
        };

        let result = user.insert(db).await?;
        info!(user_id = %result.id, "用户创建成功");
        Ok(result)
    }

    /// 根据 ID 查找用户
    #[instrument(skip(db))]
    pub async fn find_by_id(
        db: &DatabaseConnection,
        id: Uuid,
    ) -> Result<Option<user::Model>, AiStudioError> {
        let user = User::find_by_id(id).one(db).await?;
        Ok(user)
    }

    /// 根据邮箱查找用户
    #[instrument(skip(db))]
    pub async fn find_by_email(
        db: &DatabaseConnection,
        email: &str,
    ) -> Result<Option<user::Model>, AiStudioError> {
        let user = User::find()
            .filter(user::Column::Email.eq(email))
            .one(db)
            .await?;
        Ok(user)
    }

    /// 根据用户名和租户 ID 查找用户
    #[instrument(skip(db))]
    pub async fn find_by_username_in_tenant(
        db: &DatabaseConnection,
        tenant_id: Uuid,
        username: &str,
    ) -> Result<Option<user::Model>, AiStudioError> {
        let user = User::find()
            .filter(user::Column::TenantId.eq(tenant_id))
            .filter(user::Column::Username.eq(username))
            .one(db)
            .await?;
        Ok(user)
    }

    /// 检查邮箱是否存在
    #[instrument(skip(db))]
    pub async fn exists_by_email(
        db: &DatabaseConnection,
        email: &str,
    ) -> Result<bool, AiStudioError> {
        let count = User::find()
            .filter(user::Column::Email.eq(email))
            .count(db)
            .await?;
        Ok(count > 0)
    }

    /// 检查用户名在租户内是否存在
    #[instrument(skip(db))]
    pub async fn exists_by_username_in_tenant(
        db: &DatabaseConnection,
        tenant_id: Uuid,
        username: &str,
    ) -> Result<bool, AiStudioError> {
        let count = User::find()
            .filter(user::Column::TenantId.eq(tenant_id))
            .filter(user::Column::Username.eq(username))
            .count(db)
            .await?;
        Ok(count > 0)
    }

    /// 更新用户信息
    #[instrument(skip(db, user))]
    pub async fn update(
        db: &DatabaseConnection,
        user: user::Model,
    ) -> Result<user::Model, AiStudioError> {
        info!(user_id = %user.id, "更新用户信息");

        let mut active_model: user::ActiveModel = user.into();
        active_model.updated_at = Set(chrono::Utc::now().into());

        let result = active_model.update(db).await?;
        info!(user_id = %result.id, "用户信息更新成功");
        Ok(result)
    }

    /// 更新用户密码
    #[instrument(skip(db, password_hash))]
    pub async fn update_password(
        db: &DatabaseConnection,
        id: Uuid,
        password_hash: String,
    ) -> Result<user::Model, AiStudioError> {
        info!(user_id = %id, "更新用户密码");

        let user = Self::find_by_id(db, id).await?
            .ok_or_else(|| AiStudioError::not_found("用户"))?;

        let mut active_model: user::ActiveModel = user.into();
        active_model.password_hash = Set(password_hash);
        active_model.updated_at = Set(chrono::Utc::now().into());

        let result = active_model.update(db).await?;
        info!(user_id = %result.id, "用户密码更新成功");
        Ok(result)
    }

    /// 更新登录信息
    #[instrument(skip(db))]
    pub async fn update_login_info(
        db: &DatabaseConnection,
        id: Uuid,
        ip_address: Option<String>,
    ) -> Result<(), AiStudioError> {
        User::update_many()
            .col_expr(user::Column::LastLoginAt, Expr::value(chrono::Utc::now()))
            .col_expr(user::Column::LastLoginIp, Expr::value(ip_address))
            .col_expr(user::Column::FailedLoginAttempts, Expr::value(0))
            .col_expr(user::Column::UpdatedAt, Expr::value(chrono::Utc::now()))
            .filter(user::Column::Id.eq(id))
            .exec(db)
            .await?;
        Ok(())
    }

    /// 增加登录失败次数
    #[instrument(skip(db))]
    pub async fn increment_failed_attempts(
        db: &DatabaseConnection,
        id: Uuid,
    ) -> Result<(), AiStudioError> {
        User::update_many()
            .col_expr(
                user::Column::FailedLoginAttempts,
                Expr::col(user::Column::FailedLoginAttempts).add(1),
            )
            .col_expr(user::Column::UpdatedAt, Expr::value(chrono::Utc::now()))
            .filter(user::Column::Id.eq(id))
            .exec(db)
            .await?;
        Ok(())
    }

    /// 锁定用户账户
    #[instrument(skip(db))]
    pub async fn lock_account(
        db: &DatabaseConnection,
        id: Uuid,
        lock_duration_minutes: i64,
    ) -> Result<(), AiStudioError> {
        let locked_until = chrono::Utc::now() + chrono::Duration::minutes(lock_duration_minutes);
        
        User::update_many()
            .col_expr(user::Column::LockedUntil, Expr::value(locked_until))
            .col_expr(user::Column::UpdatedAt, Expr::value(chrono::Utc::now()))
            .filter(user::Column::Id.eq(id))
            .exec(db)
            .await?;
        Ok(())
    }

    /// 解锁用户账户
    #[instrument(skip(db))]
    pub async fn unlock_account(
        db: &DatabaseConnection,
        id: Uuid,
    ) -> Result<(), AiStudioError> {
        User::update_many()
            .col_expr(user::Column::LockedUntil, Expr::value(Option::<chrono::DateTime<chrono::Utc>>::None))
            .col_expr(user::Column::FailedLoginAttempts, Expr::value(0))
            .col_expr(user::Column::UpdatedAt, Expr::value(chrono::Utc::now()))
            .filter(user::Column::Id.eq(id))
            .exec(db)
            .await?;
        Ok(())
    }

    /// 获取租户内的用户列表
    #[instrument(skip(db))]
    pub async fn find_by_tenant(
        db: &DatabaseConnection,
        tenant_id: Uuid,
        limit: Option<u64>,
        offset: Option<u64>,
    ) -> Result<Vec<user::Model>, AiStudioError> {
        let mut query = User::find()
            .filter(user::Column::TenantId.eq(tenant_id))
            .order_by_desc(user::Column::CreatedAt);

        if let Some(limit) = limit {
            query = query.limit(limit);
        }

        if let Some(offset) = offset {
            query = query.offset(offset);
        }

        let users = query.all(db).await?;
        Ok(users)
    }

    /// 获取租户内用户总数
    #[instrument(skip(db))]
    pub async fn count_by_tenant(
        db: &DatabaseConnection,
        tenant_id: Uuid,
    ) -> Result<u64, AiStudioError> {
        let count = User::find()
            .filter(user::Column::TenantId.eq(tenant_id))
            .count(db)
            .await?;
        Ok(count)
    }

    /// 搜索用户
    #[instrument(skip(db))]
    pub async fn search_in_tenant(
        db: &DatabaseConnection,
        tenant_id: Uuid,
        query: &str,
        limit: Option<u64>,
        offset: Option<u64>,
    ) -> Result<Vec<user::Model>, AiStudioError> {
        let search_pattern = format!("%{}%", query);
        
        let mut search_query = User::find()
            .filter(user::Column::TenantId.eq(tenant_id))
            .filter(
                Condition::any()
                    .add(user::Column::Username.like(&search_pattern))
                    .add(user::Column::DisplayName.like(&search_pattern))
                    .add(user::Column::Email.like(&search_pattern))
            )
            .order_by_desc(user::Column::LastLoginAt);

        if let Some(limit) = limit {
            search_query = search_query.limit(limit);
        }

        if let Some(offset) = offset {
            search_query = search_query.offset(offset);
        }

        let users = search_query.all(db).await?;
        Ok(users)
    }
}