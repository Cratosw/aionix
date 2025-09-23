// 限流中间件
// 基于 Redis 的 API 调用频率限制

use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpMessage, HttpResponse, Result as ActixResult,
    web::ServiceConfig,
};
use futures::future::{LocalBoxFuture, Ready, ready};
use std::future::{ready as std_ready, Ready as StdReady};
use std::rc::Rc;
use uuid::Uuid;
use tracing::{info, warn, error, instrument, debug};

use crate::api::middleware::tenant::TenantInfo;
use crate::api::middleware::auth::{AuthenticatedUser, ApiKeyInfo};
use crate::services::rate_limit::{
    RateLimitService, RateLimitPolicy, RateLimitKeyType, RateLimitPolicies, RateLimitConfig
};
use crate::errors::AiStudioError;
use crate::api::responses::ErrorResponse;

/// 限流中间件
#[derive(Clone)]
pub struct RateLimitMiddleware {
    /// 限流策略
    pub policies: Vec<RateLimitPolicy>,
    /// 键类型生成器
    pub key_type: RateLimitKeyType,
    /// 是否启用
    pub enabled: bool,
}

impl RateLimitMiddleware {
    /// 创建新的限流中间件
    pub fn new(policies: Vec<RateLimitPolicy>, key_type: RateLimitKeyType) -> Self {
        Self {
            policies,
            key_type,
            enabled: true,
        }
    }

    /// 创建基于租户的限流中间件
    pub fn tenant(policies: Vec<RateLimitPolicy>) -> Self {
        Self::new(policies, RateLimitKeyType::Tenant(Uuid::nil()))
    }

    /// 创建基于用户的限流中间件
    pub fn user(policies: Vec<RateLimitPolicy>) -> Self {
        Self::new(policies, RateLimitKeyType::User(Uuid::nil()))
    }

    /// 创建基于 API 密钥的限流中间件
    pub fn api_key(policies: Vec<RateLimitPolicy>) -> Self {
        Self::new(policies, RateLimitKeyType::ApiKey(Uuid::nil()))
    }

    /// 创建基于 IP 的限流中间件
    pub fn ip(policies: Vec<RateLimitPolicy>) -> Self {
        Self::new(policies, RateLimitKeyType::Ip("".to_string()))
    }

    /// 创建全局限流中间件
    pub fn global(policies: Vec<RateLimitPolicy>) -> Self {
        Self::new(policies, RateLimitKeyType::Global)
    }

    /// 创建默认的 API 密钥限流中间件
    pub fn default_api_key() -> Self {
        Self::api_key(RateLimitPolicies::api_key_policies())
    }

    /// 创建默认的租户限流中间件
    pub fn default_tenant() -> Self {
        Self::tenant(RateLimitPolicies::tenant_policies())
    }

    /// 创建默认的 IP 限流中间件
    pub fn default_ip() -> Self {
        Self::ip(RateLimitPolicies::ip_policies())
    }

    /// 创建默认的全局限流中间件
    pub fn default_global() -> Self {
        Self::global(RateLimitPolicies::global_policies())
    }

    /// 设置是否启用
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

impl<S, B> Transform<S, ServiceRequest> for RateLimitMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<actix_web::body::EitherBody<B>>;
    type Error = Error;
    type Transform = RateLimitMiddlewareService<S>;
    type InitError = ();
    type Future = StdReady<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        std_ready(Ok(RateLimitMiddlewareService {
            service: Rc::new(service),
            policies: self.policies.clone(),
            key_type: self.key_type.clone(),
            enabled: self.enabled,
        }))
    }
}

pub struct RateLimitMiddlewareService<S> {
    service: Rc<S>,
    policies: Vec<RateLimitPolicy>,
    key_type: RateLimitKeyType,
    enabled: bool,
}

impl<S, B> Service<ServiceRequest> for RateLimitMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<actix_web::body::EitherBody<B>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let policies = self.policies.clone();
        let key_type = self.key_type.clone();
        let enabled = self.enabled;

        let service = self.service.clone();
        
        Box::pin(async move {
            if !enabled {
                let fut = service.call(req);
                return fut.await.map(|res| res.map_into_left_body());
            }

            // 构建实际的键类型
            let actual_key_type = match build_actual_key_type(&key_type, &req) {
                Ok(key) => key,
                Err(e) => {
                    debug!("构建限流键失败: {}, 跳过限流检查", e);
                    let fut = service.call(req);
                    return fut.await.map(|res| res.map_into_left_body());
                }
            };

            // 检查限流
            match check_rate_limits(&actual_key_type, &policies).await {
                Ok(results) => {
                    // 检查是否有任何策略被触发
                    for result in &results {
                        if !result.allowed {
                            let mut response = HttpResponse::TooManyRequests()
                                .json(ErrorResponse::detailed_error::<()>(
                                    "RATE_LIMIT_EXCEEDED".to_string(),
                                    format!("请求频率超限: {}", result.max_requests),
                                    None,
                                    None,
                                ));

                            // 添加限流相关的响应头
                            let headers = response.headers_mut();
                            headers.insert(
                                actix_web::http::header::HeaderName::from_static("x-ratelimit-limit"),
                                actix_web::http::header::HeaderValue::from_str(&result.max_requests.to_string()).unwrap(),
                            );
                            headers.insert(
                                actix_web::http::header::HeaderName::from_static("x-ratelimit-remaining"),
                                actix_web::http::header::HeaderValue::from_str(&result.remaining_requests.to_string()).unwrap(),
                            );
                            headers.insert(
                                actix_web::http::header::HeaderName::from_static("x-ratelimit-reset"),
                                actix_web::http::header::HeaderValue::from_str(&result.reset_time.timestamp().to_string()).unwrap(),
                            );
                            if let Some(retry_after) = result.retry_after {
                                headers.insert(
                                    actix_web::http::header::RETRY_AFTER,
                                    actix_web::http::header::HeaderValue::from_str(&retry_after.to_string()).unwrap(),
                                );
                            }

                            return Ok(req.into_response(response).map_into_right_body());
                        }
                    }
                }
                Err(e) => {
                    error!("限流检查失败: {}", e);
                    // 限流检查失败时，允许请求通过，但记录错误
                }
            }

            let fut = service.call(req);
            fut.await.map(|res| res.map_into_left_body())
        })
    }
}

/// 组合限流中间件（支持多种限流策略）
#[derive(Clone)]
pub struct CompositeRateLimitMiddleware {
    /// 多个限流中间件
    pub middlewares: Vec<RateLimitMiddleware>,
}

impl CompositeRateLimitMiddleware {
    /// 创建组合限流中间件
    pub fn new(middlewares: Vec<RateLimitMiddleware>) -> Self {
        Self { middlewares }
    }

    /// 创建标准的组合限流中间件
    pub fn standard() -> Self {
        Self::new(vec![
            RateLimitMiddleware::default_global(),
            RateLimitMiddleware::default_ip(),
            RateLimitMiddleware::default_tenant(),
            RateLimitMiddleware::default_api_key(),
        ])
    }

    /// 创建轻量级的组合限流中间件
    pub fn lightweight() -> Self {
        Self::new(vec![
            RateLimitMiddleware::default_ip(),
            RateLimitMiddleware::default_api_key(),
        ])
    }
}

impl<S, B> Transform<S, ServiceRequest> for CompositeRateLimitMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<actix_web::body::EitherBody<B>>;
    type Error = Error;
    type Transform = CompositeRateLimitMiddlewareService<S>;
    type InitError = ();
    type Future = StdReady<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        std_ready(Ok(CompositeRateLimitMiddlewareService {
            service: Rc::new(service),
            middlewares: self.middlewares.clone(),
        }))
    }
}

pub struct CompositeRateLimitMiddlewareService<S> {
    service: Rc<S>,
    middlewares: Vec<RateLimitMiddleware>,
}

impl<S, B> Service<ServiceRequest> for CompositeRateLimitMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<actix_web::body::EitherBody<B>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let middlewares = self.middlewares.clone();
        let service = self.service.clone();

        Box::pin(async move {
            // 依次检查所有限流策略
            for middleware in &middlewares {
                if !middleware.enabled {
                    continue;
                }

                let actual_key_type = match build_actual_key_type(&middleware.key_type, &req) {
                    Ok(key) => key,
                    Err(_) => continue,
                };

                match check_rate_limits(&actual_key_type, &middleware.policies).await {
                    Ok(results) => {
                        for result in &results {
                            if !result.allowed {
                                let response = HttpResponse::TooManyRequests()
                                    .json(ErrorResponse::detailed_error::<()>(
                                        "RATE_LIMIT_EXCEEDED".to_string(),
                                        format!("请求频率超限: {}", result.max_requests),
                                        None,
                                        None,
                                    ));
                                return Ok(req.into_response(response).map_into_right_body());
                            }
                        }
                    }
                    Err(e) => {
                        error!("限流检查失败: {}", e);
                    }
                }
            }

            let fut = service.call(req);
            fut.await.map(|res| res.map_into_left_body())
        })
    }
}

// 辅助函数

/// 构建实际的键类型
fn build_actual_key_type(
    key_type: &RateLimitKeyType,
    req: &ServiceRequest,
) -> Result<RateLimitKeyType, AiStudioError> {
    match key_type {
        RateLimitKeyType::Tenant(_) => {
            if let Some(tenant_info) = req.extensions().get::<TenantInfo>() {
                Ok(RateLimitKeyType::Tenant(tenant_info.id))
            } else {
                Err(AiStudioError::validation("rate_limit", "缺少租户信息"))
            }
        }
        RateLimitKeyType::User(_) => {
            if let Some(user) = req.extensions().get::<AuthenticatedUser>() {
                Ok(RateLimitKeyType::User(user.user_id))
            } else {
                Err(AiStudioError::validation("rate_limit", "缺少用户信息"))
            }
        }
        RateLimitKeyType::ApiKey(_) => {
            if let Some(api_key) = req.extensions().get::<ApiKeyInfo>() {
                Ok(RateLimitKeyType::ApiKey(api_key.key_id))
            } else {
                Err(AiStudioError::validation("rate_limit", "缺少 API 密钥信息"))
            }
        }
        RateLimitKeyType::Ip(_) => {
            let ip = req
                .connection_info()
                .remote_addr()
                .unwrap_or("unknown")
                .to_string();
            Ok(RateLimitKeyType::Ip(ip))
        }
        RateLimitKeyType::Global => Ok(RateLimitKeyType::Global),
        RateLimitKeyType::Custom(key) => Ok(RateLimitKeyType::Custom(key.clone())),
    }
}

/// 检查限流
#[instrument(skip(key_type, policies))]
async fn check_rate_limits(
    key_type: &RateLimitKeyType,
    policies: &[RateLimitPolicy],
) -> Result<Vec<crate::services::rate_limit::RateLimitResult>, AiStudioError> {
    // 创建限流服务（这里应该从配置或依赖注入获取）
    let config = RateLimitConfig {
        redis_url: std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string()),
        default_policies: vec![],
        key_prefix: "aionix".to_string(),
    };

    let rate_limit_service = RateLimitService::new(config)
        .map_err(|e| AiStudioError::internal(format!("创建限流服务失败: {}", e)))?;

    rate_limit_service.batch_check_rate_limits(key_type.clone(), policies).await
}

/// 限流中间件配置辅助函数
pub struct RateLimitMiddlewareConfig;

impl RateLimitMiddlewareConfig {
    /// 获取标准限流中间件
    pub fn standard() -> CompositeRateLimitMiddleware {
        CompositeRateLimitMiddleware::standard()
    }

    /// 获取轻量级限流中间件
    pub fn lightweight() -> CompositeRateLimitMiddleware {
        CompositeRateLimitMiddleware::lightweight()
    }

    /// 获取 API 密钥限流中间件
    pub fn api_key_only() -> RateLimitMiddleware {
        RateLimitMiddleware::default_api_key()
    }

    /// 获取租户限流中间件
    pub fn tenant_only() -> RateLimitMiddleware {
        RateLimitMiddleware::default_tenant()
    }

    /// 获取 IP 限流中间件
    pub fn ip_only() -> RateLimitMiddleware {
        RateLimitMiddleware::default_ip()
    }

    /// 获取全局限流中间件
    pub fn global_only() -> RateLimitMiddleware {
        RateLimitMiddleware::default_global()
    }

    /// 获取自定义限流中间件
    pub fn custom(
        policies: Vec<RateLimitPolicy>,
        key_type: RateLimitKeyType,
    ) -> RateLimitMiddleware {
        RateLimitMiddleware::new(policies, key_type)
    }
}