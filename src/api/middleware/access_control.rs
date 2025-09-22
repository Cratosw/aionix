// 综合访问控制中间件
// 结合认证、授权、租户隔离和权限检查的统一中间件

use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpMessage, HttpResponse, Result as ActixResult,
    body::BoxBody,
};
use futures::future::{LocalBoxFuture, Ready, ready};
use std::future::{ready as std_ready, Ready as StdReady};
use std::rc::Rc;
use uuid::Uuid;
use tracing::{info, warn, error, instrument, debug};
use serde::{Deserialize, Serialize};
use sea_orm::EntityTrait;

use crate::api::middleware::{
    auth::{AuthenticatedUser, ApiKeyInfo, JwtAuthMiddleware, ApiKeyAuthMiddleware},
    tenant::{TenantInfo, TenantIdentificationMiddleware, TenantIdentificationStrategy},
};
use crate::errors::AiStudioError;
use crate::api::responses::ErrorResponse;

/// 访问控制策略
#[derive(Debug, Clone)]
pub struct AccessControlPolicy {
    /// 是否需要认证
    pub require_auth: bool,
    /// 支持的认证方式
    pub auth_methods: Vec<AuthMethod>,
    /// 是否需要租户识别
    pub require_tenant: bool,
    /// 租户识别策略
    pub tenant_strategy: TenantIdentificationStrategy,
    /// 必需的权限
    pub required_permissions: Vec<String>,
    /// 必需的角色
    pub required_roles: Vec<String>,
    /// 是否检查配额
    pub check_quota: bool,
    /// 是否检查 IP 白名单
    pub check_ip_whitelist: bool,
    /// 是否启用速率限制
    pub enable_rate_limit: bool,
}

/// 认证方式枚举
#[derive(Debug, Clone, PartialEq)]
pub enum AuthMethod {
    /// JWT 令牌
    Jwt,
    /// API 密钥
    ApiKey,
    /// 可选认证（允许匿名访问）
    Optional,
}

/// 访问控制上下文
#[derive(Debug, Clone)]
pub struct AccessControlContext {
    /// 认证用户信息
    pub user: Option<AuthenticatedUser>,
    /// API 密钥信息
    pub api_key: Option<ApiKeyInfo>,
    /// 租户信息
    pub tenant: Option<TenantInfo>,
    /// 客户端 IP
    pub client_ip: String,
    /// 用户代理
    pub user_agent: Option<String>,
    /// 请求路径
    pub request_path: String,
    /// 请求方法
    pub request_method: String,
}

/// 综合访问控制中间件
pub struct AccessControlMiddleware {
    pub policy: AccessControlPolicy,
}

impl AccessControlMiddleware {
    /// 创建新的访问控制中间件
    pub fn new(policy: AccessControlPolicy) -> Self {
        Self { policy }
    }

    /// 创建标准的 API 访问控制中间件
    pub fn api_standard() -> Self {
        Self {
            policy: AccessControlPolicy {
                require_auth: true,
                auth_methods: vec![AuthMethod::Jwt, AuthMethod::ApiKey],
                require_tenant: true,
                tenant_strategy: TenantIdentificationStrategy::Combined(vec![
                    TenantIdentificationStrategy::Header,
                    TenantIdentificationStrategy::Subdomain,
                ]),
                required_permissions: vec![],
                required_roles: vec![],
                check_quota: true,
                check_ip_whitelist: true,
                enable_rate_limit: true,
            },
        }
    }

    /// 创建管理员访问控制中间件
    pub fn admin_only() -> Self {
        Self {
            policy: AccessControlPolicy {
                require_auth: true,
                auth_methods: vec![AuthMethod::Jwt],
                require_tenant: true,
                tenant_strategy: TenantIdentificationStrategy::Header,
                required_permissions: vec!["admin".to_string()],
                required_roles: vec!["admin".to_string()],
                check_quota: false,
                check_ip_whitelist: true,
                enable_rate_limit: false,
            },
        }
    }

    /// 创建公开访问中间件（仅租户识别）
    pub fn public() -> Self {
        Self {
            policy: AccessControlPolicy {
                require_auth: false,
                auth_methods: vec![AuthMethod::Optional],
                require_tenant: false,
                tenant_strategy: TenantIdentificationStrategy::Combined(vec![
                    TenantIdentificationStrategy::Header,
                    TenantIdentificationStrategy::Subdomain,
                ]),
                required_permissions: vec![],
                required_roles: vec![],
                check_quota: false,
                check_ip_whitelist: false,
                enable_rate_limit: false,
            },
        }
    }

    /// 创建带权限要求的访问控制中间件
    pub fn with_permissions(permissions: Vec<String>) -> Self {
        let mut middleware = Self::api_standard();
        middleware.policy.required_permissions = permissions;
        middleware
    }

    /// 创建带角色要求的访问控制中间件
    pub fn with_roles(roles: Vec<String>) -> Self {
        let mut middleware = Self::api_standard();
        middleware.policy.required_roles = roles;
        middleware
    }
}

impl<S, B> Transform<S, ServiceRequest> for AccessControlMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + Clone,
    S::Future: 'static,
    B: 'static + actix_web::body::MessageBody,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type Transform = AccessControlMiddlewareService<S>;
    type InitError = ();
    type Future = StdReady<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        std_ready(Ok(AccessControlMiddlewareService {
            service,
            policy: self.policy.clone(),
        }))
    }
}

pub struct AccessControlMiddlewareService<S> {
    service: S,
    policy: AccessControlPolicy,
}

impl<S, B> Service<ServiceRequest> for AccessControlMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + Clone,
    S::Future: 'static,
    B: 'static + actix_web::body::MessageBody,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service = self.service.clone();
        let policy = self.policy.clone();

        Box::pin(async move {
            // 构建访问控制上下文
            let context = AccessControlContext {
                user: req.extensions().get::<AuthenticatedUser>().cloned(),
                api_key: req.extensions().get::<ApiKeyInfo>().cloned(),
                tenant: req.extensions().get::<TenantInfo>().cloned(),
                client_ip: req
                    .connection_info()
                    .remote_addr()
                    .unwrap_or("unknown")
                    .to_string(),
                user_agent: req
                    .headers()
                    .get("User-Agent")
                    .and_then(|h| h.to_str().ok())
                    .map(|s| s.to_string()),
                request_path: req.path().to_string(),
                request_method: req.method().to_string(),
            };

            // 执行访问控制检查
            if let Err(e) = perform_access_control_checks(&policy, &context).await {
                let response = match e.status_code() {
                    401 => HttpResponse::Unauthorized().json(ErrorResponse::detailed_error::<()>(
                        e.error_code().to_string(),
                        e.to_string(),
                        None,
                        None,
                    )),
                    403 => HttpResponse::Forbidden().json(ErrorResponse::detailed_error::<()>(
                        e.error_code().to_string(),
                        e.to_string(),
                        None,
                        None,
                    )),
                    429 => HttpResponse::TooManyRequests().json(ErrorResponse::detailed_error::<()>(
                        e.error_code().to_string(),
                        e.to_string(),
                        None,
                        None,
                    )),
                    _ => HttpResponse::BadRequest().json(ErrorResponse::detailed_error::<()>(
                        e.error_code().to_string(),
                        e.to_string(),
                        None,
                        None,
                    )),
                };
                return Ok(req.into_response(response));
            }

            // 将访问控制上下文存储到请求扩展中
            req.extensions_mut().insert(context);

            let fut = self.service.call(req);
            Ok(fut.await?.map_into_boxed_body())
        })
    }
}

/// 执行访问控制检查
#[instrument(skip(policy, context))]
async fn perform_access_control_checks(
    policy: &AccessControlPolicy,
    context: &AccessControlContext,
) -> Result<(), AiStudioError> {
    // 1. 认证检查
    if policy.require_auth {
        check_authentication(policy, context).await?;
    }

    // 2. 租户检查
    if policy.require_tenant {
        check_tenant_access(context).await?;
    }

    // 3. 权限检查
    if !policy.required_permissions.is_empty() {
        check_permissions(policy, context).await?;
    }

    // 4. 角色检查
    if !policy.required_roles.is_empty() {
        check_roles(policy, context).await?;
    }

    // 5. IP 白名单检查
    if policy.check_ip_whitelist {
        check_ip_whitelist(context).await?;
    }

    // 6. 配额检查
    if policy.check_quota {
        check_quota_limits(context).await?;
    }

    // 7. 速率限制检查
    if policy.enable_rate_limit {
        check_rate_limits(context).await?;
    }

    Ok(())
}

/// 检查认证
async fn check_authentication(
    policy: &AccessControlPolicy,
    context: &AccessControlContext,
) -> Result<(), AiStudioError> {
    let has_jwt = context.user.is_some();
    let has_api_key = context.api_key.is_some();

    if policy.auth_methods.contains(&AuthMethod::Optional) {
        return Ok(());
    }

    if policy.auth_methods.contains(&AuthMethod::Jwt) && has_jwt {
        return Ok(());
    }

    if policy.auth_methods.contains(&AuthMethod::ApiKey) && has_api_key {
        return Ok(());
    }

    Err(AiStudioError::unauthorized("需要有效的认证凭据"))
}

/// 检查租户访问权限
async fn check_tenant_access(context: &AccessControlContext) -> Result<(), AiStudioError> {
    let tenant = context.tenant.as_ref()
        .ok_or_else(|| AiStudioError::forbidden("缺少租户信息"))?;

    // 检查用户是否属于租户
    if let Some(user) = &context.user {
        if !user.is_admin && user.tenant_id != tenant.id {
            return Err(AiStudioError::forbidden("用户不属于当前租户"));
        }
    }

    // 检查 API 密钥是否属于租户
    if let Some(api_key) = &context.api_key {
        if api_key.tenant_id != tenant.id {
            return Err(AiStudioError::forbidden("API 密钥不属于当前租户"));
        }
    }

    Ok(())
}

/// 检查权限
async fn check_permissions(
    policy: &AccessControlPolicy,
    context: &AccessControlContext,
) -> Result<(), AiStudioError> {
    let user_permissions = if let Some(user) = &context.user {
        if user.is_admin {
            return Ok(()); // 管理员拥有所有权限
        }
        user.permissions.clone()
    } else if let Some(api_key) = &context.api_key {
        api_key.permissions.clone()
    } else {
        vec![]
    };

    for required_permission in &policy.required_permissions {
        if !user_permissions.contains(required_permission) {
            return Err(AiStudioError::forbidden(format!(
                "缺少必要权限: {}",
                required_permission
            )));
        }
    }

    Ok(())
}

/// 检查角色
async fn check_roles(
    policy: &AccessControlPolicy,
    context: &AccessControlContext,
) -> Result<(), AiStudioError> {
    if let Some(user) = &context.user {
        if user.is_admin {
            return Ok(()); // 管理员拥有所有角色
        }

        if !policy.required_roles.contains(&user.role) {
            return Err(AiStudioError::forbidden(format!(
                "需要角色: {:?}，当前角色: {}",
                policy.required_roles, user.role
            )));
        }
    } else {
        return Err(AiStudioError::forbidden("需要用户角色验证"));
    }

    Ok(())
}

/// 检查 IP 白名单
async fn check_ip_whitelist(context: &AccessControlContext) -> Result<(), AiStudioError> {
    // 如果有 API 密钥，检查其 IP 白名单
    if let Some(api_key) = &context.api_key {
        use crate::db::entities::{api_key, prelude::*};
        use crate::db::DatabaseManager;

        let db_manager = DatabaseManager::get()?;
        let db = db_manager.get_connection();

        let key_model = ApiKey::find_by_id(api_key.key_id)
            .one(db)
            .await?
            .ok_or_else(|| AiStudioError::not_found("API 密钥"))?;

        if !key_model.is_ip_allowed(&context.client_ip)
            .map_err(|e| AiStudioError::internal(format!("检查 IP 白名单失败: {}", e)))? {
            return Err(AiStudioError::forbidden(format!(
                "IP 地址 {} 不在白名单中",
                context.client_ip
            )));
        }
    }

    Ok(())
}

/// 检查配额限制
async fn check_quota_limits(context: &AccessControlContext) -> Result<(), AiStudioError> {
    if let Some(tenant) = &context.tenant {
        use crate::db::entities::{tenant as tenant_entity, prelude::*};
        use crate::db::DatabaseManager;

        let db_manager = DatabaseManager::get()?;
        let db = db_manager.get_connection();

        let tenant_model = Tenant::find_by_id(tenant.id)
            .one(db)
            .await?
            .ok_or_else(|| AiStudioError::not_found("租户"))?;

        // 检查 API 调用配额
        if context.request_path.starts_with("/api/") {
            if tenant_model.is_quota_exceeded("monthly_api_calls")
                .map_err(|e| AiStudioError::internal(format!("检查配额失败: {}", e)))? {
                return Err(AiStudioError::quota_exceeded("月度 API 调用配额已用完"));
            }
        }

        // 检查 AI 查询配额
        if context.request_path.contains("/ai/") 
            || context.request_path.contains("/chat/") 
            || context.request_path.contains("/qa/") {
            if tenant_model.is_quota_exceeded("daily_ai_queries")
                .map_err(|e| AiStudioError::internal(format!("检查配额失败: {}", e)))? {
                return Err(AiStudioError::quota_exceeded("每日 AI 查询配额已用完"));
            }
        }

        // 检查存储配额（对于文件上传请求）
        if matches!(context.request_method.as_str(), "POST" | "PUT") 
            && (context.request_path.contains("/upload") || context.request_path.contains("/documents")) {
            if tenant_model.is_quota_exceeded("storage")
                .map_err(|e| AiStudioError::internal(format!("检查配额失败: {}", e)))? {
                return Err(AiStudioError::quota_exceeded("存储配额已用完"));
            }
        }
    }

    Ok(())
}

/// 检查速率限制
async fn check_rate_limits(context: &AccessControlContext) -> Result<(), AiStudioError> {
    // 这里应该实现基于 Redis 的速率限制
    // 为了简化，这里只做基本的检查
    
    if let Some(api_key) = &context.api_key {
        use crate::db::entities::{api_key, prelude::*};
        use crate::db::DatabaseManager;

        let db_manager = DatabaseManager::get()?;
        let db = db_manager.get_connection();

        let key_model = ApiKey::find_by_id(api_key.key_id)
            .one(db)
            .await?
            .ok_or_else(|| AiStudioError::not_found("API 密钥"))?;

        let permissions = key_model.get_permissions()
            .map_err(|e| AiStudioError::internal(format!("解析 API 密钥权限失败: {}", e)))?;

        if let Some(rate_limit) = permissions.rate_limit {
            // 简单的速率限制检查（实际应该用 Redis 实现）
            let now = chrono::Utc::now();
            if let Some(last_used) = key_model.last_used_at {
                let last_used_utc: chrono::DateTime<chrono::Utc> = last_used.into();
                let time_diff = (now - last_used_utc).num_seconds();
                
                // 如果上次使用时间在 1 分钟内，检查是否超过每分钟限制
                if time_diff < 60 && key_model.usage_count > rate_limit.requests_per_minute as i64 {
                    return Err(AiStudioError::too_many_requests("API 密钥每分钟请求限制已达上限"));
                }
            }
        }
    }

    Ok(())
}

impl Default for AccessControlPolicy {
    fn default() -> Self {
        Self {
            require_auth: true,
            auth_methods: vec![AuthMethod::Jwt],
            require_tenant: true,
            tenant_strategy: TenantIdentificationStrategy::Header,
            required_permissions: vec![],
            required_roles: vec![],
            check_quota: true,
            check_ip_whitelist: false,
            enable_rate_limit: true,
        }
    }
}