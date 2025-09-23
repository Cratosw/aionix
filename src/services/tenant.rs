// 租户服务
// 管理多租户相关功能

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc, Datelike, NaiveDate};
use tracing::{info, warn, instrument};
use utoipa::ToSchema;
use sea_orm::{EntityTrait, ColumnTrait, QueryFilter, ActiveModelTrait, QuerySelect, Set};

use crate::errors::AiStudioError;
use crate::db::entities::{tenant, user};
use crate::db::DatabaseManager;
use crate::api::{PaginationQuery, PaginatedResponse, PaginationInfo};

/// 创建租户请求
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct CreateTenantRequest {
    /// 租户名称
    pub name: String,
    /// 租户标识符
    pub slug: String,
    /// 显示名称
    pub display_name: String,
    /// 描述
    pub description: Option<String>,
    /// 联系邮箱
    pub contact_email: Option<String>,
    /// 联系电话
    pub contact_phone: Option<String>,
    /// 配置
    pub config: Option<tenant::TenantConfig>,
    /// 配额限制
    pub quota_limits: Option<tenant::TenantQuotaLimits>,
}

/// 更新租户请求
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct UpdateTenantRequest {
    /// 显示名称
    pub display_name: Option<String>,
    /// 描述
    pub description: Option<String>,
    /// 状态
    pub status: Option<tenant::TenantStatus>,
    /// 联系邮箱
    pub contact_email: Option<String>,
    /// 联系电话
    pub contact_phone: Option<String>,
    /// 配置
    pub config: Option<tenant::TenantConfig>,
    /// 配额限制
    pub quota_limits: Option<tenant::TenantQuotaLimits>,
}

/// 租户查询过滤器
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TenantFilter {
    /// 状态过滤
    pub status: Option<tenant::TenantStatus>,
    /// 名称搜索
    pub name_search: Option<String>,
    /// 创建时间范围
    pub created_after: Option<chrono::DateTime<Utc>>,
    pub created_before: Option<chrono::DateTime<Utc>>,
}

/// 租户响应
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct TenantResponse {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub display_name: String,
    pub description: Option<String>,
    pub status: tenant::TenantStatus,
    pub config: tenant::TenantConfig,
    pub quota_limits: tenant::TenantQuotaLimits,
    pub usage_stats: tenant::TenantUsageStats,
    pub contact_email: Option<String>,
    pub contact_phone: Option<String>,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
    pub last_active_at: Option<chrono::DateTime<Utc>>,
}

/// 租户统计响应
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct TenantStatsResponse {
    pub total_tenants: u64,
    pub active_tenants: u64,
    pub suspended_tenants: u64,
    pub inactive_tenants: u64,
    pub tenants_created_today: u64,
    pub tenants_created_this_month: u64,
}

/// 租户服务
pub struct TenantService {
    db: DatabaseManager,
}

impl TenantService {
    /// 创建新的租户服务实例
    pub fn new(db: DatabaseManager) -> Self {
        Self { db }
    }

    /// 创建租户
    #[instrument(skip(self, request))]
    pub async fn create_tenant(&self, request: CreateTenantRequest) -> Result<TenantResponse, AiStudioError> {
        info!(name = %request.name, slug = %request.slug, "创建租户");

        // 验证租户名称和标识符的唯一性
        self.validate_tenant_uniqueness(&request.name, &request.slug, None).await?;

        // 验证标识符格式
        self.validate_slug_format(&request.slug)?;

        let tenant_id = Uuid::new_v4();
        let now = Utc::now();

        let config = request.config.unwrap_or_default();
        let quota_limits = request.quota_limits.unwrap_or_default();
        let usage_stats = tenant::TenantUsageStats::default();

        let tenant = tenant::ActiveModel {
            id: Set(tenant_id),
            name: Set(request.name.clone()),
            slug: Set(request.slug.clone()),
            display_name: Set(request.display_name.clone()),
            description: Set(request.description.clone()),
            status: Set(tenant::TenantStatus::Active),
            config: Set(serde_json::to_value(&config)?),
            quota_limits: Set(serde_json::to_value(&quota_limits)?),
            usage_stats: Set(serde_json::to_value(&usage_stats)?),
            contact_email: Set(request.contact_email.clone()),
            contact_phone: Set(request.contact_phone.clone()),
            created_at: Set(now.into()),
            updated_at: Set(now.into()),
            last_active_at: Set(Some(now.into())),
        };

        let created_tenant = tenant.insert(&self.db.connection).await?;

        info!(tenant_id = %tenant_id, "租户创建成功");

        Ok(TenantResponse {
            id: created_tenant.id,
            name: created_tenant.name,
            slug: created_tenant.slug,
            display_name: created_tenant.display_name,
            description: created_tenant.description,
            status: created_tenant.status,
            config,
            quota_limits,
            usage_stats,
            contact_email: created_tenant.contact_email,
            contact_phone: created_tenant.contact_phone,
            created_at: created_tenant.created_at.into(),
            updated_at: created_tenant.updated_at.into(),
            last_active_at: created_tenant.last_active_at.map(|dt| dt.into()),
        })
    }

    /// 根据 ID 获取租户
    #[instrument(skip(self))]
    pub async fn get_tenant_by_id(&self, tenant_id: Uuid) -> Result<TenantResponse, AiStudioError> {
        let tenant = Tenant::find_by_id(tenant_id)
            .one(&self.db.connection)
            .await?
            .ok_or_else(|| AiStudioError::not_found("租户"))?;

        self.convert_to_response(tenant).await
    }

    /// 根据标识符获取租户
    #[instrument(skip(self))]
    pub async fn get_tenant_by_slug(&self, slug: &str) -> Result<TenantResponse, AiStudioError> {
        let tenant = Tenant::find()
            .filter(tenant::Column::Slug.eq(slug))
            .one(&self.db.connection)
            .await?
            .ok_or_else(|| AiStudioError::not_found("租户"))?;

        self.convert_to_response(tenant).await
    }

    /// 更新租户
    #[instrument(skip(self, request))]
    pub async fn update_tenant(&self, tenant_id: Uuid, request: UpdateTenantRequest) -> Result<TenantResponse, AiStudioError> {
        info!(tenant_id = %tenant_id, "更新租户");

        let tenant = Tenant::find_by_id(tenant_id)
            .one(&self.db.connection)
            .await?
            .ok_or_else(|| AiStudioError::not_found("租户"))?;

        let mut active_tenant: tenant::ActiveModel = tenant.into();

        // 更新字段
        if let Some(display_name) = request.display_name {
            active_tenant.display_name = Set(display_name);
        }
        if let Some(description) = request.description {
            active_tenant.description = Set(Some(description));
        }
        if let Some(status) = request.status {
            active_tenant.status = Set(status);
        }
        if let Some(contact_email) = request.contact_email {
            active_tenant.contact_email = Set(Some(contact_email));
        }
        if let Some(contact_phone) = request.contact_phone {
            active_tenant.contact_phone = Set(Some(contact_phone));
        }
        if let Some(config) = request.config {
            active_tenant.config = Set(serde_json::to_value(&config)?);
        }
        if let Some(quota_limits) = request.quota_limits {
            active_tenant.quota_limits = Set(serde_json::to_value(&quota_limits)?);
        }

        active_tenant.updated_at = Set(Utc::now().into());

        let updated_tenant = active_tenant.update(&self.db.connection).await?;

        info!(tenant_id = %tenant_id, "租户更新成功");

        self.convert_to_response(updated_tenant).await
    }

    /// 删除租户
    #[instrument(skip(self))]
    pub async fn delete_tenant(&self, tenant_id: Uuid) -> Result<(), AiStudioError> {
        info!(tenant_id = %tenant_id, "删除租户");

        let tenant = Tenant::find_by_id(tenant_id)
            .one(&self.db)
            .await?
            .ok_or_else(|| AiStudioError::not_found("租户"))?;

        // 删除租户下的所有用户
        user::Entity::delete_many()
            .filter(user::Column::TenantId.eq(tenant_id))
            .exec(&self.db.connection)
            .await?;

        // 删除租户
        tenant::Entity::delete_by_id(tenant_id)
            .exec(&self.db.connection)
            .await?;

        info!(tenant_id = %tenant_id, "租户删除成功");

        Ok(())
    }

    /// 列出租户（分页）
    #[instrument(skip(self))]
    pub async fn list_tenants(
        &self,
        filter: Option<TenantFilter>,
        pagination: PaginationQuery,
    ) -> Result<PaginatedResponse<TenantResponse>, AiStudioError> {
        let mut query = Tenant::find();

        // 应用过滤器
        if let Some(filter) = filter {
            if let Some(status) = filter.status {
                query = query.filter(tenant::Column::Status.eq(status));
            }
            if let Some(name_search) = filter.name_search {
                query = query.filter(
                    tenant::Column::Name.contains(&name_search)
                        .or(tenant::Column::DisplayName.contains(&name_search))
                );
            }
            if let Some(created_after) = filter.created_after {
                query = query.filter(tenant::Column::CreatedAt.gte(created_after));
            }
            if let Some(created_before) = filter.created_before {
                query = query.filter(tenant::Column::CreatedAt.lte(created_before));
            }
        }

        // 应用排序
        query = match pagination.sort_by.as_deref() {
            Some("name") => query.order_by_asc(tenant::Column::Name),
            Some("created_at") => query.order_by_desc(tenant::Column::CreatedAt),
            Some("updated_at") => query.order_by_desc(tenant::Column::UpdatedAt),
            _ => query.order_by_desc(tenant::Column::CreatedAt),
        };

        // 获取总数
        let total = query.clone().count(&self.db.connection).await?;

        // 应用分页
        let tenants = query
            .offset(pagination.offset())
            .limit(pagination.limit())
            .all(&self.db.connection)
            .await?;

        // 转换为响应格式
        let mut tenant_responses = Vec::new();
        for tenant in tenants {
            tenant_responses.push(self.convert_to_response(tenant).await?);
        }

        let pagination_info = PaginationInfo::new(pagination.page, pagination.page_size, total);

        Ok(PaginatedResponse::new(tenant_responses, pagination_info))
    }

    /// 获取租户统计信息
    #[instrument(skip(self))]
    pub async fn get_tenant_stats(&self) -> Result<TenantStatsResponse, AiStudioError> {
        let total_tenants = Tenant::find().count(&self.db.connection).await?;
        
        let active_tenants = Tenant::find()
            .filter(tenant::Column::Status.eq(tenant::TenantStatus::Active))
            .count(&self.db.connection).await?;
        
        let suspended_tenants = Tenant::find()
            .filter(tenant::Column::Status.eq(tenant::TenantStatus::Suspended))
            .count(&self.db.connection).await?;
        
        let inactive_tenants = Tenant::find()
            .filter(tenant::Column::Status.eq(tenant::TenantStatus::Inactive))
            .count(&self.db.connection).await?;

        let today = Utc::now().date_naive();
        let tenants_created_today = Tenant::find()
            .filter(tenant::Column::CreatedAt.gte(today.and_hms_opt(0, 0, 0).unwrap()))
            .count(&self.db.connection).await?;

        let month_start = today.with_day(1).unwrap();
        let tenants_created_this_month = Tenant::find()
            .filter(tenant::Column::CreatedAt.gte(month_start.and_hms_opt(0, 0, 0).unwrap()))
            .count(&self.db.connection).await?;

        Ok(TenantStatsResponse {
            total_tenants,
            active_tenants,
            suspended_tenants,
            inactive_tenants,
            tenants_created_today,
            tenants_created_this_month,
        })
    }

    /// 检查租户配额
    #[instrument(skip(self))]
    pub async fn check_tenant_quota(&self, tenant_id: Uuid, resource_type: &str, requested_amount: i64) -> Result<bool, AiStudioError> {
        TenantQuotaChecker::check_quota(&self.db, tenant_id, resource_type, requested_amount).await
    }

    /// 更新租户使用统计
    #[instrument(skip(self))]
    pub async fn update_tenant_usage(&self, tenant_id: Uuid) -> Result<(), AiStudioError> {
        let stats = TenantStatsQuery::get_stats(&self.db, tenant_id).await?;
        TenantStatsQuery::update_usage_stats(&self.db, tenant_id, &stats).await?;
        Ok(())
    }

    /// 暂停租户
    #[instrument(skip(self))]
    pub async fn suspend_tenant(&self, tenant_id: Uuid, reason: Option<String>) -> Result<TenantResponse, AiStudioError> {
        info!(tenant_id = %tenant_id, reason = ?reason, "暂停租户");

        let request = UpdateTenantRequest {
            status: Some(tenant::TenantStatus::Suspended),
            ..Default::default()
        };

        self.update_tenant(tenant_id, request).await
    }

    /// 激活租户
    #[instrument(skip(self))]
    pub async fn activate_tenant(&self, tenant_id: Uuid) -> Result<TenantResponse, AiStudioError> {
        info!(tenant_id = %tenant_id, "激活租户");

        let request = UpdateTenantRequest {
            status: Some(tenant::TenantStatus::Active),
            ..Default::default()
        };

        self.update_tenant(tenant_id, request).await
    }

    // 私有辅助方法

    /// 验证租户唯一性
    async fn validate_tenant_uniqueness(&self, name: &str, slug: &str, exclude_id: Option<Uuid>) -> Result<(), AiStudioError> {
        let mut name_query = Tenant::find().filter(tenant::Column::Name.eq(name));
        let mut slug_query = Tenant::find().filter(tenant::Column::Slug.eq(slug));

        if let Some(exclude_id) = exclude_id {
            name_query = name_query.filter(tenant::Column::Id.ne(exclude_id));
            slug_query = slug_query.filter(tenant::Column::Id.ne(exclude_id));
        }

        if name_query.one(&self.db).await?.is_some() {
            return Err(AiStudioError::conflict("租户名称已存在".to_string()));
        }

        if slug_query.one(&self.db).await?.is_some() {
            return Err(AiStudioError::conflict("租户标识符已存在".to_string()));
        }

        Ok(())
    }

    /// 验证标识符格式
    fn validate_slug_format(&self, slug: &str) -> Result<(), AiStudioError> {
        if slug.is_empty() {
            return Err(AiStudioError::validation("slug", "租户标识符不能为空"));
        }

        if slug.len() > 100 {
            return Err(AiStudioError::validation("slug", "租户标识符长度不能超过100个字符"));
        }

        // 检查格式：只允许小写字母、数字和连字符，不能以连字符开头或结尾
        let regex = regex::Regex::new(r"^[a-z0-9][a-z0-9-]*[a-z0-9]$").unwrap();
        if !regex.is_match(slug) {
            return Err(AiStudioError::validation(
                "slug", 
                "租户标识符只能包含小写字母、数字和连字符，且不能以连字符开头或结尾"
            ));
        }

        // 检查保留字
        let reserved_slugs = vec!["api", "www", "admin", "root", "system", "public", "private"];
        if reserved_slugs.contains(&slug) {
            return Err(AiStudioError::validation("slug", "该租户标识符为保留字，请选择其他标识符"));
        }

        Ok(())
    }

    /// 转换为响应格式
    async fn convert_to_response(&self, tenant: tenant::Model) -> Result<TenantResponse, AiStudioError> {
        let config = tenant.get_config().unwrap_or_default();
        let quota_limits = tenant.get_quota_limits().unwrap_or_default();
        let usage_stats = tenant.get_usage_stats().unwrap_or_default();

        Ok(TenantResponse {
            id: tenant.id,
            name: tenant.name,
            slug: tenant.slug,
            display_name: tenant.display_name,
            description: tenant.description,
            status: tenant.status,
            config,
            quota_limits,
            usage_stats,
            contact_email: tenant.contact_email,
            contact_phone: tenant.contact_phone,
            created_at: tenant.created_at.into(),
            updated_at: tenant.updated_at.into(),
            last_active_at: tenant.last_active_at.map(|dt| dt.into()),
        })
    }
}