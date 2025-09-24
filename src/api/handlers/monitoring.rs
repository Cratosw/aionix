// 监控管理 API 处理器

use actix_web::{web, HttpResponse, Result as ActixResult};
use uuid::Uuid;

use crate::api::extractors::AdminExtractor;
use crate::api::responses::HttpResponseBuilder;
use crate::api::middleware::tenant::TenantInfo;
use crate::api::middleware::auth::AuthenticatedUser;
use crate::services::monitoring::{
    MonitoringService, MetricType, MetricDataPoint
};
use crate::services::notification::{NotificationMessage, NotificationType};
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
    path = "/monitoring/system/health",
    tag = "monitoring",
    responses(
        (status = 200, description = "系统健康状态", body = SystemHealth),
        (status = 503, description = "系统不健康", body = SystemHealth)
    )
)]
pub async fn get_system_metrics(
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
    tag = "monitoring",
    params(
        ("tenant_id" = Uuid, Path, description = "租户 ID"),
        ("period_hours" = Option<i32>, Query, description = "统计周期（小时）")
    ),
    responses(
        (status = 200, description = "租户使用统计", body = TenantUsageStats),
        (status = 403, description = "无权访问", body = ApiError),
        (status = 404, description = "租户不存在", body = ApiError)
    )
)]
pub async fn get_service_status(
    path: web::Path<Uuid>,
    query: web::Query<UsageStatsQuery>,
    _tenant_info: web::ReqData<TenantInfo>,
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
pub async fn get_metric_trends(
    path: web::Path<(Uuid, String)>,
    query: web::Query<TrendsQuery>,
    _tenant_info: web::ReqData<TenantInfo>,
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
pub async fn get_notifications(
    path: web::Path<Uuid>,
    _query: web::Query<NotificationsQuery>,
    _tenant_info: web::ReqData<TenantInfo>,
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