// API 路由定义
// 定义所有 API 端点的路由配置

use actix_web::{web, HttpResponse, Result as ActixResult};
use utoipa::{OpenApi, ToSchema};

use crate::api::handlers::{self, health, version, tenant, quota, rate_limit, monitoring, auth};
use crate::api::models::*;
// use crate::api::middleware::{
//     RequestIdMiddleware, ApiVersionMiddleware, RequestLoggingMiddleware,
//     SecurityHeadersMiddleware, ResponseTimeMiddleware, ContentTypeMiddleware,
//     MiddlewareConfig,
// };
use crate::api::responses::HttpResponseBuilder;
use crate::services::tenant::{TenantResponse, TenantStatsResponse, CreateTenantRequest, UpdateTenantRequest};
use crate::services::auth::{LoginRequest, LoginResponse, RegisterRequest, RegisterResponse, RefreshTokenRequest, PasswordResetRequest, PasswordResetConfirmRequest, UserInfo};
use crate::services::quota::{QuotaCheckResult, QuotaUpdateRequest, QuotaStatsResponse};
use crate::api::handlers::rate_limit::RateLimitCheckRequest;
use crate::services::rate_limit::RateLimitPolicy;
use crate::services::monitoring::{SystemHealth};
use crate::services::tenant::TenantInfo;


/// API 文档聚合
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Aionix AI Studio API",
        description = "企业级 AI Studio API 接口文档",
        version = "1.0.0",
        contact(
            name = "Aionix Team",
            url = "https://aionix.ai",
            email = "support@aionix.ai"
        ),
        license(
            name = "MIT",
            url = "https://opensource.org/licenses/MIT"
        )
    ),
    servers(
        (url = "/api/v1", description = "API v1")
    ),
    tags(
        (name = "health", description = "健康检查端点"),
        (name = "version", description = "版本信息端点"),
        (name = "auth", description = "认证相关端点"),
        (name = "tenant", description = "租户管理端点"),
        (name = "quota", description = "配额管理端点"),
        (name = "rate-limit", description = "速率限制端点"),
        (name = "monitoring", description = "监控端点"),
    ),
    paths(merge(
        health::HealthApiDoc,
        version::VersionApiDoc,
        tenant::TenantApiDoc,
        quota::QuotaApiDoc,
        rate_limit::RateLimitApiDoc,
        monitoring::MonitoringApiDoc,
        auth::AuthApiDoc
    ))
)]
pub struct ApiDoc;

/// 根路径处理器
async fn api_root() -> ActixResult<HttpResponse> {
    let info = serde_json::json!({
        "name": "Aionix AI Studio API",
        "version": env!("CARGO_PKG_VERSION"),
        "description": "企业级 AI Studio API 接口",
        "documentation": "/api/v1/docs",
        "health": "/api/v1/health",
        "version": "/api/v1/version",
        "timestamp": chrono::Utc::now(),
        "endpoints": {
            "health": {
                "simple": "/api/v1/health",
                "detailed": "/api/v1/health/detailed",
                "ready": "/api/v1/ready",
                "live": "/api/v1/live"
            },
            "version": {
                "info": "/api/v1/version",
                "build": "/api/v1/version/build-info",
                "spec": "/api/v1/version/spec"
            },
            "docs": {
                "openapi": "/api/v1/openapi.json",
                "swagger": "/api/v1/docs"
            }
        }
    });

    HttpResponseBuilder::ok(info)
}

/// 配置 API 路由
pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            .service(
                web::scope("/v1")
                    // API 根路径
                    .route("", web::get().to(api_root))
                    // 健康检查路由
                    .configure(health::configure_health_routes)
                    // 版本信息路由
                    .configure(version::configure_version_routes)
                    // 租户管理路由
                    .configure(tenant::configure_tenant_routes)
                    // 配额管理路由
                    .configure(quota::configure_quota_routes)
                    // 限流管理路由
                    .configure(rate_limit::configure_rate_limit_routes)
                    // 监控管理路由
                    .configure(monitoring::configure_monitoring_routes)
                    // OpenAPI JSON 端点
                    .route("/openapi.json", web::get().to(get_openapi_spec))
                    // 未来的路由将在这里添加：
                    // - 租户管理 (/tenants)
                    // - 用户管理 (/users)
                    // - 认证 (/auth)
                    // - 知识库 (/knowledge-bases)
                    // - 文档 (/documents)
                    // - 问答 (/qa)
                    // - Agent (/agents)
                    // - 工作流 (/workflows)
            )
    );
}

/// 获取 OpenAPI 规范
async fn get_openapi_spec() -> ActixResult<HttpResponse> {
    let openapi = ApiDoc::openapi();
    HttpResponseBuilder::ok(openapi)
}

/// 配置 Swagger UI
pub fn configure_swagger_ui(cfg: &mut web::ServiceConfig) {
    cfg.service(
        utoipa_swagger_ui::SwaggerUi::new("/api/v1/docs/{_:.*}")
            .url("/api/v1/openapi.json", ApiDoc::openapi())
    );
}

/// API 路由配置辅助函数
pub struct ApiRouteConfig;

impl ApiRouteConfig {
    /// 配置所有 API 路由
    pub fn configure_all(cfg: &mut web::ServiceConfig) {
        // 配置主要路由
        configure_routes(cfg);
        
        // 配置 Swagger UI
        configure_swagger_ui(cfg);
    }

    /// 配置开发环境路由
    pub fn configure_dev(cfg: &mut web::ServiceConfig) {
        Self::configure_all(cfg);
        
        // 开发环境特有的路由
        cfg.service(
            web::scope("/dev")
                .route("/test", web::get().to(dev_test_endpoint))
                .route("/debug", web::get().to(dev_debug_endpoint))
        );
    }

    /// 配置生产环境路由
    pub fn configure_prod(cfg: &mut web::ServiceConfig) {
        Self::configure_all(cfg);
        
        // 生产环境可能需要额外的安全中间件
        // 这里可以添加生产环境特有的配置
    }
}

/// 开发测试端点
async fn dev_test_endpoint() -> ActixResult<HttpResponse> {
    HttpResponseBuilder::ok(serde_json::json!({
        "message": "开发测试端点",
        "timestamp": chrono::Utc::now(),
        "environment": "development"
    }))
}

/// 开发调试端点
async fn dev_debug_endpoint() -> ActixResult<HttpResponse> {
    HttpResponseBuilder::ok(serde_json::json!({
        "debug_info": {
            "version": env!("CARGO_PKG_VERSION"),
            "build_profile": if cfg!(debug_assertions) { "debug" } else { "release" },
            "features": crate::api::handlers::version::get_enabled_features(),
            "memory_usage": "模拟内存使用信息",
            "active_connections": 42,
            "uptime": "模拟运行时间"
        },
        "timestamp": chrono::Utc::now()
    }))
}
