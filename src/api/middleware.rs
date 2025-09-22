// API 中间件
// 定义 API 相关的中间件，包括请求日志、CORS、限流等

pub mod auth;
pub mod tenant;
pub mod access_control;
pub mod quota;
pub mod rate_limit;

pub use auth::*;
pub use tenant::*;
pub use access_control::*;
pub use quota::*;
pub use rate_limit::*;

use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpMessage, HttpResponse,
};
use futures::future::{LocalBoxFuture, Ready, ready};
use std::future::{ready as std_ready, Ready as StdReady};
use std::rc::Rc;
use uuid::Uuid;
use tracing::{info, warn, error, instrument};
use chrono::Utc;

/// 请求 ID 中间件
pub struct RequestIdMiddleware;

impl<S, B> Transform<S, ServiceRequest> for RequestIdMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = RequestIdMiddlewareService<S>;
    type InitError = ();
    type Future = StdReady<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        std_ready(Ok(RequestIdMiddlewareService { service }))
    }
}

pub struct RequestIdMiddlewareService<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for RequestIdMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, mut req: ServiceRequest) -> Self::Future {
        // 获取或生成请求 ID
        let request_id = req
            .headers()
            .get("X-Request-ID")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string())
            .unwrap_or_else(|| Uuid::new_v4().to_string());

        // 将请求 ID 存储在请求扩展中
        req.extensions_mut().insert(RequestId(request_id.clone()));

        let fut = self.service.call(req);

        Box::pin(async move {
            let mut res = fut.await?;
            
            // 在响应头中添加请求 ID
            res.headers_mut().insert(
                actix_web::http::header::HeaderName::from_static("x-request-id"),
                actix_web::http::header::HeaderValue::from_str(&request_id).unwrap(),
            );

            Ok(res)
        })
    }
}

/// 请求 ID 包装器
#[derive(Debug, Clone)]
pub struct RequestId(pub String);

/// API 版本中间件
pub struct ApiVersionMiddleware {
    pub version: String,
}

impl ApiVersionMiddleware {
    pub fn new(version: String) -> Self {
        Self { version }
    }
}

impl<S, B> Transform<S, ServiceRequest> for ApiVersionMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = ApiVersionMiddlewareService<S>;
    type InitError = ();
    type Future = StdReady<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        std_ready(Ok(ApiVersionMiddlewareService {
            service,
            version: self.version.clone(),
        }))
    }
}

pub struct ApiVersionMiddlewareService<S> {
    service: S,
    version: String,
}

impl<S, B> Service<ServiceRequest> for ApiVersionMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let version = self.version.clone();
        let fut = self.service.call(req);

        Box::pin(async move {
            let mut res = fut.await?;
            
            // 在响应头中添加 API 版本
            res.headers_mut().insert(
                actix_web::http::header::HeaderName::from_static("x-api-version"),
                actix_web::http::header::HeaderValue::from_str(&version).unwrap(),
            );

            Ok(res)
        })
    }
}

/// 请求日志中间件
pub struct RequestLoggingMiddleware;

impl<S, B> Transform<S, ServiceRequest> for RequestLoggingMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = RequestLoggingMiddlewareService<S>;
    type InitError = ();
    type Future = StdReady<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        std_ready(Ok(RequestLoggingMiddlewareService { service }))
    }
}

pub struct RequestLoggingMiddlewareService<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for RequestLoggingMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let start_time = std::time::Instant::now();
        let method = req.method().to_string();
        let path = req.path().to_string();
        let query = req.query_string().to_string();
        let user_agent = req
            .headers()
            .get("User-Agent")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("unknown")
            .to_string();
        let remote_addr = req
            .connection_info()
            .remote_addr()
            .unwrap_or("unknown")
            .to_string();

        // 获取请求 ID
        let request_id = req
            .extensions()
            .get::<RequestId>()
            .map(|r| r.0.clone())
            .unwrap_or_else(|| "unknown".to_string());

        let fut = self.service.call(req);

        Box::pin(async move {
            let res = fut.await;
            let duration = start_time.elapsed();

            match &res {
                Ok(response) => {
                    let status = response.status().as_u16();
                    
                    if status >= 400 {
                        warn!(
                            request_id = %request_id,
                            method = %method,
                            path = %path,
                            query = %query,
                            status = status,
                            duration_ms = duration.as_millis(),
                            remote_addr = %remote_addr,
                            user_agent = %user_agent,
                            "HTTP request completed with error"
                        );
                    } else {
                        info!(
                            request_id = %request_id,
                            method = %method,
                            path = %path,
                            query = %query,
                            status = status,
                            duration_ms = duration.as_millis(),
                            remote_addr = %remote_addr,
                            user_agent = %user_agent,
                            "HTTP request completed"
                        );
                    }
                }
                Err(err) => {
                    error!(
                        request_id = %request_id,
                        method = %method,
                        path = %path,
                        query = %query,
                        duration_ms = duration.as_millis(),
                        remote_addr = %remote_addr,
                        user_agent = %user_agent,
                        error = %err,
                        "HTTP request failed"
                    );
                }
            }

            res
        })
    }
}

/// 安全头中间件
pub struct SecurityHeadersMiddleware;

impl<S, B> Transform<S, ServiceRequest> for SecurityHeadersMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = SecurityHeadersMiddlewareService<S>;
    type InitError = ();
    type Future = StdReady<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        std_ready(Ok(SecurityHeadersMiddlewareService { service }))
    }
}

pub struct SecurityHeadersMiddlewareService<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for SecurityHeadersMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let fut = self.service.call(req);

        Box::pin(async move {
            let mut res = fut.await?;
            
            let headers = res.headers_mut();
            
            // 添加安全头
            headers.insert(
                actix_web::http::header::HeaderName::from_static("x-content-type-options"),
                actix_web::http::header::HeaderValue::from_static("nosniff"),
            );
            headers.insert(
                actix_web::http::header::HeaderName::from_static("x-frame-options"),
                actix_web::http::header::HeaderValue::from_static("DENY"),
            );
            headers.insert(
                actix_web::http::header::HeaderName::from_static("x-xss-protection"),
                actix_web::http::header::HeaderValue::from_static("1; mode=block"),
            );
            headers.insert(
                actix_web::http::header::HeaderName::from_static("strict-transport-security"),
                actix_web::http::header::HeaderValue::from_static("max-age=31536000; includeSubDomains"),
            );
            headers.insert(
                actix_web::http::header::HeaderName::from_static("referrer-policy"),
                actix_web::http::header::HeaderValue::from_static("strict-origin-when-cross-origin"),
            );

            Ok(res)
        })
    }
}

/// 响应时间中间件
pub struct ResponseTimeMiddleware;

impl<S, B> Transform<S, ServiceRequest> for ResponseTimeMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = ResponseTimeMiddlewareService<S>;
    type InitError = ();
    type Future = StdReady<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        std_ready(Ok(ResponseTimeMiddlewareService { service }))
    }
}

pub struct ResponseTimeMiddlewareService<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for ResponseTimeMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let start_time = std::time::Instant::now();
        let fut = self.service.call(req);

        Box::pin(async move {
            let mut res = fut.await?;
            let duration = start_time.elapsed();
            
            // 在响应头中添加响应时间
            res.headers_mut().insert(
                actix_web::http::header::HeaderName::from_static("x-response-time"),
                actix_web::http::header::HeaderValue::from_str(&format!("{}ms", duration.as_millis())).unwrap(),
            );

            Ok(res)
        })
    }
}

/// 内容类型验证中间件
pub struct ContentTypeMiddleware {
    pub allowed_types: Vec<String>,
}

impl ContentTypeMiddleware {
    pub fn json_only() -> Self {
        Self {
            allowed_types: vec!["application/json".to_string()],
        }
    }

    pub fn new(allowed_types: Vec<String>) -> Self {
        Self { allowed_types }
    }
}

impl<S, B> Transform<S, ServiceRequest> for ContentTypeMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = ContentTypeMiddlewareService<S>;
    type InitError = ();
    type Future = StdReady<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        std_ready(Ok(ContentTypeMiddlewareService {
            service,
            allowed_types: self.allowed_types.clone(),
        }))
    }
}

pub struct ContentTypeMiddlewareService<S> {
    service: S,
    allowed_types: Vec<String>,
}

impl<S, B> Service<ServiceRequest> for ContentTypeMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        // 只对有 body 的请求检查内容类型
        if matches!(req.method(), &actix_web::http::Method::POST | &actix_web::http::Method::PUT | &actix_web::http::Method::PATCH) {
            let content_type = req
                .headers()
                .get("Content-Type")
                .and_then(|h| h.to_str().ok())
                .unwrap_or("");

            let is_allowed = self.allowed_types.iter().any(|allowed| {
                content_type.starts_with(allowed)
            });

            if !is_allowed {
                let response = HttpResponse::BadRequest()
                    .json(crate::api::responses::ErrorResponse::error::<()>(
                        "INVALID_CONTENT_TYPE".to_string(),
                        format!("不支持的内容类型: {}，支持的类型: {:?}", content_type, self.allowed_types),
                    ));
                
                return Box::pin(async move {
                    Ok(req.into_response(response))
                });
            }
        }

        let fut = self.service.call(req);
        Box::pin(async move { fut.await })
    }
}

/// 中间件配置辅助函数
pub fn configure_api_middleware() -> Vec<Box<dyn Fn(&mut actix_web::dev::ServiceConfig)>> {
    vec![
        Box::new(|cfg| {
            cfg.wrap(RequestIdMiddleware);
        }),
        Box::new(|cfg| {
            cfg.wrap(ApiVersionMiddleware::new(env!("CARGO_PKG_VERSION").to_string()));
        }),
        Box::new(|cfg| {
            cfg.wrap(RequestLoggingMiddleware);
        }),
        Box::new(|cfg| {
            cfg.wrap(SecurityHeadersMiddleware);
        }),
        Box::new(|cfg| {
            cfg.wrap(ResponseTimeMiddleware);
        }),
    ]
}

/// 中间件配置器
pub struct MiddlewareConfig;

impl MiddlewareConfig {
    /// 配置标准 API 中间件栈（需要认证和租户）
    pub fn api_standard() -> impl Fn(&mut actix_web::dev::ServiceConfig) {
        |cfg| {
            cfg.wrap(AccessControlMiddleware::api_standard());
        }
    }

    /// 配置管理员 API 中间件栈
    pub fn admin_only() -> impl Fn(&mut actix_web::dev::ServiceConfig) {
        |cfg| {
            cfg.wrap(AccessControlMiddleware::admin_only());
        }
    }

    /// 配置公开 API 中间件栈（不需要认证）
    pub fn public() -> impl Fn(&mut actix_web::dev::ServiceConfig) {
        |cfg| {
            cfg.wrap(AccessControlMiddleware::public());
        }
    }

    /// 配置带权限要求的 API 中间件栈
    pub fn with_permissions(permissions: Vec<String>) -> impl Fn(&mut actix_web::dev::ServiceConfig) {
        move |cfg| {
            cfg.wrap(AccessControlMiddleware::with_permissions(permissions.clone()));
        }
    }

    /// 配置带角色要求的 API 中间件栈
    pub fn with_roles(roles: Vec<String>) -> impl Fn(&mut actix_web::dev::ServiceConfig) {
        move |cfg| {
            cfg.wrap(AccessControlMiddleware::with_roles(roles.clone()));
        }
    }

    /// 配置 JWT 认证中间件
    pub fn jwt_auth(secret_key: String) -> impl Fn(&mut actix_web::dev::ServiceConfig) {
        move |cfg| {
            cfg.wrap(JwtAuthMiddleware::new(secret_key.clone()));
        }
    }

    /// 配置 API 密钥认证中间件
    pub fn api_key_auth() -> impl Fn(&mut actix_web::dev::ServiceConfig) {
        |cfg| {
            cfg.wrap(ApiKeyAuthMiddleware::new());
        }
    }

    /// 配置租户识别中间件
    pub fn tenant_identification() -> impl Fn(&mut actix_web::dev::ServiceConfig) {
        |cfg| {
            cfg.wrap(TenantIdentificationMiddleware::default());
            cfg.wrap(TenantIsolationMiddleware);
        }
    }

    /// 配置完整的中间件栈（基础 + 访问控制 + 配额 + 限流）
    pub fn full_stack() -> Vec<Box<dyn Fn(&mut actix_web::dev::ServiceConfig)>> {
        let mut middleware = configure_api_middleware();
        middleware.push(Box::new(Self::api_standard()));
        middleware.push(Box::new(QuotaMiddlewareConfig::api_calls()));
        // middleware.push(Box::new(RateLimitMiddlewareConfig::lightweight()));
        middleware
    }

    /// 配置管理员完整中间件栈
    pub fn admin_full_stack() -> Vec<Box<dyn Fn(&mut actix_web::dev::ServiceConfig)>> {
        let mut middleware = configure_api_middleware();
        middleware.push(Box::new(Self::admin_only()));
        middleware
    }

    /// 配置公开完整中间件栈
    pub fn public_full_stack() -> Vec<Box<dyn Fn(&mut actix_web::dev::ServiceConfig)>> {
        let mut middleware = configure_api_middleware();
        middleware.push(Box::new(Self::public()));
        middleware
    }

    /// 配置 AI 查询中间件栈
    pub fn ai_query_stack() -> Vec<Box<dyn Fn(&mut actix_web::dev::ServiceConfig)>> {
        let mut middleware = configure_api_middleware();
        middleware.push(Box::new(Self::api_standard()));
        middleware.push(Box::new(QuotaMiddlewareConfig::ai_queries()));
        // middleware.push(Box::new(RateLimitMiddlewareConfig::api_key_only()));
        middleware
    }

    /// 配置文件上传中间件栈
    pub fn file_upload_stack(file_size_bytes: u64) -> Vec<Box<dyn Fn(&mut actix_web::dev::ServiceConfig)>> {
        let mut middleware = configure_api_middleware();
        middleware.push(Box::new(Self::api_standard()));
        middleware.push(Box::new(QuotaMiddlewareConfig::storage(file_size_bytes)));
        // middleware.push(Box::new(RateLimitMiddlewareConfig::lightweight()));
        middleware
    }
}