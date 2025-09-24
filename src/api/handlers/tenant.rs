// 租户管理 API 处理器

use actix_web::{web, HttpResponse, Result as ActixResult, HttpRequest};
use uuid::Uuid;

use crate::api::extractors::AdminExtractor;
use crate::api::responses::HttpResponseBuilder;
use crate::api::models::PaginationQuery;
// use crate::api::middleware::tenant;
use crate::services::tenant::{
    TenantService, CreateTenantRequest, UpdateTenantRequest, TenantFilter
};
use crate::db::DatabaseManager;

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
#[utoipa::path(
    post,
    path = "/tenants",
    tag = "tenant",
    request_body = CreateTenantRequest,
    responses(
        (status = 201, description = "租户创建成功", body = TenantResponse),
        (status = 400, description = "请求参数错误", body = ApiError),
        (status = 409, description = "租户已存在", body = ApiError)
    )
)]
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
#[utoipa::path(
    get,
    path = "/tenants/{tenant_id}",
    tag = "tenant",
    params(
        ("tenant_id" = Uuid, Path, description = "租户 ID")
    ),
    responses(
        (status = 200, description = "租户信息", body = TenantResponse),
        (status = 404, description = "租户不存在", body = ApiError)
    )
)]
pub async fn get_tenant(
    _admin: AdminExtractor,
    path: web::Path<Uuid>,
) -> ActixResult<HttpResponse> {
    let tenant_id = path.into_inner();
    let db_manager = DatabaseManager::get()?;
    let service = TenantService::new(db_manager.get_connection().clone());

    let tenant = service.get_tenant(tenant_id).await?;

    HttpResponseBuilder::ok(tenant)
}

/// 获取租户列表
#[utoipa::path(
    get,
    path = "/tenants",
    tag = "tenant",
    params(
        PaginationQuery,
        TenantListQuery
    ),
    responses(
        (status = 200, description = "租户列表", body = PaginatedResponse<TenantResponse>)
    )
)]
pub async fn list_tenants(
    _admin: AdminExtractor,
    query: web::Query<TenantListQuery>,
    pagination: web::Query<crate::api::models::PaginationQuery>,
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
        name: query.name_search.clone(), // 修正字段名
        slug: None,
        display_name: None,
        created_after: query.created_after,
        created_before: query.created_before,
    };

    let pagination_query = crate::api::models::PaginationQuery {
        page: pagination.page,
        page_size: pagination.page_size,
        sort_by: pagination.sort_by.clone(),
        sort_order: pagination.sort_order.clone(),
    };

    let tenants = service.list_tenants(pagination_query, Some(filter)).await?;

    HttpResponseBuilder::ok(tenants)
}

/// 更新租户
#[utoipa::path(
    put,
    path = "/tenants/{tenant_id}",
    tag = "tenant",
    params(
        ("tenant_id" = Uuid, Path, description = "租户 ID")
    ),
    request_body = UpdateTenantRequest,
    responses(
        (status = 200, description = "租户更新成功", body = TenantResponse),
        (status = 404, description = "租户不存在", body = ApiError)
    )
)]
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
#[utoipa::path(
    delete,
    path = "/tenants/{tenant_id}",
    tag = "tenant",
    params(
        ("tenant_id" = Uuid, Path, description = "租户 ID")
    ),
    responses(
        (status = 204, description = "租户删除成功"),
        (status = 404, description = "租户不存在", body = ApiError)
    )
)]
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

/// 获取租户统计
#[utoipa::path(
    get,
    path = "/tenants/{tenant_id}/stats",
    tag = "tenant",
    params(
        ("tenant_id" = Uuid, Path, description = "租户 ID")
    ),
    responses(
        (status = 200, description = "租户统计信息", body = TenantStatsResponse),
        (status = 404, description = "租户不存在", body = ApiError)
    )
)]
pub async fn get_tenant_stats(
    _admin: AdminExtractor,
    path: web::Path<Uuid>,
) -> ActixResult<HttpResponse> {
    let db_manager = DatabaseManager::get()?;
    let service = TenantService::new(db_manager.get_connection().clone());

    let stats = service.get_tenant_stats().await?;

    HttpResponseBuilder::ok(stats)
}

/// 暂停租户
#[utoipa::path(
    post,
    path = "/tenants/{tenant_id}/suspend",
    tag = "tenant",
    params(
        ("tenant_id" = Uuid, Path, description = "租户 ID")
    ),
    request_body = SuspendTenantRequest,
    responses(
        (status = 200, description = "租户暂停成功", body = TenantResponse),
        (status = 404, description = "租户不存在", body = ApiError)
    )
)]
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
#[utoipa::path(
    post,
    path = "/tenants/{tenant_id}/activate",
    tag = "tenant",
    params(
        ("tenant_id" = Uuid, Path, description = "租户 ID")
    ),
    responses(
        (status = 200, description = "租户激活成功", body = TenantResponse),
        (status = 404, description = "租户不存在", body = ApiError)
    )
)]
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

/// 根据标识符获取租户详情
#[utoipa::path(
    get,
    path = "/tenants/by-slug/{slug}",
    tag = "tenant",
    params(
        ("slug" = String, Path, description = "租户标识符")
    ),
    responses(
        (status = 200, description = "租户信息", body = TenantResponse),
        (status = 404, description = "租户不存在", body = ApiError)
    )
)]
pub async fn get_tenant_by_slug(
    _req: HttpRequest,
    path: web::Path<String>,
) -> ActixResult<HttpResponse> {
    let slug = path.into_inner();
    let db_manager = DatabaseManager::get()?;
    let service = TenantService::new(db_manager.get_connection().clone());

    let tenant = service.get_tenant_by_slug(&slug).await?;

    HttpResponseBuilder::ok(tenant)
}

// 辅助结构体

/// 租户列表查询参数
#[derive(serde::Deserialize, utoipa::IntoParams)]
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
#[derive(serde::Deserialize, utoipa::IntoParams)]
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