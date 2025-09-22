// 健康检查处理器

use actix_web::{web, HttpResponse, Result as ActixResult};
use utoipa::{OpenApi, ToSchema};
use chrono::Utc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::api::models::{HealthResponse, HealthStatus, DependencyHealth, SystemInfo};
use crate::api::responses::HttpResponseBuilder;
use crate::db::DatabaseManager;
use crate::errors::AiStudioError;

/// 健康检查 API 文档
// #[derive(OpenApi)]
// #[openapi(
//     paths(health_check, health_detailed),
//     components(schemas(HealthResponse, HealthStatus, DependencyHealth, SystemInfo))
// )]
// pub struct HealthApiDoc;

/// 简单健康检查
#[utoipa::path(
    get,
    path = "/health",
    tag = "Health",
    summary = "简单健康检查",
    description = "返回服务的基本健康状态",
    responses(
        (status = 200, description = "服务健康", body = HealthResponse),
        (status = 503, description = "服务不健康", body = HealthResponse)
    )
)]
pub async fn health_check() -> ActixResult<HttpResponse> {
    let health_response = HealthResponse {
        status: HealthStatus::Healthy,
        timestamp: Utc::now(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        dependencies: vec![],
        system: SystemInfo {
            uptime_seconds: get_uptime_seconds(),
            memory_usage_bytes: get_memory_usage(),
            cpu_usage_percent: get_cpu_usage(),
            active_connections: get_active_connections(),
        },
    };

    HttpResponseBuilder::ok(health_response)
}

/// 详细健康检查
#[utoipa::path(
    get,
    path = "/health/detailed",
    tag = "Health",
    summary = "详细健康检查",
    description = "返回服务及其依赖的详细健康状态",
    responses(
        (status = 200, description = "服务健康", body = HealthResponse),
        (status = 503, description = "服务不健康", body = HealthResponse)
    )
)]
pub async fn health_detailed() -> ActixResult<HttpResponse> {
    let mut dependencies = Vec::new();
    let mut overall_status = HealthStatus::Healthy;

    // 检查数据库连接
    let db_health = check_database_health().await;
    if matches!(db_health.status, HealthStatus::Unhealthy) {
        overall_status = HealthStatus::Unhealthy;
    } else if matches!(db_health.status, HealthStatus::Degraded) && matches!(overall_status, HealthStatus::Healthy) {
        overall_status = HealthStatus::Degraded;
    }
    dependencies.push(db_health);

    // 检查 Redis 连接（如果启用）
    #[cfg(feature = "redis")]
    {
        let redis_health = check_redis_health().await;
        if matches!(redis_health.status, HealthStatus::Unhealthy) {
            overall_status = HealthStatus::Unhealthy;
        } else if matches!(redis_health.status, HealthStatus::Degraded) && matches!(overall_status, HealthStatus::Healthy) {
            overall_status = HealthStatus::Degraded;
        }
        dependencies.push(redis_health);
    }

    // 检查 AI 服务连接（如果启用）
    #[cfg(feature = "ai")]
    {
        let ai_health = check_ai_service_health().await;
        if matches!(ai_health.status, HealthStatus::Unhealthy) {
            overall_status = HealthStatus::Degraded; // AI 服务不是关键依赖
        }
        dependencies.push(ai_health);
    }

    let health_response = HealthResponse {
        status: overall_status,
        timestamp: Utc::now(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        dependencies,
        system: SystemInfo {
            uptime_seconds: get_uptime_seconds(),
            memory_usage_bytes: get_memory_usage(),
            cpu_usage_percent: get_cpu_usage(),
            active_connections: get_active_connections(),
        },
    };

    let status_code = match health_response.status {
        HealthStatus::Healthy => 200,
        HealthStatus::Degraded => 200,
        HealthStatus::Unhealthy => 503,
    };

    Ok(HttpResponse::build(actix_web::http::StatusCode::from_u16(status_code).unwrap())
        .json(health_response))
}

/// 就绪检查
#[utoipa::path(
    get,
    path = "/ready",
    tag = "Health",
    summary = "就绪检查",
    description = "检查服务是否准备好接收请求",
    responses(
        (status = 200, description = "服务就绪"),
        (status = 503, description = "服务未就绪")
    )
)]
pub async fn readiness_check() -> ActixResult<HttpResponse> {
    // 检查关键依赖是否可用
    let db_health = check_database_health().await;
    
    if matches!(db_health.status, HealthStatus::Unhealthy) {
        return Ok(HttpResponse::ServiceUnavailable().json(serde_json::json!({
            "ready": false,
            "reason": "数据库连接不可用"
        })));
    }

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "ready": true
    })))
}

/// 存活检查
#[utoipa::path(
    get,
    path = "/live",
    tag = "Health",
    summary = "存活检查",
    description = "检查服务是否存活",
    responses(
        (status = 200, description = "服务存活")
    )
)]
pub async fn liveness_check() -> ActixResult<HttpResponse> {
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "alive": true,
        "timestamp": Utc::now()
    })))
}

// 私有辅助函数

/// 检查数据库健康状态
async fn check_database_health() -> DependencyHealth {
    let start_time = std::time::Instant::now();
    
    match DatabaseManager::get() {
        Ok(db_manager) => {
            match db_manager.health_check().await {
                Ok(_) => DependencyHealth {
                    name: "database".to_string(),
                    status: HealthStatus::Healthy,
                    response_time_ms: Some(start_time.elapsed().as_millis() as u64),
                    error: None,
                },
                Err(e) => DependencyHealth {
                    name: "database".to_string(),
                    status: HealthStatus::Unhealthy,
                    response_time_ms: Some(start_time.elapsed().as_millis() as u64),
                    error: Some(e.to_string()),
                },
            }
        }
        Err(e) => DependencyHealth {
            name: "database".to_string(),
            status: HealthStatus::Unhealthy,
            response_time_ms: Some(start_time.elapsed().as_millis() as u64),
            error: Some(e.to_string()),
        },
    }
}

/// 检查 Redis 健康状态
#[cfg(feature = "redis")]
async fn check_redis_health() -> DependencyHealth {
    let start_time = std::time::Instant::now();
    
    // 这里应该实现实际的 Redis 健康检查
    // 为了简化，这里返回一个模拟的健康状态
    DependencyHealth {
        name: "redis".to_string(),
        status: HealthStatus::Healthy,
        response_time_ms: Some(start_time.elapsed().as_millis() as u64),
        error: None,
    }
}

/// 检查 AI 服务健康状态
#[cfg(feature = "ai")]
async fn check_ai_service_health() -> DependencyHealth {
    let start_time = std::time::Instant::now();
    
    // 这里应该实现实际的 AI 服务健康检查
    // 为了简化，这里返回一个模拟的健康状态
    DependencyHealth {
        name: "ai_service".to_string(),
        status: HealthStatus::Healthy,
        response_time_ms: Some(start_time.elapsed().as_millis() as u64),
        error: None,
    }
}

/// 获取系统运行时间（秒）
fn get_uptime_seconds() -> u64 {
    // 这里应该实现实际的系统运行时间获取
    // 为了简化，返回一个模拟值
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// 获取内存使用量（字节）
fn get_memory_usage() -> u64 {
    // 这里应该实现实际的内存使用量获取
    // 为了简化，返回一个模拟值
    1024 * 1024 * 100 // 100MB
}

/// 获取 CPU 使用率（百分比）
fn get_cpu_usage() -> f64 {
    // 这里应该实现实际的 CPU 使用率获取
    // 为了简化，返回一个模拟值
    15.5
}

/// 获取活跃连接数
fn get_active_connections() -> u32 {
    // 这里应该实现实际的活跃连接数获取
    // 为了简化，返回一个模拟值
    42
}

/// 配置健康检查路由
pub fn configure_health_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/health")
            .route("", web::get().to(health_check))
            .route("/detailed", web::get().to(health_detailed))
    )
    .route("/ready", web::get().to(readiness_check))
    .route("/live", web::get().to(liveness_check));
}