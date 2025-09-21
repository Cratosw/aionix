// 租户仓储实现

use crate::db::entities::{tenant, prelude::*};
use crate::errors::AiStudioError;
use sea_orm::{prelude::*, *};
use uuid::Uuid;
use tracing::{info, warn, instrument};

/// 租户仓储
pub struct TenantRepository;

impl TenantRepository {
    /// 创建新租户
    #[instrument(skip(db))]
    pub async fn create(
        db: &DatabaseConnection,
        name: String,
        slug: String,
        display_name: String,
    ) -> Result<tenant::Model, AiStudioError> {
        info!(name = %name, slug = %slug, "创建新租户");

        // 检查租户名称和标识符是否已存在
        if Self::exists_by_name(db, &name).await? {
            return Err(AiStudioError::conflict(format!("租户名称 '{}' 已存在", name)));
        }

        if Self::exists_by_slug(db, &slug).await? {
            return Err(AiStudioError::conflict(format!("租户标识符 '{}' 已存在", slug)));
        }

        let tenant = tenant::ActiveModel {
            id: Set(Uuid::new_v4()),
            name: Set(name),
            slug: Set(slug),
            display_name: Set(display_name),
            description: Set(None),
            status: Set(tenant::TenantStatus::Active),
            config: Set(serde_json::to_value(tenant::TenantConfig::default())?),
            quota_limits: Set(serde_json::to_value(tenant::TenantQuotaLimits::default())?),
            usage_stats: Set(serde_json::to_value(tenant::TenantUsageStats::default())?),
            contact_email: Set(None),
            contact_phone: Set(None),
            created_at: Set(chrono::Utc::now().into()),
            updated_at: Set(chrono::Utc::now().into()),
            last_active_at: Set(Some(chrono::Utc::now().into())),
        };

        let result = tenant.insert(db).await?;
        info!(tenant_id = %result.id, "租户创建成功");
        Ok(result)
    }

    /// 根据 ID 查找租户
    #[instrument(skip(db))]
    pub async fn find_by_id(
        db: &DatabaseConnection,
        id: Uuid,
    ) -> Result<Option<tenant::Model>, AiStudioError> {
        let tenant = Tenant::find_by_id(id).one(db).await?;
        Ok(tenant)
    }

    /// 根据标识符查找租户
    #[instrument(skip(db))]
    pub async fn find_by_slug(
        db: &DatabaseConnection,
        slug: &str,
    ) -> Result<Option<tenant::Model>, AiStudioError> {
        let tenant = Tenant::find()
            .filter(tenant::Column::Slug.eq(slug))
            .one(db)
            .await?;
        Ok(tenant)
    }

    /// 根据名称查找租户
    #[instrument(skip(db))]
    pub async fn find_by_name(
        db: &DatabaseConnection,
        name: &str,
    ) -> Result<Option<tenant::Model>, AiStudioError> {
        let tenant = Tenant::find()
            .filter(tenant::Column::Name.eq(name))
            .one(db)
            .await?;
        Ok(tenant)
    }

    /// 检查租户名称是否存在
    #[instrument(skip(db))]
    pub async fn exists_by_name(
        db: &DatabaseConnection,
        name: &str,
    ) -> Result<bool, AiStudioError> {
        let count = Tenant::find()
            .filter(tenant::Column::Name.eq(name))
            .count(db)
            .await?;
        Ok(count > 0)
    }

    /// 检查租户标识符是否存在
    #[instrument(skip(db))]
    pub async fn exists_by_slug(
        db: &DatabaseConnection,
        slug: &str,
    ) -> Result<bool, AiStudioError> {
        let count = Tenant::find()
            .filter(tenant::Column::Slug.eq(slug))
            .count(db)
            .await?;
        Ok(count > 0)
    }

    /// 更新租户信息
    #[instrument(skip(db, tenant))]
    pub async fn update(
        db: &DatabaseConnection,
        tenant: tenant::Model,
    ) -> Result<tenant::Model, AiStudioError> {
        info!(tenant_id = %tenant.id, "更新租户信息");

        let mut active_model: tenant::ActiveModel = tenant.into();
        active_model.updated_at = Set(chrono::Utc::now().into());

        let result = active_model.update(db).await?;
        info!(tenant_id = %result.id, "租户信息更新成功");
        Ok(result)
    }

    /// 更新租户状态
    #[instrument(skip(db))]
    pub async fn update_status(
        db: &DatabaseConnection,
        id: Uuid,
        status: tenant::TenantStatus,
    ) -> Result<tenant::Model, AiStudioError> {
        info!(tenant_id = %id, status = ?status, "更新租户状态");

        let tenant = Self::find_by_id(db, id).await?
            .ok_or_else(|| AiStudioError::not_found("租户"))?;

        let mut active_model: tenant::ActiveModel = tenant.into();
        active_model.status = Set(status);
        active_model.updated_at = Set(chrono::Utc::now().into());

        let result = active_model.update(db).await?;
        info!(tenant_id = %result.id, "租户状态更新成功");
        Ok(result)
    }

    /// 更新租户配置
    #[instrument(skip(db, config))]
    pub async fn update_config(
        db: &DatabaseConnection,
        id: Uuid,
        config: tenant::TenantConfig,
    ) -> Result<tenant::Model, AiStudioError> {
        info!(tenant_id = %id, "更新租户配置");

        let tenant = Self::find_by_id(db, id).await?
            .ok_or_else(|| AiStudioError::not_found("租户"))?;

        let mut active_model: tenant::ActiveModel = tenant.into();
        active_model.config = Set(serde_json::to_value(config)?);
        active_model.updated_at = Set(chrono::Utc::now().into());

        let result = active_model.update(db).await?;
        info!(tenant_id = %result.id, "租户配置更新成功");
        Ok(result)
    }

    /// 更新租户使用统计
    #[instrument(skip(db, stats))]
    pub async fn update_usage_stats(
        db: &DatabaseConnection,
        id: Uuid,
        stats: tenant::TenantUsageStats,
    ) -> Result<tenant::Model, AiStudioError> {
        let tenant = Self::find_by_id(db, id).await?
            .ok_or_else(|| AiStudioError::not_found("租户"))?;

        let mut active_model: tenant::ActiveModel = tenant.into();
        active_model.usage_stats = Set(serde_json::to_value(stats)?);
        active_model.updated_at = Set(chrono::Utc::now().into());

        let result = active_model.update(db).await?;
        Ok(result)
    }

    /// 更新最后活跃时间
    #[instrument(skip(db))]
    pub async fn update_last_active(
        db: &DatabaseConnection,
        id: Uuid,
    ) -> Result<(), AiStudioError> {
        Tenant::update_many()
            .col_expr(tenant::Column::LastActiveAt, Expr::value(chrono::Utc::now()))
            .col_expr(tenant::Column::UpdatedAt, Expr::value(chrono::Utc::now()))
            .filter(tenant::Column::Id.eq(id))
            .exec(db)
            .await?;
        Ok(())
    }

    /// 获取活跃租户列表
    #[instrument(skip(db))]
    pub async fn find_active(
        db: &DatabaseConnection,
        limit: Option<u64>,
        offset: Option<u64>,
    ) -> Result<Vec<tenant::Model>, AiStudioError> {
        let mut query = Tenant::find()
            .filter(tenant::Column::Status.eq(tenant::TenantStatus::Active))
            .order_by_desc(tenant::Column::LastActiveAt);

        if let Some(limit) = limit {
            query = query.limit(limit);
        }

        if let Some(offset) = offset {
            query = query.offset(offset);
        }

        let tenants = query.all(db).await?;
        Ok(tenants)
    }

    /// 获取租户总数
    #[instrument(skip(db))]
    pub async fn count(db: &DatabaseConnection) -> Result<u64, AiStudioError> {
        let count = Tenant::find().count(db).await?;
        Ok(count)
    }

    /// 获取活跃租户总数
    #[instrument(skip(db))]
    pub async fn count_active(db: &DatabaseConnection) -> Result<u64, AiStudioError> {
        let count = Tenant::find()
            .filter(tenant::Column::Status.eq(tenant::TenantStatus::Active))
            .count(db)
            .await?;
        Ok(count)
    }

    /// 删除租户（软删除，更改状态为 Inactive）
    #[instrument(skip(db))]
    pub async fn soft_delete(
        db: &DatabaseConnection,
        id: Uuid,
    ) -> Result<tenant::Model, AiStudioError> {
        warn!(tenant_id = %id, "软删除租户");

        let result = Self::update_status(db, id, tenant::TenantStatus::Inactive).await?;
        warn!(tenant_id = %result.id, "租户已软删除");
        Ok(result)
    }

    /// 硬删除租户（谨慎使用）
    #[instrument(skip(db))]
    pub async fn hard_delete(
        db: &DatabaseConnection,
        id: Uuid,
    ) -> Result<(), AiStudioError> {
        warn!(tenant_id = %id, "硬删除租户");

        let result = Tenant::delete_by_id(id).exec(db).await?;
        if result.rows_affected == 0 {
            return Err(AiStudioError::not_found("租户"));
        }

        warn!(tenant_id = %id, "租户已硬删除");
        Ok(())
    }

    /// 搜索租户
    #[instrument(skip(db))]
    pub async fn search(
        db: &DatabaseConnection,
        query: &str,
        limit: Option<u64>,
        offset: Option<u64>,
    ) -> Result<Vec<tenant::Model>, AiStudioError> {
        let search_pattern = format!("%{}%", query);
        
        let mut search_query = Tenant::find()
            .filter(
                Condition::any()
                    .add(tenant::Column::Name.like(&search_pattern))
                    .add(tenant::Column::DisplayName.like(&search_pattern))
                    .add(tenant::Column::Slug.like(&search_pattern))
            )
            .order_by_desc(tenant::Column::LastActiveAt);

        if let Some(limit) = limit {
            search_query = search_query.limit(limit);
        }

        if let Some(offset) = offset {
            search_query = search_query.offset(offset);
        }

        let tenants = search_query.all(db).await?;
        Ok(tenants)
    }
}