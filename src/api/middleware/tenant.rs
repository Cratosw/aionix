// 租户识别中间件
// 从请求头、子域名或路径参数中提取租户信息

use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpMessage, HttpResponse,
    body::BoxBody, web::ServiceConfig,
};
use futures::future::LocalBoxFuture;
use std::future::{ready as std_ready, Ready as StdReady};
use uuid::Uuid;
use tracing::{warn, instrument, debug};
use sea_orm::{EntityTrait, ColumnTrait, QueryFilter};

use crate::db::DatabaseManager;
use crate::db::entities::{tenant, prelude::*};
use crate::db::migrations::tenant_filter::TenantContext;
use crate::errors::AiStudioError;
use crate::api::responses::ErrorResponse;

/// 租户识别策略
#[derive(Debug, Clone)]
pub enum TenantIdentificationStrategy {
    /// 从请求头 X-Tenant-ID 获取
    Header,
    /// 从子域名获取
    Subdomain,
    /// 从路径参数获取
    PathParam,
    /// 从查询参数获取
    QueryParam,
    /// 组合策略（按优先级尝试）
    Combined(Vec<TenantIdentificationStrategy>),
}

/// 租户信息
#[derive(Debug, Clone)]
pub struct TenantInfo {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub display_name: String,
    pub status: tenant::TenantStatus,
    pub context: TenantContext,
}

/// 租户识别中间件
pub struct TenantIdentificationMiddleware {
    pub strategy: TenantIdentificationStrategy,
    pub required: bool,
}

impl TenantIdentificationMiddleware {
    /// 创建新的租户识别中间件
    pub fn new(strategy: TenantIdentificationStrategy) -> Self {
        Self {
            strategy,
            required: true,
        }
    }

    /// 创建可选的租户识别中间件
    pub fn optional(strategy: TenantIdentificationStrategy) -> Self {
        Self {
            strategy,
            required: false,
        }
    }

    /// 创建默认的租户识别中间件（组合策略）
    pub fn default() -> Self {
        Self {
            strategy: TenantIdentificationStrategy::Combined(vec![
                TenantIdentificationStrategy::Header,
                TenantIdentificationStrategy::Subdomain,
                TenantIdentificationStrategy::QueryParam,
            ]),
            required: true,
        }
    }
}

impl<S, B> Transform<S, ServiceRequest> for TenantIdentificationMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + Clone + 'static,
    S::Future: 'static,
    B: 'static + actix_web::body::MessageBody,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type Transform = TenantIdentificationMiddlewareService<S>;
    type InitError = ();
    type Future = StdReady<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        std_ready(Ok(TenantIdentificationMiddlewareService {
            service,
            strategy: self.strategy.clone(),
            required: self.required,
        }))
    }
}

pub struct TenantIdentificationMiddlewareService<S> {
    service: S,
    strategy: TenantIdentificationStrategy,
    required: bool,
}

impl<S, B> Service<ServiceRequest> for TenantIdentificationMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + Clone + 'static,
    S::Future: 'static,
    B: 'static + actix_web::body::MessageBody,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service = self.service.clone();
        let strategy = self.strategy.clone();
        let required = self.required;

        Box::pin(async move {
            match identify_tenant(&req, &strategy).await {
                Ok(Some(tenant_info)) => {
                    debug!(
                        tenant_id = %tenant_info.id,
                        tenant_slug = %tenant_info.slug,
                        "租户识别成功"
                    );

                    // 检查租户状态
                    if tenant_info.status != tenant::TenantStatus::Active {
                        let response = HttpResponse::Forbidden()
                            .json(ErrorResponse::detailed_error::<()>(
                                "TENANT_INACTIVE".to_string(),
                                "租户已被暂停或停用".to_string(),
                                None,
                                None,
                            ));
                        return Ok(req.into_response(response));
                    }

                    // 将租户信息存储在请求扩展中
                    req.extensions_mut().insert(tenant_info);
                }
                Ok(None) => {
                    if required {
                        let response = HttpResponse::BadRequest()
                            .json(ErrorResponse::detailed_error::<()>(
                                "TENANT_REQUIRED".to_string(),
                                "无法识别租户信息".to_string(),
                                None,
                                None,
                            ));
                        return Ok(req.into_response(response));
                    }
                }
                Err(e) => {
                    warn!("租户识别失败: {}", e);
                    if required {
                        let response = HttpResponse::BadRequest()
                            .json(ErrorResponse::detailed_error::<()>(
                                "TENANT_IDENTIFICATION_FAILED".to_string(),
                                e.to_string(),
                                None,
                                None,
                            ));
                        return Ok(req.into_response(response));
                    }
                }
            }

            let fut = service.call(req);
            Ok(fut.await?.map_into_boxed_body())
        })
    }
}

/// 租户数据隔离中间件
pub struct TenantIsolationMiddleware;

impl<S, B> Transform<S, ServiceRequest> for TenantIsolationMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + Clone + 'static,
    S::Future: 'static,
    B: 'static + actix_web::body::MessageBody,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type Transform = TenantIsolationMiddlewareService<S>;
    type InitError = ();
    type Future = StdReady<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        std_ready(Ok(TenantIsolationMiddlewareService { service }))
    }
}

pub struct TenantIsolationMiddlewareService<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for TenantIsolationMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + Clone + 'static,
    S::Future: 'static,
    B: 'static + actix_web::body::MessageBody,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service = self.service.clone();
        Box::pin(async move {
            // 验证租户数据隔离
            let tenant_info = req.extensions().get::<TenantInfo>().cloned();
            if let Some(tenant_info) = tenant_info {
                // 检查租户配额限制
                if let Err(e) = check_tenant_quota_limits(&tenant_info, &req).await {
                    let response = HttpResponse::TooManyRequests()
                        .json(ErrorResponse::detailed_error::<()>(
                            "QUOTA_EXCEEDED".to_string(),
                            e.to_string(),
                            None,
                            None,
                        ));
                    return Ok(req.into_response(response));
                }

                // 验证用户租户归属
                let auth_user = req.extensions().get::<crate::api::middleware::auth::AuthenticatedUser>().cloned();
                if let Some(auth_user) = auth_user {
                    // 检查用户是否属于当前租户
                    if !auth_user.is_admin && auth_user.tenant_id != tenant_info.id {
                        let response = HttpResponse::Forbidden()
                            .json(ErrorResponse::detailed_error::<()>(
                                "TENANT_MISMATCH".to_string(),
                                "用户不属于当前租户".to_string(),
                                None,
                                None,
                            ));
                        return Ok(req.into_response(response));
                    }
                }

                // 验证 API 密钥租户归属
                let api_key_info = req.extensions().get::<crate::api::middleware::auth::ApiKeyInfo>().cloned();
                if let Some(api_key_info) = api_key_info {
                    if api_key_info.tenant_id != tenant_info.id {
                        let response = HttpResponse::Forbidden()
                            .json(ErrorResponse::detailed_error::<()>(
                                "API_KEY_TENANT_MISMATCH".to_string(),
                                "API 密钥不属于当前租户".to_string(),
                                None,
                                None,
                            ));
                        return Ok(req.into_response(response));
                    }
                }

                // 设置租户上下文到请求扩展中
                req.extensions_mut().insert(tenant_info.context.clone());
            }

            let fut = service.call(req);
            Ok(fut.await?.map_into_boxed_body())
        })
    }
}

// 辅助函数

/// 识别租户
#[instrument(skip(req))]
async fn identify_tenant(
    req: &ServiceRequest,
    strategy: &TenantIdentificationStrategy,
) -> Result<Option<TenantInfo>, AiStudioError> {
    match strategy {
        TenantIdentificationStrategy::Header => {
            identify_tenant_from_header(req).await
        }
        TenantIdentificationStrategy::Subdomain => {
            identify_tenant_from_subdomain(req).await
        }
        TenantIdentificationStrategy::PathParam => {
            identify_tenant_from_path_param(req).await
        }
        TenantIdentificationStrategy::QueryParam => {
            identify_tenant_from_query_param(req).await
        }
        TenantIdentificationStrategy::Combined(strategies) => {
            for s in strategies {
                let res = match s {
                    TenantIdentificationStrategy::Header => identify_tenant_from_header(req).await?,
                    TenantIdentificationStrategy::Subdomain => identify_tenant_from_subdomain(req).await?,
                    TenantIdentificationStrategy::PathParam => identify_tenant_from_path_param(req).await?,
                    TenantIdentificationStrategy::QueryParam => identify_tenant_from_query_param(req).await?,
                    TenantIdentificationStrategy::Combined(_) => None,
                };
                if res.is_some() { return Ok(res); }
            }
            Ok(None)
        }
    }
}

/// 从请求头识别租户
async fn identify_tenant_from_header(req: &ServiceRequest) -> Result<Option<TenantInfo>, AiStudioError> {
    let tenant_id_header = req.headers().get("X-Tenant-ID").and_then(|h| h.to_str().ok());
    let tenant_slug_header = req.headers().get("X-Tenant-Slug").and_then(|h| h.to_str().ok());

    if let Some(tenant_id_str) = tenant_id_header {
        let tenant_id = Uuid::parse_str(tenant_id_str)
            .map_err(|_| AiStudioError::validation("tenant_id", "无效的租户 ID 格式"))?;
        
        return get_tenant_by_id(tenant_id).await.map(Some);
    }

    if let Some(tenant_slug) = tenant_slug_header {
        return get_tenant_by_slug(tenant_slug).await.map(Some);
    }

    Ok(None)
}

/// 从子域名识别租户
async fn identify_tenant_from_subdomain(req: &ServiceRequest) -> Result<Option<TenantInfo>, AiStudioError> {
    let host = req
        .headers()
        .get("Host")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");

    let subdomain = extract_subdomain(host);

    if let Some(subdomain) = subdomain {
        // 跳过常见的系统子域名
        if matches!(subdomain.as_str(), "www" | "api" | "admin" | "app" | "dashboard") {
            return Ok(None);
        }

        return get_tenant_by_slug(&subdomain).await.map(Some);
    }

    Ok(None)
}

/// 从路径参数识别租户
async fn identify_tenant_from_path_param(req: &ServiceRequest) -> Result<Option<TenantInfo>, AiStudioError> {
    // 尝试从路径中提取租户信息
    // 例如：/tenants/{tenant_id}/... 或 /{tenant_slug}/...
    let path = req.path();
    
    // 匹配 /tenants/{tenant_id} 模式
    if let Some(captures) = regex::Regex::new(r"/tenants/([0-9a-f-]{36})")
        .unwrap()
        .captures(path)
    {
        if let Some(tenant_id_str) = captures.get(1) {
            let tenant_id = Uuid::parse_str(tenant_id_str.as_str())
                .map_err(|_| AiStudioError::validation("tenant_id", "无效的租户 ID 格式"))?;
            
            return get_tenant_by_id(tenant_id).await.map(Some);
        }
    }

    // 匹配 /{tenant_slug}/... 模式（第一个路径段）
    let path_segments: Vec<&str> = path.trim_start_matches('/').split('/').collect();
    if !path_segments.is_empty() && !path_segments[0].is_empty() {
        let potential_slug = path_segments[0];
        
        // 跳过 API 路径
        if matches!(potential_slug, "api" | "health" | "docs" | "openapi.json") {
            return Ok(None);
        }

        // 尝试作为租户标识符查询
        if let Ok(tenant_info) = get_tenant_by_slug(potential_slug).await {
            return Ok(Some(tenant_info));
        }
    }

    Ok(None)
}

/// 从查询参数识别租户
async fn identify_tenant_from_query_param(req: &ServiceRequest) -> Result<Option<TenantInfo>, AiStudioError> {
    let query_string = req.query_string();
    let params: std::collections::HashMap<String, String> = 
        serde_urlencoded::from_str(query_string).unwrap_or_default();

    if let Some(tenant_id_str) = params.get("tenant_id") {
        let tenant_id = Uuid::parse_str(tenant_id_str)
            .map_err(|_| AiStudioError::validation("tenant_id", "无效的租户 ID 格式"))?;
        
        return get_tenant_by_id(tenant_id).await.map(Some);
    }

    if let Some(tenant_slug) = params.get("tenant_slug") {
        return get_tenant_by_slug(tenant_slug).await.map(Some);
    }

    Ok(None)
}

/// 根据 ID 获取租户信息
async fn get_tenant_by_id(tenant_id: Uuid) -> Result<TenantInfo, AiStudioError> {
    let db_manager = DatabaseManager::get()?;
    let db = db_manager.get_connection();

    let tenant = Tenant::find_by_id(tenant_id)
        .one(db)
        .await?
        .ok_or_else(|| AiStudioError::not_found("租户"))?;

    Ok(TenantInfo {
        id: tenant.id,
        slug: tenant.slug.clone(),
        name: tenant.name.clone(),
        display_name: tenant.display_name.clone(),
        status: tenant.status.clone(),
        context: TenantContext::new(tenant.id, tenant.slug, false),
    })
}

/// 根据标识符获取租户信息
pub async fn get_tenant_by_slug(slug: &str) -> Result<TenantInfo, AiStudioError> {
    let db_manager = DatabaseManager::get()?;
    let db = db_manager.get_connection();

    let tenant = Tenant::find()
        .filter(tenant::Column::Slug.eq(slug))
        .one(db)
        .await?
        .ok_or_else(|| AiStudioError::not_found("租户"))?;

    Ok(TenantInfo {
        id: tenant.id,
        slug: tenant.slug.clone(),
        name: tenant.name.clone(),
        display_name: tenant.display_name.clone(),
        status: tenant.status.clone(),
        context: TenantContext::new(tenant.id, tenant.slug, false),
    })
}

/// 从主机名提取子域名
fn extract_subdomain(host: &str) -> Option<String> {
    // 移除端口号
    let host = host.split(':').next().unwrap_or(host);
    
    let parts: Vec<&str> = host.split('.').collect();
    
    // 至少需要 3 个部分才有子域名（subdomain.domain.tld）
    if parts.len() >= 3 {
        Some(parts[0].to_string())
    } else {
        None
    }
}

/// 检查租户配额限制
#[instrument(skip(tenant_info, req))]
async fn check_tenant_quota_limits(tenant_info: &TenantInfo, req: &ServiceRequest) -> Result<(), AiStudioError> {
    let db_manager = DatabaseManager::get()?;
    let db = db_manager.get_connection();
    
    let tenant = Tenant::find_by_id(tenant_info.id)
        .one(db)
        .await?
        .ok_or_else(|| AiStudioError::not_found("租户"))?;
    
    // 检查 API 调用配额
    let path = req.path();
    if path.starts_with("/api/") {
        if tenant.is_quota_exceeded("monthly_api_calls")
            .map_err(|e| AiStudioError::internal(format!("检查配额失败: {}", e)))? {
            return Err(AiStudioError::quota_exceeded("月度 API 调用配额已用完".to_string()));
        }
    }
    
    // 检查 AI 查询配额
    if path.contains("/ai/") || path.contains("/chat/") || path.contains("/qa/") {
        if tenant.is_quota_exceeded("daily_ai_queries")
            .map_err(|e| AiStudioError::internal(format!("检查配额失败: {}", e)))? {
            return Err(AiStudioError::quota_exceeded("每日 AI 查询配额已用完".to_string()));
        }
    }
    
    // 检查存储配额（对于文件上传请求）
    if matches!(req.method(), &actix_web::http::Method::POST | &actix_web::http::Method::PUT) 
        && (path.contains("/upload") || path.contains("/documents")) {
        if tenant.is_quota_exceeded("storage")
            .map_err(|e| AiStudioError::internal(format!("检查配额失败: {}", e)))? {
            return Err(AiStudioError::quota_exceeded("存储配额已用完".to_string()));
        }
    }
    
    Ok(())
}

/// 租户中间件配置辅助函数
pub struct TenantMiddlewareConfig;

impl TenantMiddlewareConfig {
    /// 配置标准的租户中间件栈
    pub fn standard() -> Vec<Box<dyn Fn(&mut ServiceConfig)>> {
        vec![
            Box::new(|_cfg| { }),
            Box::new(|_cfg| { }),
        ]
    }

    /// 配置仅头部识别的租户中间件
    pub fn header_only() -> Vec<Box<dyn Fn(&mut ServiceConfig)>> {
        vec![
            Box::new(|_cfg| { }),
            Box::new(|_cfg| { }),
        ]
    }

    /// 配置子域名识别的租户中间件
    pub fn subdomain_only() -> Vec<Box<dyn Fn(&mut ServiceConfig)>> {
        vec![
            Box::new(|_cfg| { }),
            Box::new(|_cfg| { }),
        ]
    }

    /// 配置可选的租户中间件（不强制要求租户）
    pub fn optional() -> Vec<Box<dyn Fn(&mut ServiceConfig)>> {
        vec![
            Box::new(|_cfg| { }),
        ]
    }
}