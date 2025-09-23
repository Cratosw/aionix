// 限流管理 API 处理器

use actix_web::{web, HttpResponse, Result as ActixResult};
use uuid::Uuid;

use crate::api::extractors::AdminExtractor;
use crate::api::responses::HttpResponseBuilder;
use crate::api::middleware::tenant::TenantInfo;
use crate::api::middleware::auth::{AuthenticatedUser, ApiKeyInfo};
use crate::services::rate_limit::{
    RateLimitService, RateLimitPolicy, RateLimitKeyType, RateLimitConfig, RateLimitResult
};
use crate::errors::AiStudioError;

/// 限流管理 API 文档
// #[derive(OpenApi)]
// #[openapi(
//     paths(
//         get_rate_limit_stats,
//         check_rate_limit,
//         reset_rate_limit,
//         get_rate_limit_policies
//     ),
//     components(schemas(
//         crate::services::rate_limit::RateLimitPolicy,
//         crate::services::rate_limit::RateLimitResult,
//         RateLimitStatsResponse,
//         RateLimitCheckRequest,
//     ))
// )]
// pub struct RateLimitApiDoc;

/// 获取限流统计
#[utoipa::path(
    get,
    path = "/rate-limit/stats",
    tag = "rate-limit",
    summary = "获取限流统计",
    description = "获取当前用户或 API 密钥的限流统计信息",
    responses(
        (status = 200, description = "限流统计信息", body = Vec<RateLimitStat>),
        (status = 401, description = "未认证", body = ApiError)
    )
)]
pub async fn get_rate_limits(
    user: Option<web::ReqData<AuthenticatedUser>>,
    api_key: Option<web::ReqData<ApiKeyInfo>>,
    tenant_info: Option<web::ReqData<TenantInfo>>,
) -> ActixResult<HttpResponse> {
    let rate_limit_service = create_rate_limit_service()?;
    let mut stats = Vec::new();

    // 根据认证方式获取不同的限流统计
    if let Some(user_info) = user {
        let key_type = RateLimitKeyType::User(user_info.user_id);
        let policies = get_user_policies();
        
        for policy in policies {
            match rate_limit_service.get_request_stats(key_type.clone(), &policy).await {
                Ok(result) => stats.push(RateLimitStat {
                    key_type: "user".to_string(),
                    policy_name: policy.name,
                    result,
                }),
                Err(e) => {
                    tracing::warn!("获取用户限流统计失败: {}", e);
                }
            }
        }
    }

    if let Some(api_key_info) = api_key {
        let key_type = RateLimitKeyType::ApiKey(api_key_info.key_id);
        let policies = get_api_key_policies();
        
        for policy in policies {
            match rate_limit_service.get_request_stats(key_type.clone(), &policy).await {
                Ok(result) => stats.push(RateLimitStat {
                    key_type: "api_key".to_string(),
                    policy_name: policy.name,
                    result,
                }),
                Err(e) => {
                    tracing::warn!("获取 API 密钥限流统计失败: {}", e);
                }
            }
        }
    }

    if let Some(tenant_info) = tenant_info {
        let key_type = RateLimitKeyType::Tenant(tenant_info.id);
        let policies = get_tenant_policies();
        
        for policy in policies {
            match rate_limit_service.get_request_stats(key_type.clone(), &policy).await {
                Ok(result) => stats.push(RateLimitStat {
                    key_type: "tenant".to_string(),
                    policy_name: policy.name,
                    result,
                }),
                Err(e) => {
                    tracing::warn!("获取租户限流统计失败: {}", e);
                }
            }
        }
    }

    let response = RateLimitStatsResponse {
        stats,
        timestamp: chrono::Utc::now(),
    };

    HttpResponseBuilder::ok(response)
}

/// 检查限流状态
#[utoipa::path(
    post,
    path = "/rate-limit/check",
    tag = "rate-limit",
    summary = "检查限流状态",
    description = "检查指定操作是否受到限流限制",
    request_body = RateLimitCheckRequest,
    responses(
        (status = 200, description = "限流检查结果", body = RateLimitResult),
        (status = 429, description = "请求过于频繁", body = ApiError),
        (status = 401, description = "未认证", body = ApiError)
    )
)]
pub async fn check_rate_limit(
    request: web::Json<RateLimitCheckRequest>,
    user: Option<web::ReqData<AuthenticatedUser>>,
    api_key: Option<web::ReqData<ApiKeyInfo>>,
    tenant_info: Option<web::ReqData<TenantInfo>>,
) -> ActixResult<HttpResponse> {
    let rate_limit_service = create_rate_limit_service()?;
    let req = request.into_inner();

    let key_type = match req.key_type.as_str() {
        "user" => {
            if let Some(user_info) = user {
                RateLimitKeyType::User(user_info.user_id)
            } else {
                return Err(AiStudioError::validation("auth", "需要用户认证").into());
            }
        }
        "api_key" => {
            if let Some(api_key_info) = api_key {
                RateLimitKeyType::ApiKey(api_key_info.key_id)
            } else {
                return Err(AiStudioError::validation("auth", "需要 API 密钥认证").into());
            }
        }
        "tenant" => {
            if let Some(tenant_info) = tenant_info {
                RateLimitKeyType::Tenant(tenant_info.id)
            } else {
                return Err(AiStudioError::validation("tenant", "需要租户信息").into());
            }
        }
        _ => {
            return Err(AiStudioError::validation("key_type", "无效的键类型").into());
        }
    };

    let policy = RateLimitPolicy {
        window_seconds: req.window_seconds,
        max_requests: req.max_requests,
        name: req.policy_name.unwrap_or_else(|| "custom".to_string()),
        enabled: true,
    };

    let result = rate_limit_service.check_rate_limit(key_type, &policy).await?;
    HttpResponseBuilder::ok(result)
}

/// 重置限流计数器
pub async fn reset_rate_limit(
    request: web::Json<RateLimitResetRequest>,
    _admin: AdminExtractor,
) -> ActixResult<HttpResponse> {
    let rate_limit_service = create_rate_limit_service()?;
    let req = request.into_inner();

    let key_type = match req.key_type.as_str() {
        "user" => RateLimitKeyType::User(req.key_id),
        "api_key" => RateLimitKeyType::ApiKey(req.key_id),
        "tenant" => RateLimitKeyType::Tenant(req.key_id),
        "ip" => RateLimitKeyType::Ip(req.ip_address.unwrap_or_default()),
        "global" => RateLimitKeyType::Global,
        _ => {
            return Err(AiStudioError::validation("key_type", "无效的键类型").into());
        }
    };

    let policy = RateLimitPolicy {
        window_seconds: req.window_seconds,
        max_requests: req.max_requests,
        name: req.policy_name,
        enabled: true,
    };

    rate_limit_service.reset_rate_limit(key_type, &policy).await?;
    
    HttpResponseBuilder::ok(serde_json::json!({
        "message": "限流计数器重置成功",
        "reset_time": chrono::Utc::now()
    }))
}

/// 获取限流策略
pub async fn get_rate_limit_policies() -> ActixResult<HttpResponse> {
    use crate::services::rate_limit::RateLimitPolicies;

    let policies = serde_json::json!({
        "api_key_policies": RateLimitPolicies::api_key_policies(),
        "tenant_policies": RateLimitPolicies::tenant_policies(),
        "ip_policies": RateLimitPolicies::ip_policies(),
        "global_policies": RateLimitPolicies::global_policies(),
    });

    HttpResponseBuilder::ok(policies)
}

/// 限流统计响应
#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct RateLimitStatsResponse {
    /// 统计数据
    pub stats: Vec<RateLimitStat>,
    /// 时间戳
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// 限流统计项
#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct RateLimitStat {
    /// 键类型
    pub key_type: String,
    /// 策略名称
    pub policy_name: String,
    /// 限流结果
    pub result: RateLimitResult,
}

/// 限流检查请求
#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct RateLimitCheckRequest {
    /// 键类型（user, api_key, tenant）
    pub key_type: String,
    /// 时间窗口（秒）
    pub window_seconds: u64,
    /// 最大请求数
    pub max_requests: u64,
    /// 策略名称（可选）
    pub policy_name: Option<String>,
}

/// 限流重置请求
#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct RateLimitResetRequest {
    /// 键类型
    pub key_type: String,
    /// 键 ID
    pub key_id: Uuid,
    /// IP 地址（当键类型为 ip 时）
    pub ip_address: Option<String>,
    /// 时间窗口（秒）
    pub window_seconds: u64,
    /// 最大请求数
    pub max_requests: u64,
    /// 策略名称
    pub policy_name: String,
}

// 辅助函数

/// 创建限流服务
fn create_rate_limit_service() -> Result<RateLimitService, AiStudioError> {
    let redis_url = std::env::var("REDIS_URL")
        .unwrap_or_else(|_| "redis://localhost:6379".to_string());
    
    let config = RateLimitConfig {
        redis_url,
        default_policies: vec![],
        key_prefix: "aionix".to_string(),
    };

    RateLimitService::new(config)
}

/// 获取用户限流策略
fn get_user_policies() -> Vec<RateLimitPolicy> {
    vec![
        RateLimitPolicy {
            window_seconds: 60,
            max_requests: 100,
            name: "user_per_minute".to_string(),
            enabled: true,
        },
        RateLimitPolicy {
            window_seconds: 3600,
            max_requests: 1000,
            name: "user_per_hour".to_string(),
            enabled: true,
        },
    ]
}

/// 获取 API 密钥限流策略
fn get_api_key_policies() -> Vec<RateLimitPolicy> {
    use crate::services::rate_limit::RateLimitPolicies;
    RateLimitPolicies::api_key_policies()
}

/// 获取租户限流策略
fn get_tenant_policies() -> Vec<RateLimitPolicy> {
    use crate::services::rate_limit::RateLimitPolicies;
    RateLimitPolicies::tenant_policies()
}

/// 配置限流路由
pub fn configure_rate_limit_routes(cfg: &mut web::ServiceConfig) {
    use crate::api::middleware::MiddlewareConfig;
    
    cfg.service(
        web::scope("/rate-limit")
            // 需要认证的路由
            .service(
                web::scope("")
                    .configure(MiddlewareConfig::api_standard())
                    .route("/stats", web::get().to(get_rate_limit_stats))
                    .route("/check", web::post().to(check_rate_limit))
                    .route("/policies", web::get().to(get_rate_limit_policies))
            )
            // 管理员专用路由
            .service(
                web::scope("")
                    .configure(MiddlewareConfig::admin_only())
                    .route("/reset", web::post().to(reset_rate_limit))
            )
    );
}