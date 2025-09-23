// 配额管理 API 处理器

use actix_web::{web, HttpResponse, Result as ActixResult};
use uuid::Uuid;

use crate::api::extractors::AdminExtractor;
use crate::api::responses::HttpResponseBuilder;
use crate::api::middleware::tenant::TenantInfo;
use crate::api::middleware::auth::AuthenticatedUser;
use crate::services::quota::{
    QuotaService, QuotaType, QuotaUpdateRequest
};
use crate::db::DatabaseManager;
use crate::errors::AiStudioError;

/// 配额管理 API 文档
// #[derive(OpenApi)]
// #[openapi(
//     paths(
//         get_quota_stats,
//         check_quota,
//         update_quota,
//         reset_quota,
//         get_quota_trends
//     ),
//     components(schemas(
//         crate::services::quota::QuotaType,
//         crate::services::quota::QuotaUsage,
//         crate::services::quota::QuotaCheckResult,
//         crate::services::quota::QuotaUpdateRequest,
//         crate::services::quota::QuotaStatsResponse,
//         crate::services::quota::QuotaHealth,
//     ))
// )]
// pub struct QuotaApiDoc;

/// 获取租户配额统计
pub async fn get_quota_stats(
    path: web::Path<Uuid>,
    tenant_info: web::ReqData<TenantInfo>,
    user: web::ReqData<AuthenticatedUser>,
) -> ActixResult<HttpResponse> {
    let tenant_id = path.into_inner();
    
    // 检查权限：用户必须属于该租户或为管理员
    if !user.is_admin && user.tenant_id != tenant_id {
        return Err(AiStudioError::forbidden("无权访问该租户的配额信息").into());
    }

    let db_manager = DatabaseManager::get()
        .map_err(|e| AiStudioError::internal(format!("获取数据库连接失败: {}", e)))?;
    let db = db_manager.get_connection();
    let quota_service = QuotaService::new(db.clone());

    let stats = quota_service.get_quota_stats(tenant_id).await?;
    HttpResponseBuilder::ok(stats)
}

/// 检查特定配额
pub async fn check_quota(
    path: web::Path<(Uuid, String)>,
    query: web::Query<CheckQuotaQuery>,
    tenant_info: web::ReqData<TenantInfo>,
    user: web::ReqData<AuthenticatedUser>,
) -> ActixResult<HttpResponse> {
    let (tenant_id, quota_type_str) = path.into_inner();
    
    // 检查权限
    if !user.is_admin && user.tenant_id != tenant_id {
        return Err(AiStudioError::forbidden("无权访问该租户的配额信息").into());
    }

    let quota_type = parse_quota_type(&quota_type_str)?;
    let amount = query.amount.unwrap_or(1);

    let db_manager = DatabaseManager::get()
        .map_err(|e| AiStudioError::internal(format!("获取数据库连接失败: {}", e)))?;
    let db = db_manager.get_connection();
    let quota_service = QuotaService::new(db.clone());

    let result = quota_service.check_quota(tenant_id, quota_type, amount).await?;
    HttpResponseBuilder::ok(result)
}

/// 更新配额使用量
pub async fn update_quota(
    path: web::Path<Uuid>,
    request: web::Json<QuotaUpdateRequest>,
    _admin: AdminExtractor,
) -> ActixResult<HttpResponse> {
    let tenant_id = path.into_inner();

    let db_manager = DatabaseManager::get()
        .map_err(|e| AiStudioError::internal(format!("获取数据库连接失败: {}", e)))?;
    let db = db_manager.get_connection();
    let quota_service = QuotaService::new(db.clone());

    let usage = quota_service.update_quota_usage(tenant_id, request.into_inner()).await?;
    HttpResponseBuilder::ok(usage)
}

/// 重置时间相关配额
pub async fn reset_quota(
    path: web::Path<Uuid>,
    _admin: AdminExtractor,
) -> ActixResult<HttpResponse> {
    let tenant_id = path.into_inner();

    let db_manager = DatabaseManager::get()
        .map_err(|e| AiStudioError::internal(format!("获取数据库连接失败: {}", e)))?;
    let db = db_manager.get_connection();
    let quota_service = QuotaService::new(db.clone());

    quota_service.reset_time_based_quotas(tenant_id).await?;
    HttpResponseBuilder::ok(serde_json::json!({
        "message": "配额重置成功",
        "tenant_id": tenant_id,
        "reset_time": chrono::Utc::now()
    }))
}

/// 获取配额使用趋势
pub async fn get_quota_trends(
    path: web::Path<(Uuid, String)>,
    query: web::Query<TrendsQuery>,
    _tenant_info: web::ReqData<TenantInfo>,
    user: web::ReqData<AuthenticatedUser>,
) -> ActixResult<HttpResponse> {
    let (tenant_id, quota_type_str) = path.into_inner();
    
    // 检查权限
    if !user.is_admin && user.tenant_id != tenant_id {
        return Err(AiStudioError::forbidden("无权访问该租户的配额信息").into());
    }

    let quota_type = parse_quota_type(&quota_type_str)?;
    let days = query.days.unwrap_or(7);

    let db_manager = DatabaseManager::get()
        .map_err(|e| AiStudioError::internal(format!("获取数据库连接失败: {}", e)))?;
    let db = db_manager.get_connection();
    let quota_service = QuotaService::new(db.clone());

    let trends = quota_service.get_quota_trends(tenant_id, quota_type, days).await?;
    HttpResponseBuilder::ok(serde_json::json!({
        "tenant_id": tenant_id,
        "quota_type": quota_type_str,
        "days": days,
        "trends": trends
    }))
}

/// 配额检查查询参数
#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct CheckQuotaQuery {
    /// 请求数量
    pub amount: Option<u64>,
}

/// 趋势查询参数
#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct TrendsQuery {
    /// 查询天数
    pub days: Option<u32>,
}

/// 解析配额类型字符串
fn parse_quota_type(quota_type_str: &str) -> Result<QuotaType, AiStudioError> {
    match quota_type_str.to_lowercase().as_str() {
        "users" => Ok(QuotaType::Users),
        "knowledge_bases" | "knowledge-bases" => Ok(QuotaType::KnowledgeBases),
        "documents" => Ok(QuotaType::Documents),
        "storage" => Ok(QuotaType::Storage),
        "monthly_api_calls" | "monthly-api-calls" => Ok(QuotaType::MonthlyApiCalls),
        "daily_ai_queries" | "daily-ai-queries" => Ok(QuotaType::DailyAiQueries),
        _ => Err(AiStudioError::validation("quota_type", format!("无效的配额类型: {}", quota_type_str))),
    }
}

/// 配置配额路由
pub fn configure_quota_routes(cfg: &mut web::ServiceConfig) {
    use crate::api::middleware::MiddlewareConfig;
    
    cfg.service(
        web::scope("/quota")
            // 需要认证的路由
            .service(
                web::scope("")
                    .configure(MiddlewareConfig::api_standard())
                    .route("/stats", web::get().to(get_quota_stats))
                    .route("/{quota_type}/check", web::get().to(check_quota))
                    .route("/{quota_type}/trends", web::get().to(get_quota_trends))
            )
            // 管理员专用路由
            .service(
                web::scope("")
                    .configure(MiddlewareConfig::admin_only())
                    .route("/update", web::post().to(update_quota))
                    .route("/reset", web::post().to(reset_quota))
            )
    );
}