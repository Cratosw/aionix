// 监控管理 API 处理器

use actix_web::{web, HttpResponse, Result as ActixResult};
use utoipa::{OpenApi, ToSchema};
use uuid::Uuid;

use crate::api::extractors::{AdminExtractor, PaginationExtractor};
use crate::api::responses::HttpResponseBuilder;
use crate::api::middleware::tenant::TenantInfo;
use crate::api::middleware::auth::AuthenticatedUser;
use crate::services::monitoring::{
    MonitoringService, MetricType, MetricDataPoint, SystemHealth, TenantUsageStats
};
use crate::services::notification::{NotificationService, NotificationMessage, NotificationType};
use crate::db::DatabaseManager;
use crate::errors::AiStudioError;

/// 监控管理 API 文档
// #[derive(OpenApi)]
// #[openapi(
//     paths(
//         get_system_health,
//         get_tenant_usage_stats,
//         get_metric_trends,
//         record_metric,
//         get_notifications
//     ),
//     components(schemas(
//         crate::services::monitoring::SystemHealth,
//         crate::services::monitoring::HealthStatus,
//         crate::services::monitoring::ComponentHealth,
//         crate::services::monitoring::TenantUsageStats,
//         crate::services::monitoring::UsageMetric,
//         crate::services::monitoring::MetricType,
//         crate::services::monitoring::MetricDataPoint,
//         crate::services::notification::NotificationMessage,
//         crate::services::notification::NotificationType,
//         crate::services::notification::NotificationStatus,
//         MetricRecordRequest,
//         UsageStatsQuery,
//         TrendsQuery,
//     ))
// )]
// pub struct MonitoringApiDoc;

/// 获取系统健康状态
#[utoipa::path(
    get,
    path = "/monitoring/health",
    tag = "Monitoring",
    summary = "获取系统健康状态",
    description = "获取系统整体健康状态和各组件状态",
    responses(
        (status = 200, description = "系统健康状态", body = SystemHealth),
        (status = 403, description = "权限不足"),
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_system_health(
    _admin: AdminExtractor,
) -> ActixResult<HttpResponse> {
    let db_manager = DatabaseManager::get()
        .map_err(|e| AiStudioError::internal(format!("获取数据库连接失败: {}", e)))?;
    let db = db_manager.get_connection();
    let monitoring_service = MonitoringService::new(db.clone());

    let health = monitoring_service.get_system_health().await?;
    HttpResponseBuilder::ok(health)
}

/// 获取租户使用统计
#[utoipa::path(
    get,
    path = "/monitoring/tenants/{tenant_id}/usage",
    tag = "Monitoring",
    summary = "获取租户使用统计",
    description = "获取指定租户的详细使用统计信息",
    params(
        ("tenant_id" = Uuid, Path, description = "租户 ID"),
        ("period_hours" = u32, Query, description = "统计时间范围（小时）", example = 24)
    ),
    responses(
        (status = 200, description = "租户使用统计", body = TenantUsageStats),
        (status = 404, description = "租户不存在"),
        (status = 403, description = "权限不足"),
    ),
    security(
        ("bearer_auth" = []),
        ("api_key" = [])
    )
)]
pub async fn get_tenant_usage_stats(
    path: web::Path<Uuid>,
    query: web::Query<UsageStatsQuery>,
    tenant_info: web::ReqData<TenantInfo>,
    user: web::ReqData<AuthenticatedUser>,
) -> ActixResult<HttpResponse> {
    let tenant_id = path.into_inner();
    
    // 检查权限：用户必须属于该租户或为管理员
    if !user.is_admin && user.tenant_id != tenant_id {
        return Err(AiStudioError::forbidden("无权访问该租户的使用统计").into());
    }

    let period_hours = query.period_hours.unwrap_or(24);

    let db_manager = DatabaseManager::get()
        .map_err(|e| AiStudioError::internal(format!("获取数据库连接失败: {}", e)))?;
    let db = db_manager.get_connection();
    let monitoring_service = MonitoringService::new(db.clone());

    let stats = monitoring_service.get_tenant_usage_stats(tenant_id, period_hours).await?;
    HttpResponseBuilder::ok(stats)
}

/// 获取指标趋势
#[utoipa::path(
    get,
    path = "/monitoring/tenants/{tenant_id}/metrics/{metric_type}/trends",
    tag = "Monitoring",
    summary = "获取指标趋势",
    description = "获取指定租户特定指标类型的趋势数据",
    params(
        ("tenant_id" = Uuid, Path, description = "租户 ID"),
        ("metric_type" = MetricType, Path, description = "指标类型"),
        ("hours" = u32, Query, description = "查询时间范围（小时）", example = 24)
    ),
    responses(
        (status = 200, description = "指标趋势数据"),
        (status = 404, description = "租户不存在"),
        (status = 403, description = "权限不足"),
    ),
    security(
        ("bearer_auth" = []),
        ("api_key" = [])
    )
)]
pub async fn get_metric_trends(
    path: web::Path<(Uuid, String)>,
    query: web::Query<TrendsQuery>,
    tenant_info: web::ReqData<TenantInfo>,
    user: web::ReqData<AuthenticatedUser>,
) -> ActixResult<HttpResponse> {
    let (tenant_id, metric_type_str) = path.into_inner();
    
    // 检查权限
    if !user.is_admin && user.tenant_id != tenant_id {
        return Err(AiStudioError::forbidden("无权访问该租户的指标数据").into());
    }

    let metric_type = parse_metric_type(&metric_type_str)?;
    let hours = query.hours.unwrap_or(24);

    let db_manager = DatabaseManager::get()
        .map_err(|e| AiStudioError::internal(format!("获取数据库连接失败: {}", e)))?;
    let db = db_manager.get_connection();
    let monitoring_service = MonitoringService::new(db.clone());

    let trends = monitoring_service.get_metric_trends(tenant_id, metric_type, hours).await?;
    HttpResponseBuilder::ok(serde_json::json!({
        "tenant_id": tenant_id,
        "metric_type": metric_type_str,
        "hours": hours,
        "trends": trends
    }))
}

/// 记录指标数据
#[utoipa::path(
    post,
    path = "/monitoring/tenants/{tenant_id}/metrics",
    tag = "Monitoring",
    summary = "记录指标数据",
    description = "记录指定租户的指标数据点（管理员专用）",
    params(
        ("tenant_id" = Uuid, Path, description = "租户 ID")
    ),
    request_body = MetricRecordRequest,
    responses(
        (status = 200, description = "记录成功"),
        (status = 404, description = "租户不存在"),
        (status = 403, description = "权限不足"),
        (status = 400, description = "请求参数错误"),
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn record_metric(
    path: web::Path<Uuid>,
    request: web::Json<MetricRecordRequest>,
    _admin: AdminExtractor,
) -> ActixResult<HttpResponse> {
    let tenant_id = path.into_inner();
    let req = request.into_inner();

    let db_manager = DatabaseManager::get()
        .map_err(|e| AiStudioError::internal(format!("获取数据库连接失败: {}", e)))?;
    let db = db_manager.get_connection();
    let monitoring_service = MonitoringService::new(db.clone());

    let data_point = MetricDataPoint {
        metric_type: req.metric_type,
        value: req.value,
        timestamp: req.timestamp.unwrap_or_else(|| chrono::Utc::now()),
        labels: req.labels.unwrap_or_default(),
    };

    monitoring_service.record_metric(tenant_id, data_point).await?;
    
    HttpResponseBuilder::ok(serde_json::json!({
        "message": "指标记录成功",
        "tenant_id": tenant_id,
        "timestamp": chrono::Utc::now()
    }))
}

/// 获取通知列表
#[utoipa::path(
    get,
    path = "/monitoring/tenants/{tenant_id}/notifications",
    tag = "Monitoring",
    summary = "获取通知列表",
    description = "获取指定租户的通知消息列表",
    params(
        ("tenant_id" = Uuid, Path, description = "租户 ID"),
        ("notification_type" = Option<NotificationType>, Query, description = "通知类型过滤"),
        ("limit" = Option<u32>, Query, description = "返回数量限制", example = 50)
    ),
    responses(
        (status = 200, description = "通知列表"),
        (status = 404, description = "租户不存在"),
        (status = 403, description = "权限不足"),
    ),
    security(
        ("bearer_auth" = []),
        ("api_key" = [])
    )
)]
pub async fn get_notifications(
    path: web::Path<Uuid>,
    query: web::Query<NotificationsQuery>,
    tenant_info: web::ReqData<TenantInfo>,
    user: web::ReqData<AuthenticatedUser>,
) -> ActixResult<HttpResponse> {
    let tenant_id = path.into_inner();
    
    // 检查权限
    if !user.is_admin && user.tenant_id != tenant_id {
        return Err(AiStudioError::forbidden("无权访问该租户的通知信息").into());
    }

    // 这里应该从数据库查询通知列表
    // 为了简化，返回空列表
    let notifications: Vec<NotificationMessage> = vec![];
    
    HttpResponseBuilder::ok(serde_json::json!({
        "tenant_id": tenant_id,
        "notifications": notifications,
        "total": notifications.len(),
        "timestamp": chrono::Utc::now()
    }))
}

/// 指标记录请求
#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct MetricRecordRequest {
    /// 指标类型
    pub metric_type: MetricType,
    /// 数值
    pub value: f64,
    /// 时间戳（可选，默认为当前时间）
    pub timestamp: Option<chrono::DateTime<chrono::Utc>>,
    /// 标签（可选）
    pub labels: Option<std::collections::HashMap<String, String>>,
}

/// 使用统计查询参数
#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct UsageStatsQuery {
    /// 统计时间范围（小时）
    pub period_hours: Option<u32>,
}

/// 趋势查询参数
#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct TrendsQuery {
    /// 查询时间范围（小时）
    pub hours: Option<u32>,
}

/// 通知查询参数
#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct NotificationsQuery {
    /// 通知类型过滤
    pub notification_type: Option<NotificationType>,
    /// 返回数量限制
    pub limit: Option<u32>,
}

/// 解析指标类型字符串
fn parse_metric_type(metric_type_str: &str) -> Result<MetricType, AiStudioError> {
    match metric_type_str.to_lowercase().as_str() {
        "api_calls" | "api-calls" => Ok(MetricType::ApiCalls),
        "ai_queries" | "ai-queries" => Ok(MetricType::AiQueries),
        "storage_usage" | "storage-usage" => Ok(MetricType::StorageUsage),
        "user_activity" | "user-activity" => Ok(MetricType::UserActivity),
        "error_rate" | "error-rate" => Ok(MetricType::ErrorRate),
        "response_time" | "response-time" => Ok(MetricType::ResponseTime),
        "concurrent_connections" | "concurrent-connections" => Ok(MetricType::ConcurrentConnections),
        _ => Err(AiStudioError::validation("metric_type", format!("无效的指标类型: {}", metric_type_str))),
    }
}

/// 配置监控路由
pub fn configure_monitoring_routes(cfg: &mut web::ServiceConfig) {
    use crate::api::middleware::MiddlewareConfig;
    
    cfg.service(
        web::scope("/monitoring")
            // 管理员专用路由
            .service(
                web::scope("")
                    .configure(MiddlewareConfig::admin_only())
                    .route("/health", web::get().to(get_system_health))
                    .route("/tenants/{tenant_id}/metrics", web::post().to(record_metric))
            )
            // 需要认证的路由
            .service(
                web::scope("")
                    .configure(MiddlewareConfig::api_standard())
                    .route("/tenants/{tenant_id}/usage", web::get().to(get_tenant_usage_stats))
                    .route("/tenants/{tenant_id}/metrics/{metric_type}/trends", web::get().to(get_metric_trends))
                    .route("/tenants/{tenant_id}/notifications", web::get().to(get_notifications))
            )
    );
}