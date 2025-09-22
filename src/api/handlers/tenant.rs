// 租户管理 API 处理器

use actix_web::{web, HttpResponse, Result as ActixResult};
use utoipa::{OpenApi, ToSchema};
use uuid::Uuid;

use crate::api::extractors::{AdminExtractor, PaginationExtractor, SearchExtractor};
use crate::api::responses::HttpResponseBuilder;
use crate::api::models::PaginationQuery;
use crate::services::tenant::{
    TenantService, CreateTenantRequest, UpdateTenantRequest, TenantFilter, 
    TenantResponse, TenantStatsResponse
};
use crate::db::DatabaseManager;
use crate::errors::AiStudioError;

/// 租户管理 API 文档
// #[derive(OpenApi)]
// #[openapi(
//     paths(
//         create_tenant,
//         get_tenant,
//         get_tenant_by_slug,
//         update_tenant,
//         delete_tenant,
//         list_tenants,
//         get_tenant_stats,
//         suspend_tenant,
//         activate_tenant,
//         check_tenant_quota
//     ),
//     components(schemas(
//         CreateTenantRequest,
//         UpdateTenantRequest,
//         TenantResponse,
//         TenantStatsResponse,
//         crate::db::entities::tenant::TenantStatus,
//         crate::db::entities::tenant::TenantConfig,
//         crate::db::entities::tenant::TenantFeatures,
//         crate::db::entities::tenant::TenantQuotaLimits,
//         crate::db::entities::tenant::TenantUsageStats,
//     ))
// )]
// pub struct TenantApiDoc;

/// 创建租户
pub async fn create_tenant(
    _admin: AdminExtractor,
    request: web::Json<CreateTenantRequest>,
) -> ActixResult<HttpResponse> {
    let db_manager = DatabaseManager::get()?;
    let service = TenantService::new(db_manager.get_connection().clone());

    let tenant = service.create_tenant(request.into_inner()).await?;

    HttpResponseBuilder::created(tenant)
}

/// 获取租户详情
pub async fn get_tenant(
    _admin: AdminExtractor,
    path: web::Path<Uuid>,
) -> ActixResult<HttpResponse> {
    let tenant_id = path.into_inner();
    let db_manager = DatabaseManager::get()?;
    let service = TenantService::new(db_manager.get_connection().clone());

    let tenant = service.get_tenant_by_id(tenant_id).await?;

    HttpResponseBuilder::ok(tenant)
}

/// 根据标识符获取租户
pub async fn get_tenant_by_slug(
    _admin: AdminExtractor,
    path: web::Path<String>,
) -> ActixResult<HttpResponse> {
    let slug = path.into_inner();
    let db_manager = DatabaseManager::get()?;
    let service = TenantService::new(db_manager.get_connection().clone());

    let tenant = service.get_tenant_by_slug(&slug).await?;

    HttpResponseBuilder::ok(tenant)
}

/// 更新租户
pub async fn update_tenant(
    _admin: AdminExtractor,
    path: web::Path<Uuid>,
    request: web::Json<UpdateTenantRequest>,
) -> ActixResult<HttpResponse> {
    let tenant_id = path.into_inner();
    let db_manager = DatabaseManager::get()?;
    let service = TenantService::new(db_manager.get_connection().clone());

    let tenant = service.update_tenant(tenant_id, request.into_inner()).await?;

    HttpResponseBuilder::ok(tenant)
}

/// 删除租户
pub async fn delete_tenant(
    _admin: AdminExtractor,
    path: web::Path<Uuid>,
) -> ActixResult<HttpResponse> {
    let tenant_id = path.into_inner();
    let db_manager = DatabaseManager::get()?;
    let service = TenantService::new(db_manager.get_connection().clone());

    service.delete_tenant(tenant_id).await?;

    HttpResponseBuilder::no_content()
}

/// 列出租户
pub async fn list_tenants(
    _admin: AdminExtractor,
    pagination: PaginationExtractor,
    query: web::Query<TenantListQuery>,
) -> ActixResult<HttpResponse> {
    let db_manager = DatabaseManager::get()?;
    let service = TenantService::new(db_manager.get_connection().clone());

    let filter = TenantFilter {
        status: query.status.as_ref().and_then(|s| match s.as_str() {
            "active" => Some(crate::db::entities::tenant::TenantStatus::Active),
            "suspended" => Some(crate::db::entities::tenant::TenantStatus::Suspended),
            "inactive" => Some(crate::db::entities::tenant::TenantStatus::Inactive),
            _ => None,
        }),
        name_search: query.name_search.clone(),
        created_after: query.created_after,
        created_before: query.created_before,
    };

    let pagination_query = PaginationQuery {
        page: pagination.page,
        page_size: pagination.page_size,
        sort_by: pagination.sort_by,
        sort_order: match pagination.sort_order.as_str() {
            "asc" => crate::api::models::SortOrder::Asc,
            _ => crate::api::models::SortOrder::Desc,
        },
    };

    let tenants = service.list_tenants(Some(filter), pagination_query).await?;

    HttpResponseBuilder::ok(tenants)
}

/// 获取租户统计
pub async fn get_tenant_stats(
    _admin: AdminExtractor,
) -> ActixResult<HttpResponse> {
    let db_manager = DatabaseManager::get()?;
    let service = TenantService::new(db_manager.get_connection().clone());

    let stats = service.get_tenant_stats().await?;

    HttpResponseBuilder::ok(stats)
}

/// 暂停租户
pub async fn suspend_tenant(
    _admin: AdminExtractor,
    path: web::Path<Uuid>,
    request: web::Json<SuspendTenantRequest>,
) -> ActixResult<HttpResponse> {
    let tenant_id = path.into_inner();
    let db_manager = DatabaseManager::get()?;
    let service = TenantService::new(db_manager.get_connection().clone());

    let tenant = service.suspend_tenant(tenant_id, request.reason.clone()).await?;

    HttpResponseBuilder::ok(tenant)
}

/// 激活租户
pub async fn activate_tenant(
    _admin: AdminExtractor,
    path: web::Path<Uuid>,
) -> ActixResult<HttpResponse> {
    let tenant_id = path.into_inner();
    let db_manager = DatabaseManager::get()?;
    let service = TenantService::new(db_manager.get_connection().clone());

    let tenant = service.activate_tenant(tenant_id).await?;

    HttpResponseBuilder::ok(tenant)
}

/// 检查租户配额
pub async fn check_tenant_quota(
    _admin: AdminExtractor,
    path: web::Path<(Uuid, String)>,
    query: web::Query<QuotaCheckQuery>,
) -> ActixResult<HttpResponse> {
    let (tenant_id, resource_type) = path.into_inner();
    let requested_amount = query.requested_amount.unwrap_or(1);
    
    let db_manager = DatabaseManager::get()?;
    let service = TenantService::new(db_manager.get_connection().clone());

    let can_allocate = service.check_tenant_quota(tenant_id, &resource_type, requested_amount).await?;

    let response = QuotaCheckResponse {
        tenant_id,
        resource_type,
        requested_amount,
        can_allocate,
        checked_at: chrono::Utc::now(),
    };

    HttpResponseBuilder::ok(response)
}

// 辅助结构体

/// 租户列表查询参数
#[derive(serde::Deserialize)]
pub struct TenantListQuery {
    pub status: Option<String>,
    pub name_search: Option<String>,
    pub created_after: Option<chrono::DateTime<chrono::Utc>>,
    pub created_before: Option<chrono::DateTime<chrono::Utc>>,
}

/// 暂停租户请求
#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct SuspendTenantRequest {
    /// 暂停原因
    pub reason: Option<String>,
}

/// 配额检查查询参数
#[derive(serde::Deserialize)]
pub struct QuotaCheckQuery {
    pub requested_amount: Option<i64>,
}

/// 配额检查响应
#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct QuotaCheckResponse {
    pub tenant_id: Uuid,
    pub resource_type: String,
    pub requested_amount: i64,
    pub can_allocate: bool,
    pub checked_at: chrono::DateTime<chrono::Utc>,
}

/// 配置租户路由
pub fn configure_tenant_routes(cfg: &mut web::ServiceConfig) {
    use crate::api::middleware::MiddlewareConfig;
    
    cfg.service(
        web::scope("/tenants")
            // 管理员权限的路由
            .service(
                web::scope("")
                    .configure(MiddlewareConfig::admin_only())
                    .route("", web::post().to(create_tenant))
                    .route("", web::get().to(list_tenants))
                    .route("/stats", web::get().to(get_tenant_stats))
                    .route("/{tenant_id}", web::put().to(update_tenant))
                    .route("/{tenant_id}", web::delete().to(delete_tenant))
                    .route("/{tenant_id}/suspend", web::post().to(suspend_tenant))
                    .route("/{tenant_id}/activate", web::post().to(activate_tenant))
            )
            // 标准认证的路由
            .service(
                web::scope("")
                    .configure(MiddlewareConfig::api_standard())
                    .route("/by-slug/{slug}", web::get().to(get_tenant_by_slug))
                    .route("/{tenant_id}", web::get().to(get_tenant))
                    .route("/{tenant_id}/quota/{resource_type}", web::get().to(check_tenant_quota))
            )
    );
}