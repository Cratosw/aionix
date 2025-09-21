// 会话仓储实现

use crate::db::entities::{session, prelude::*};
use crate::errors::AiStudioError;
use sea_orm::{prelude::*, *};
use uuid::Uuid;
use tracing::{info, warn, instrument};

/// 会话仓储
pub struct SessionRepository;

impl SessionRepository {
    /// 创建新会话
    #[instrument(skip(db, token_hash))]
    pub async fn create(
        db: &DatabaseConnection,
        user_id: Uuid,
        tenant_id: Uuid,
        token_hash: String,
        refresh_token_hash: Option<String>,
        session_type: session::SessionType,
        client_ip: Option<String>,
        user_agent: Option<String>,
        expires_at: chrono::DateTime<chrono::Utc>,
        refresh_expires_at: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<session::Model, AiStudioError> {
        info!(user_id = %user_id, tenant_id = %tenant_id, "创建新会话");

        let session = session::ActiveModel {
            id: Set(Uuid::new_v4()),
            user_id: Set(user_id),
            tenant_id: Set(tenant_id),
            token_hash: Set(token_hash),
            refresh_token_hash: Set(refresh_token_hash),
            session_type: Set(session_type),
            status: Set(session::SessionStatus::Active),
            client_ip: Set(client_ip),
            user_agent: Set(user_agent),
            device_info: Set(serde_json::to_value(session::DeviceInfo::default())?),
            metadata: Set(serde_json::to_value(session::SessionMetadata::default())?),
            expires_at: Set(expires_at.into()),
            refresh_expires_at: Set(refresh_expires_at.map(|dt| dt.into())),
            last_activity_at: Set(chrono::Utc::now().into()),
            last_url: Set(None),
            created_at: Set(chrono::Utc::now().into()),
            updated_at: Set(chrono::Utc::now().into()),
        };

        let result = session.insert(db).await?;
        info!(session_id = %result.id, "会话创建成功");
        Ok(result)
    }

    /// 根据 ID 查找会话
    #[instrument(skip(db))]
    pub async fn find_by_id(
        db: &DatabaseConnection,
        id: Uuid,
    ) -> Result<Option<session::Model>, AiStudioError> {
        let session = Session::find_by_id(id).one(db).await?;
        Ok(session)
    }

    /// 根据令牌哈希查找会话
    #[instrument(skip(db, token_hash))]
    pub async fn find_by_token_hash(
        db: &DatabaseConnection,
        token_hash: &str,
    ) -> Result<Option<session::Model>, AiStudioError> {
        let session = Session::find()
            .filter(session::Column::TokenHash.eq(token_hash))
            .filter(session::Column::Status.eq(session::SessionStatus::Active))
            .one(db)
            .await?;
        Ok(session)
    }

    /// 根据刷新令牌哈希查找会话
    #[instrument(skip(db, refresh_token_hash))]
    pub async fn find_by_refresh_token_hash(
        db: &DatabaseConnection,
        refresh_token_hash: &str,
    ) -> Result<Option<session::Model>, AiStudioError> {
        let session = Session::find()
            .filter(session::Column::RefreshTokenHash.eq(refresh_token_hash))
            .filter(session::Column::Status.eq(session::SessionStatus::Active))
            .one(db)
            .await?;
        Ok(session)
    }

    /// 更新会话活跃时间
    #[instrument(skip(db))]
    pub async fn update_activity(
        db: &DatabaseConnection,
        id: Uuid,
        last_url: Option<String>,
    ) -> Result<(), AiStudioError> {
        Session::update_many()
            .col_expr(session::Column::LastActivityAt, Expr::value(chrono::Utc::now()))
            .col_expr(session::Column::LastUrl, Expr::value(last_url))
            .col_expr(session::Column::UpdatedAt, Expr::value(chrono::Utc::now()))
            .filter(session::Column::Id.eq(id))
            .exec(db)
            .await?;
        Ok(())
    }

    /// 撤销会话
    #[instrument(skip(db))]
    pub async fn revoke(
        db: &DatabaseConnection,
        id: Uuid,
    ) -> Result<session::Model, AiStudioError> {
        info!(session_id = %id, "撤销会话");

        let session = Self::find_by_id(db, id).await?
            .ok_or_else(|| AiStudioError::not_found("会话"))?;

        let mut active_model: session::ActiveModel = session.into();
        active_model.status = Set(session::SessionStatus::Revoked);
        active_model.updated_at = Set(chrono::Utc::now().into());

        let result = active_model.update(db).await?;
        info!(session_id = %result.id, "会话已撤销");
        Ok(result)
    }

    /// 撤销用户的所有会话
    #[instrument(skip(db))]
    pub async fn revoke_all_by_user(
        db: &DatabaseConnection,
        user_id: Uuid,
    ) -> Result<u64, AiStudioError> {
        info!(user_id = %user_id, "撤销用户所有会话");

        let result = Session::update_many()
            .col_expr(session::Column::Status, Expr::value(session::SessionStatus::Revoked))
            .col_expr(session::Column::UpdatedAt, Expr::value(chrono::Utc::now()))
            .filter(session::Column::UserId.eq(user_id))
            .filter(session::Column::Status.eq(session::SessionStatus::Active))
            .exec(db)
            .await?;

        info!(user_id = %user_id, revoked_count = result.rows_affected, "用户会话已撤销");
        Ok(result.rows_affected)
    }

    /// 撤销租户的所有会话
    #[instrument(skip(db))]
    pub async fn revoke_all_by_tenant(
        db: &DatabaseConnection,
        tenant_id: Uuid,
    ) -> Result<u64, AiStudioError> {
        info!(tenant_id = %tenant_id, "撤销租户所有会话");

        let result = Session::update_many()
            .col_expr(session::Column::Status, Expr::value(session::SessionStatus::Revoked))
            .col_expr(session::Column::UpdatedAt, Expr::value(chrono::Utc::now()))
            .filter(session::Column::TenantId.eq(tenant_id))
            .filter(session::Column::Status.eq(session::SessionStatus::Active))
            .exec(db)
            .await?;

        info!(tenant_id = %tenant_id, revoked_count = result.rows_affected, "租户会话已撤销");
        Ok(result.rows_affected)
    }

    /// 清理过期会话
    #[instrument(skip(db))]
    pub async fn cleanup_expired(
        db: &DatabaseConnection,
    ) -> Result<u64, AiStudioError> {
        info!("清理过期会话");

        let now = chrono::Utc::now();
        let result = Session::update_many()
            .col_expr(session::Column::Status, Expr::value(session::SessionStatus::Expired))
            .col_expr(session::Column::UpdatedAt, Expr::value(now))
            .filter(session::Column::ExpiresAt.lt(now))
            .filter(session::Column::Status.eq(session::SessionStatus::Active))
            .exec(db)
            .await?;

        info!(expired_count = result.rows_affected, "过期会话已清理");
        Ok(result.rows_affected)
    }

    /// 获取用户的活跃会话
    #[instrument(skip(db))]
    pub async fn find_active_by_user(
        db: &DatabaseConnection,
        user_id: Uuid,
        limit: Option<u64>,
        offset: Option<u64>,
    ) -> Result<Vec<session::Model>, AiStudioError> {
        let mut query = Session::find()
            .filter(session::Column::UserId.eq(user_id))
            .filter(session::Column::Status.eq(session::SessionStatus::Active))
            .order_by_desc(session::Column::LastActivityAt);

        if let Some(limit) = limit {
            query = query.limit(limit);
        }

        if let Some(offset) = offset {
            query = query.offset(offset);
        }

        let sessions = query.all(db).await?;
        Ok(sessions)
    }

    /// 获取租户的活跃会话
    #[instrument(skip(db))]
    pub async fn find_active_by_tenant(
        db: &DatabaseConnection,
        tenant_id: Uuid,
        limit: Option<u64>,
        offset: Option<u64>,
    ) -> Result<Vec<session::Model>, AiStudioError> {
        let mut query = Session::find()
            .filter(session::Column::TenantId.eq(tenant_id))
            .filter(session::Column::Status.eq(session::SessionStatus::Active))
            .order_by_desc(session::Column::LastActivityAt);

        if let Some(limit) = limit {
            query = query.limit(limit);
        }

        if let Some(offset) = offset {
            query = query.offset(offset);
        }

        let sessions = query.all(db).await?;
        Ok(sessions)
    }

    /// 统计用户活跃会话数
    #[instrument(skip(db))]
    pub async fn count_active_by_user(
        db: &DatabaseConnection,
        user_id: Uuid,
    ) -> Result<u64, AiStudioError> {
        let count = Session::find()
            .filter(session::Column::UserId.eq(user_id))
            .filter(session::Column::Status.eq(session::SessionStatus::Active))
            .count(db)
            .await?;
        Ok(count)
    }

    /// 统计租户活跃会话数
    #[instrument(skip(db))]
    pub async fn count_active_by_tenant(
        db: &DatabaseConnection,
        tenant_id: Uuid,
    ) -> Result<u64, AiStudioError> {
        let count = Session::find()
            .filter(session::Column::TenantId.eq(tenant_id))
            .filter(session::Column::Status.eq(session::SessionStatus::Active))
            .count(db)
            .await?;
        Ok(count)
    }

    /// 删除旧会话记录
    #[instrument(skip(db))]
    pub async fn delete_old_sessions(
        db: &DatabaseConnection,
        days_old: i64,
    ) -> Result<u64, AiStudioError> {
        warn!(days_old = days_old, "删除旧会话记录");

        let cutoff_date = chrono::Utc::now() - chrono::Duration::days(days_old);
        let result = Session::delete_many()
            .filter(session::Column::CreatedAt.lt(cutoff_date))
            .filter(
                Condition::any()
                    .add(session::Column::Status.eq(session::SessionStatus::Expired))
                    .add(session::Column::Status.eq(session::SessionStatus::Revoked))
            )
            .exec(db)
            .await?;

        warn!(deleted_count = result.rows_affected, "旧会话记录已删除");
        Ok(result.rows_affected)
    }

    /// 更新会话设备信息
    #[instrument(skip(db, device_info))]
    pub async fn update_device_info(
        db: &DatabaseConnection,
        id: Uuid,
        device_info: session::DeviceInfo,
    ) -> Result<(), AiStudioError> {
        Session::update_many()
            .col_expr(session::Column::DeviceInfo, Expr::value(serde_json::to_value(device_info)?))
            .col_expr(session::Column::UpdatedAt, Expr::value(chrono::Utc::now()))
            .filter(session::Column::Id.eq(id))
            .exec(db)
            .await?;
        Ok(())
    }

    /// 更新会话元数据
    #[instrument(skip(db, metadata))]
    pub async fn update_metadata(
        db: &DatabaseConnection,
        id: Uuid,
        metadata: session::SessionMetadata,
    ) -> Result<(), AiStudioError> {
        Session::update_many()
            .col_expr(session::Column::Metadata, Expr::value(serde_json::to_value(metadata)?))
            .col_expr(session::Column::UpdatedAt, Expr::value(chrono::Utc::now()))
            .filter(session::Column::Id.eq(id))
            .exec(db)
            .await?;
        Ok(())
    }
}