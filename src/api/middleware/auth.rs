// 认证和授权中间件
// 处理 JWT 令牌验证、API 密钥验证和权限检查

use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpMessage, HttpResponse, Result as ActixResult,
    body::BoxBody,
};
use futures::future::{LocalBoxFuture, Ready, ready};
use jsonwebtoken::{decode, DecodingKey, Validation, Algorithm};
use serde::{Deserialize, Serialize};
use std::future::{ready as std_ready, Ready as StdReady};
use std::rc::Rc;
use uuid::Uuid;
use tracing::{info, warn, error, instrument};
use chrono::{DateTime, Utc};

use crate::db::DatabaseManager;
use crate::db::entities::{tenant, user};
use crate::errors::AiStudioError;
use crate::api::responses::ErrorResponse;
use sea_orm::{EntityTrait, ActiveModelTrait};

/// JWT 声明结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtClaims {
    /// 用户 ID
    pub sub: String,
    /// 租户 ID
    pub tenant_id: String,
    /// 用户名
    pub username: String,
    /// 用户角色
    pub role: String,
    /// 权限列表
    pub permissions: Vec<String>,
    /// 是否为管理员
    pub is_admin: bool,
    /// 签发时间
    pub iat: i64,
    /// 过期时间
    pub exp: i64,
    /// 签发者
    pub iss: String,
}

/// 用户认证信息
#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub user_id: Uuid,
    pub tenant_id: Uuid,
    pub username: String,
    pub role: String,
    pub permissions: Vec<String>,
    pub is_admin: bool,
    pub authenticated_at: DateTime<Utc>,
}

/// API 密钥信息
#[derive(Debug, Clone)]
pub struct ApiKeyInfo {
    pub key_id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
    pub permissions: Vec<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub last_used_at: Option<DateTime<Utc>>,
}

/// JWT 认证中间件
pub struct JwtAuthMiddleware {
    pub secret_key: String,
    pub required_permissions: Vec<String>,
}

impl JwtAuthMiddleware {
    pub fn new(secret_key: String) -> Self {
        Self {
            secret_key,
            required_permissions: vec![],
        }
    }

    pub fn with_permissions(secret_key: String, permissions: Vec<String>) -> Self {
        Self {
            secret_key,
            required_permissions: permissions,
        }
    }
}

impl<S, B> Transform<S, ServiceRequest> for JwtAuthMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static + actix_web::body::MessageBody,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type Transform = JwtAuthMiddlewareService<S>;
    type InitError = ();
    type Future = StdReady<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        std_ready(Ok(JwtAuthMiddlewareService {
            service,
            secret_key: self.secret_key.clone(),
            required_permissions: self.required_permissions.clone(),
        }))
    }
}

pub struct JwtAuthMiddlewareService<S> {
    service: S,
    secret_key: String,
    required_permissions: Vec<String>,
}

impl<S, B> Service<ServiceRequest> for JwtAuthMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static + actix_web::body::MessageBody,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, mut req: ServiceRequest) -> Self::Future {
        let secret_key = self.secret_key.clone();
        let required_permissions = self.required_permissions.clone();

        Box::pin(async move {
            // 提取 Authorization 头
            let auth_header = req
                .headers()
                .get("Authorization")
                .and_then(|h| h.to_str().ok());

            let token = match auth_header {
                Some(header) if header.starts_with("Bearer ") => &header[7..],
                _ => {
                    let response = HttpResponse::Unauthorized()
                        .json(ErrorResponse::unauthorized::<()>());
                    return Ok(req.into_response(response));
                }
            };

            // 验证 JWT 令牌
            match verify_jwt_token(token, &secret_key).await {
                Ok(user) => {
                    // 检查权限
                    if !required_permissions.is_empty() {
                        let has_permission = required_permissions.iter().any(|perm| {
                            user.is_admin || user.permissions.contains(perm)
                        });

                        if !has_permission {
                            let response = HttpResponse::Forbidden()
                                .json(ErrorResponse::forbidden::<()>());
                            return Ok(req.into_response(response));
                        }
                    }

                    // 将用户信息存储在请求扩展中
                    req.extensions_mut().insert(user);
                }
                Err(e) => {
                    warn!("JWT 验证失败: {}", e);
                    let response = HttpResponse::Unauthorized()
                        .json(ErrorResponse::detailed_error::<()>(
                            "INVALID_TOKEN".to_string(),
                            "无效的访问令牌".to_string(),
                            None,
                            None,
                        ));
                    return Ok(req.into_response(response));
                }
            }

            let fut = self.service.call(req);
            Ok(fut.await?.map_into_boxed_body())
        })
    }
}

/// API 密钥认证中间件
pub struct ApiKeyAuthMiddleware {
    pub required_permissions: Vec<String>,
}

impl ApiKeyAuthMiddleware {
    pub fn new() -> Self {
        Self {
            required_permissions: vec![],
        }
    }

    pub fn with_permissions(permissions: Vec<String>) -> Self {
        Self {
            required_permissions: permissions,
        }
    }
}

impl<S, B> Transform<S, ServiceRequest> for ApiKeyAuthMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static + actix_web::body::MessageBody,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type Transform = ApiKeyAuthMiddlewareService<S>;
    type InitError = ();
    type Future = StdReady<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        std_ready(Ok(ApiKeyAuthMiddlewareService {
            service,
            required_permissions: self.required_permissions.clone(),
        }))
    }
}

pub struct ApiKeyAuthMiddlewareService<S> {
    service: S,
    required_permissions: Vec<String>,
}

impl<S, B> Service<ServiceRequest> for ApiKeyAuthMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static + actix_web::body::MessageBody,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, mut req: ServiceRequest) -> Self::Future {
        let required_permissions = self.required_permissions.clone();

        Box::pin(async move {
            // 提取 API 密钥
            let api_key = req
                .headers()
                .get("X-API-Key")
                .and_then(|h| h.to_str().ok());

            let api_key = match api_key {
                Some(key) => key,
                None => {
                    let response = HttpResponse::Unauthorized()
                        .json(ErrorResponse::detailed_error::<()>(
                            "MISSING_API_KEY".to_string(),
                            "缺少 API 密钥".to_string(),
                            None,
                            None,
                        ));
                    return Ok(req.into_response(response));
                }
            };

            // 获取客户端 IP
            let client_ip = req
                .connection_info()
                .remote_addr()
                .unwrap_or("unknown")
                .to_string();

            // 验证 API 密钥
            match verify_api_key_with_ip(api_key, &client_ip).await {
                Ok(api_key_info) => {
                    // 检查权限
                    if !required_permissions.is_empty() {
                        let has_permission = required_permissions.iter().any(|perm| {
                            api_key_info.permissions.contains(perm)
                        });

                        if !has_permission {
                            let response = HttpResponse::Forbidden()
                                .json(ErrorResponse::detailed_error::<()>(
                                    "INSUFFICIENT_PERMISSIONS".to_string(),
                                    format!("API 密钥缺少必要权限: {:?}", required_permissions),
                                    None,
                                    None,
                                ));
                            return Ok(req.into_response(response));
                        }
                    }

                    // 检查速率限制
                    if let Err(e) = check_api_key_rate_limit(&api_key_info).await {
                        let response = HttpResponse::TooManyRequests()
                            .json(ErrorResponse::detailed_error::<()>(
                                "RATE_LIMIT_EXCEEDED".to_string(),
                                e.to_string(),
                                None,
                                None,
                            ));
                        return Ok(req.into_response(response));
                    }

                    // 将 API 密钥信息存储在请求扩展中
                    req.extensions_mut().insert(api_key_info);
                }
                Err(e) => {
                    warn!("API 密钥验证失败: {}", e);
                    let response = HttpResponse::Unauthorized()
                        .json(ErrorResponse::detailed_error::<()>(
                            "INVALID_API_KEY".to_string(),
                            "无效的 API 密钥".to_string(),
                            None,
                            None,
                        ));
                    return Ok(req.into_response(response));
                }
            }

            let fut = self.service.call(req);
            Ok(fut.await?.map_into_boxed_body())
        })
    }
}

/// 可选认证中间件（支持 JWT 或 API 密钥）
pub struct OptionalAuthMiddleware;

impl<S, B> Transform<S, ServiceRequest> for OptionalAuthMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static + actix_web::body::MessageBody,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type Transform = OptionalAuthMiddlewareService<S>;
    type InitError = ();
    type Future = StdReady<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        std_ready(Ok(OptionalAuthMiddlewareService { service }))
    }
}

pub struct OptionalAuthMiddlewareService<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for OptionalAuthMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static + actix_web::body::MessageBody,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, mut req: ServiceRequest) -> Self::Future {
        Box::pin(async move {
            // 尝试 JWT 认证
            if let Some(auth_header) = req.headers().get("Authorization").and_then(|h| h.to_str().ok()) {
                if auth_header.starts_with("Bearer ") {
                    let token = &auth_header[7..];
                    if let Ok(user) = verify_jwt_token(token, "default_secret").await {
                        req.extensions_mut().insert(user);
                    }
                }
            }

            // 尝试 API 密钥认证
            if let Some(api_key) = req.headers().get("X-API-Key").and_then(|h| h.to_str().ok()) {
                if let Ok(api_key_info) = verify_api_key(api_key).await {
                    req.extensions_mut().insert(api_key_info);
                }
            }

            let fut = self.service.call(req);
            Ok(fut.await?.map_into_boxed_body())
        })
    }
}

// 辅助函数

/// 验证 JWT 令牌
#[instrument(skip(token, secret_key))]
async fn verify_jwt_token(token: &str, secret_key: &str) -> Result<AuthenticatedUser, AiStudioError> {
    let decoding_key = DecodingKey::from_secret(secret_key.as_ref());
    let validation = Validation::new(Algorithm::HS256);

    let token_data = decode::<JwtClaims>(token, &decoding_key, &validation)
        .map_err(|e| AiStudioError::unauthorized(format!("JWT 解析失败: {}", e)))?;

    let claims = token_data.claims;

    // 检查令牌是否过期
    let now = Utc::now().timestamp();
    if claims.exp < now {
        return Err(AiStudioError::unauthorized("令牌已过期".to_string()));
    }

    // 解析 UUID
    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| AiStudioError::unauthorized("无效的用户 ID".to_string()))?;
    let tenant_id = Uuid::parse_str(&claims.tenant_id)
        .map_err(|_| AiStudioError::unauthorized("无效的租户 ID".to_string()))?;

    // 验证用户是否仍然存在且活跃
    verify_user_status(user_id, tenant_id).await?;

    Ok(AuthenticatedUser {
        user_id,
        tenant_id,
        username: claims.username,
        role: claims.role,
        permissions: claims.permissions,
        is_admin: claims.is_admin,
        authenticated_at: Utc::now(),
    })
}

/// 验证 API 密钥（带 IP 检查）
#[instrument(skip(api_key))]
async fn verify_api_key_with_ip(api_key: &str, client_ip: &str) -> Result<ApiKeyInfo, AiStudioError> {
    let api_key_info = verify_api_key(api_key).await?;
    
    // 检查 IP 白名单
    let db_manager = DatabaseManager::get()?;
    let db = db_manager.get_connection();
    
    use crate::db::entities::{api_key, prelude::*};
    let key_model = ApiKey::find_by_id(api_key_info.key_id)
        .one(db)
        .await?
        .ok_or_else(|| AiStudioError::not_found("API 密钥"))?;
    
    if !key_model.is_ip_allowed(client_ip)
        .map_err(|e| AiStudioError::internal(format!("检查 IP 白名单失败: {}", e)))? {
        return Err(AiStudioError::forbidden(format!("IP 地址 {} 不在白名单中", client_ip)));
    }
    
    Ok(api_key_info)
}

/// 验证 API 密钥
#[instrument(skip(api_key))]
async fn verify_api_key(api_key: &str) -> Result<ApiKeyInfo, AiStudioError> {
    use crate::db::entities::{api_key, prelude::*};
    use sea_orm::{ColumnTrait, QueryFilter};
    
    // 检查 API 密钥格式
    if !api_key.starts_with("ak_") || api_key.len() < 32 {
        return Err(AiStudioError::unauthorized("无效的 API 密钥格式".to_string()));
    }
    
    let db_manager = DatabaseManager::get()?;
    let db = db_manager.get_connection();
    
    // 查找所有活跃的 API 密钥
    let api_keys = ApiKey::find()
        .filter(api_key::Column::Status.eq(api_key::ApiKeyStatus::Active))
        .all(db)
        .await?;
    
    // 验证 API 密钥
    for key_model in api_keys {
        if let Ok(true) = crate::db::entities::api_key::ApiKeyUtils::verify_key(api_key, &key_model.key_hash) {
            // 检查是否过期
            if key_model.is_expired() {
                return Err(AiStudioError::unauthorized("API 密钥已过期".to_string()));
            }
            
            // 获取权限信息
            let permissions = key_model.get_permissions()
                .map_err(|e| AiStudioError::internal(format!("解析 API 密钥权限失败: {}", e)))?;
            
            // 更新最后使用时间（异步执行，不阻塞当前请求）
            let key_id = key_model.id;
            tokio::spawn(async move {
                if let Ok(db_manager) = DatabaseManager::get() {
                    let db = db_manager.get_connection();
                    let mut active_model: api_key::ActiveModel = key_model.into();
                    active_model.last_used_at = sea_orm::Set(Some(Utc::now().into()));
                    active_model.usage_count = sea_orm::Set(active_model.usage_count.unwrap() + 1);
                    let _ = active_model.update(db).await;
                }
            });
            
            return Ok(ApiKeyInfo {
                key_id: key_model.id,
                tenant_id: key_model.tenant_id,
                name: key_model.name,
                permissions: permissions.scopes,
                expires_at: key_model.expires_at.map(|dt| dt.into()),
                last_used_at: key_model.last_used_at.map(|dt| dt.into()),
            });
        }
    }
    
    Err(AiStudioError::unauthorized("无效的 API 密钥".to_string()))
}

/// 检查 API 密钥速率限制
#[instrument(skip(api_key_info))]
async fn check_api_key_rate_limit(api_key_info: &ApiKeyInfo) -> Result<(), AiStudioError> {
    use crate::db::entities::{api_key, prelude::*};
    
    let db_manager = DatabaseManager::get()?;
    let db = db_manager.get_connection();
    
    let key_model = ApiKey::find_by_id(api_key_info.key_id)
        .one(db)
        .await?
        .ok_or_else(|| AiStudioError::not_found("API 密钥"))?;
    
    let permissions = key_model.get_permissions()
        .map_err(|e| AiStudioError::internal(format!("解析 API 密钥权限失败: {}", e)))?;
    
    if let Some(rate_limit) = permissions.rate_limit {
        // 这里应该实现基于 Redis 的速率限制检查
        // 为了简化，这里只做基本的检查
        
        // 检查每日限制（基于使用次数的简单检查）
        let today_start = chrono::Utc::now().date_naive().and_hms_opt(0, 0, 0).unwrap();
        let today_start_utc = chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(today_start, chrono::Utc);
        
        if let Some(last_used) = key_model.last_used_at {
            let last_used_utc: chrono::DateTime<chrono::Utc> = last_used.into();
            if last_used_utc >= today_start_utc {
                // 简单的每日限制检查（实际应该用 Redis 计数器）
                if key_model.usage_count > rate_limit.requests_per_day as i64 {
                    return Err(AiStudioError::too_many_requests("API 密钥每日请求限制已达上限".to_string()));
                }
            }
        }
    }
    
    Ok(())
}

/// 验证用户状态
#[instrument]
async fn verify_user_status(user_id: Uuid, tenant_id: Uuid) -> Result<(), AiStudioError> {
    let db_manager = DatabaseManager::get()?;
    let db = db_manager.get_connection();

    // 检查租户状态
    let tenant = tenant::Entity::find_by_id(tenant_id)
        .one(db)
        .await?
        .ok_or_else(|| AiStudioError::unauthorized("租户不存在".to_string()))?;

    if tenant.status != tenant::TenantStatus::Active {
        return Err(AiStudioError::forbidden("租户已被暂停或停用".to_string()));
    }

    // 检查用户状态
    let user = user::Entity::find_by_id(user_id)
        .one(db)
        .await?
        .ok_or_else(|| AiStudioError::unauthorized("用户不存在".to_string()))?;

    if user.tenant_id != tenant_id {
        return Err(AiStudioError::forbidden("用户不属于指定租户".to_string()));
    }

    // 这里应该检查用户状态，但由于用户实体可能还没有完全实现，先跳过
    // if user.status != user::UserStatus::Active {
    //     return Err(AiStudioError::forbidden("用户已被暂停或停用".to_string()));
    // }

    Ok(())
}

/// JWT 工具函数
pub struct JwtUtils;

impl JwtUtils {
    /// 生成 JWT 令牌
    pub fn generate_token(
        user_id: Uuid,
        tenant_id: Uuid,
        username: String,
        role: String,
        permissions: Vec<String>,
        is_admin: bool,
        secret_key: &str,
        expires_in_hours: i64,
    ) -> Result<String, AiStudioError> {
        let now = Utc::now();
        let exp = now + chrono::Duration::hours(expires_in_hours);

        let claims = JwtClaims {
            sub: user_id.to_string(),
            tenant_id: tenant_id.to_string(),
            username,
            role,
            permissions,
            is_admin,
            iat: now.timestamp(),
            exp: exp.timestamp(),
            iss: "aionix-ai-studio".to_string(),
        };

        let encoding_key = jsonwebtoken::EncodingKey::from_secret(secret_key.as_ref());
        let header = jsonwebtoken::Header::new(Algorithm::HS256);

        jsonwebtoken::encode(&header, &claims, &encoding_key)
            .map_err(|e| AiStudioError::internal(format!("JWT 生成失败: {}", e)))
    }

    /// 刷新令牌
    pub fn refresh_token(
        old_token: &str,
        secret_key: &str,
        new_expires_in_hours: i64,
    ) -> Result<String, AiStudioError> {
        let decoding_key = DecodingKey::from_secret(secret_key.as_ref());
        let validation = Validation::new(Algorithm::HS256);

        let token_data = decode::<JwtClaims>(old_token, &decoding_key, &validation)
            .map_err(|e| AiStudioError::unauthorized(format!("令牌解析失败: {}", e)))?;

        let old_claims = token_data.claims;
        let now = Utc::now();
        let exp = now + chrono::Duration::hours(new_expires_in_hours);

        let new_claims = JwtClaims {
            sub: old_claims.sub,
            tenant_id: old_claims.tenant_id,
            username: old_claims.username,
            role: old_claims.role,
            permissions: old_claims.permissions,
            is_admin: old_claims.is_admin,
            iat: now.timestamp(),
            exp: exp.timestamp(),
            iss: old_claims.iss,
        };

        let encoding_key = jsonwebtoken::EncodingKey::from_secret(secret_key.as_ref());
        let header = jsonwebtoken::Header::new(Algorithm::HS256);

        jsonwebtoken::encode(&header, &new_claims, &encoding_key)
            .map_err(|e| AiStudioError::internal(format!("JWT 生成失败: {}", e)))
    }
}

/// 权限检查器
pub struct PermissionChecker;

impl PermissionChecker {
    /// 检查用户是否有指定权限
    pub fn has_permission(user: &AuthenticatedUser, permission: &str) -> bool {
        user.is_admin || user.permissions.contains(&permission.to_string())
    }

    /// 检查用户是否有任一权限
    pub fn has_any_permission(user: &AuthenticatedUser, permissions: &[String]) -> bool {
        if user.is_admin {
            return true;
        }

        permissions.iter().any(|perm| user.permissions.contains(perm))
    }

    /// 检查用户是否有所有权限
    pub fn has_all_permissions(user: &AuthenticatedUser, permissions: &[String]) -> bool {
        if user.is_admin {
            return true;
        }

        permissions.iter().all(|perm| user.permissions.contains(perm))
    }

    /// 检查用户是否属于指定租户
    pub fn belongs_to_tenant(user: &AuthenticatedUser, tenant_id: Uuid) -> bool {
        user.tenant_id == tenant_id
    }

    /// 检查用户是否有指定角色
    pub fn has_role(user: &AuthenticatedUser, role: &str) -> bool {
        user.role == role || user.is_admin
    }
}