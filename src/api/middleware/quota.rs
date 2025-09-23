// 配额管理中间件
// 检查租户配额限制和使用统计

use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpMessage, HttpResponse,
    body::BoxBody, web::ServiceConfig,
};
use futures::future::LocalBoxFuture;
use std::future::{ready as std_ready, Ready as StdReady};
use std::rc::Rc;
use uuid::Uuid;
use tracing::{error, instrument};

use crate::api::middleware::tenant::TenantInfo;
use crate::api::responses::HttpResponseBuilder;
use crate::api::responses::ErrorResponse;
use crate::db::TenantContext;
use crate::services::quota::{QuotaService, QuotaType, QuotaUpdateRequest};
use crate::db::DatabaseManager;
use crate::errors::AiStudioError;

/// 配额检查中间件
pub struct QuotaCheckMiddleware {
    /// 需要检查的配额类型和请求量
    pub quota_checks: Vec<(QuotaType, u64)>,
    /// 是否在请求成功后更新配额
    pub update_on_success: bool,
}

impl QuotaCheckMiddleware {
    /// 创建新的配额检查中间件
    pub fn new(quota_checks: Vec<(QuotaType, u64)>) -> Self {
        Self {
            quota_checks,
            update_on_success: true,
        }
    }

    /// 创建 API 调用配额检查中间件
    pub fn api_calls() -> Self {
        Self::new(vec![(QuotaType::MonthlyApiCalls, 1)])
    }

    /// 创建 AI 查询配额检查中间件
    pub fn ai_queries() -> Self {
        Self::new(vec![(QuotaType::DailyAiQueries, 1)])
    }

    /// 创建存储配额检查中间件
    pub fn storage(bytes: u64) -> Self {
        Self::new(vec![(QuotaType::Storage, bytes)])
    }

    /// 创建文档配额检查中间件
    pub fn documents(count: u64) -> Self {
        Self::new(vec![(QuotaType::Documents, count)])
    }

    /// 创建知识库配额检查中间件
    pub fn knowledge_bases(count: u64) -> Self {
        Self::new(vec![(QuotaType::KnowledgeBases, count)])
    }

    /// 创建用户配额检查中间件
    pub fn users(count: u64) -> Self {
        Self::new(vec![(QuotaType::Users, count)])
    }

    /// 设置是否在成功后更新配额
    pub fn with_update_on_success(mut self, update: bool) -> Self {
        self.update_on_success = update;
        self
    }
}

impl<S, B> Transform<S, ServiceRequest> for QuotaCheckMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + Clone + 'static,
    S::Future: 'static,
    B: 'static + actix_web::body::MessageBody,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type Transform = QuotaCheckMiddlewareService<S>;
    type InitError = ();
    type Future = StdReady<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        std_ready(Ok(QuotaCheckMiddlewareService {
            service: Rc::new(service),
            quota_checks: self.quota_checks.clone(),
            update_on_success: self.update_on_success,
        }))
    }
}

pub struct QuotaCheckMiddlewareService<S> {
    service: Rc<S>,
    quota_checks: Vec<(QuotaType, u64)>,
    update_on_success: bool,
}

impl<S, B> Service<ServiceRequest> for QuotaCheckMiddlewareService<S>
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
        let quota_checks = self.quota_checks.clone();
        let update_on_success = self.update_on_success;
        let service = self.service.clone();

        Box::pin(async move {
            let (req, tenant_id) = {
                let req = req;
                let tenant_info_opt = req.extensions().get::<TenantInfo>().cloned();
                match tenant_info_opt {
                    Some(ti) => {
                        // 先释放借用
                        (req, ti.id)
                    }
                    None => {
                        let response = HttpResponse::BadRequest()
                            .json(ErrorResponse::detailed_error::<()>(
                                "TENANT_REQUIRED".to_string(),
                                "配额检查需要租户信息".to_string(),
                                None,
                                None,
                            ));
                        return Ok(req.into_response(response));
                    }
                }
            };

            // 检查配额
            if let Err(e) = check_quotas(&TenantInfo { 
                id: tenant_id, 
                slug: String::new(), 
                name: String::new(), 
                display_name: String::new(),
                status: crate::db::entities::tenant::TenantStatus::Active,
                context: TenantContext::new(tenant_id, String::new(), false),
            }, &[]).await {
                return Ok(req.into_response(
                    HttpResponseBuilder::forbidden::<()>()?
                ));
            }

            if update_on_success {
                req.extensions_mut().insert(QuotaUpdateInfo {
                    tenant_id,
                    quota_checks: quota_checks.clone(),
                });
            }

            let fut = service.call(req);
            Ok(fut.await?.map_into_boxed_body())
        })
    }
}

/// 配额更新中间件（在请求成功后更新配额使用量）
pub struct QuotaUpdateMiddleware;

impl<S, B> Transform<S, ServiceRequest> for QuotaUpdateMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + Clone + 'static,
    S::Future: 'static,
    B: 'static + actix_web::body::MessageBody,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type Transform = QuotaUpdateMiddlewareService<S>;
    type InitError = ();
    type Future = StdReady<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        std_ready(Ok(QuotaUpdateMiddlewareService { service }))
    }
}

pub struct QuotaUpdateMiddlewareService<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for QuotaUpdateMiddlewareService<S>
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

        let inner = self.service.clone();
        Box::pin(async move {
            let quota_update_info = req.extensions().get::<QuotaUpdateInfo>().cloned();

            let res = inner.call(req).await?;

            // 如果请求成功且有配额更新信息，则更新配额
            if res.status().is_success() {
                if let Some(update_info) = quota_update_info {
                    tokio::spawn(async move {
                        if let Err(e) = update_quotas_async(update_info).await {
                            error!("异步更新配额失败: {}", e);
                        }
                    });
                }
            }

            Ok(res.map_into_boxed_body())
        })
    }
}

/// 配额重置中间件（定期重置时间相关的配额）
pub struct QuotaResetMiddleware {
    /// 检查间隔（秒）
    pub check_interval: u64,
}

impl QuotaResetMiddleware {
    /// 创建配额重置中间件
    pub fn new(check_interval: u64) -> Self {
        Self { check_interval }
    }

    /// 创建默认配额重置中间件（每小时检查一次）
    pub fn default() -> Self {
        Self::new(3600)
    }
}

impl<S, B> Transform<S, ServiceRequest> for QuotaResetMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + Clone + 'static,
    S::Future: 'static,
    B: 'static + actix_web::body::MessageBody,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type Transform = QuotaResetMiddlewareService<S>;
    type InitError = ();
    type Future = StdReady<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        std_ready(Ok(QuotaResetMiddlewareService {
            service: Rc::new(service),
            check_interval: self.check_interval,
        }))
    }
}

pub struct QuotaResetMiddlewareService<S> {
    service: Rc<S>,
    check_interval: u64,
}

impl<S, B> Service<ServiceRequest> for QuotaResetMiddlewareService<S>
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
        let check_interval = self.check_interval;
        let service = self.service.clone();

        Box::pin(async move {
            let tenant_id_opt = req.extensions().get::<TenantInfo>().map(|ti| ti.id);
            if let Some(tenant_id) = tenant_id_opt {
                tokio::spawn(async move {
                    if let Err(e) = check_and_reset_quotas(tenant_id).await {
                        error!("检查和重置配额失败: {}", e);
                    }
                });
            }

            let fut = service.call(req);
            Ok(fut.await?.map_into_boxed_body())
        })
    }
}

/// 配额更新信息
#[derive(Debug, Clone)]
struct QuotaUpdateInfo {
    tenant_id: Uuid,
    quota_checks: Vec<(QuotaType, u64)>,
}

// 辅助函数

/// 检查配额
#[instrument(skip(tenant_info, quota_checks))]
async fn check_quotas(
    tenant_info: &TenantInfo,
    quota_checks: &[(QuotaType, u64)],
) -> Result<(), AiStudioError> {
    if quota_checks.is_empty() {
        return Ok(());
    }

    let db_manager = DatabaseManager::get()?;
    let db = db_manager.get_connection();
    let quota_service = QuotaService::new(db.clone());

    for (quota_type, requested_amount) in quota_checks {
        let result = quota_service
            .check_quota(tenant_info.id, quota_type.clone(), *requested_amount)
            .await?;

        if !result.allowed {
            return Err(AiStudioError::quota_exceeded(
                result.rejection_reason.unwrap_or_else(|| {
                    format!("配额超限: {:?}", quota_type)
                })
            ));
        }
    }

    Ok(())
}

/// 异步更新配额
#[instrument(skip(update_info))]
async fn update_quotas_async(update_info: QuotaUpdateInfo) -> Result<(), AiStudioError> {
    let db_manager = DatabaseManager::get()?;
    let db = db_manager.get_connection();
    let quota_service = QuotaService::new(db.clone());

    for (quota_type, amount) in update_info.quota_checks {
        let request = QuotaUpdateRequest {
            quota_type: quota_type.clone(),
            delta: amount as i64,
            operation: format!("API 调用更新: {:?}", quota_type),
            resource_id: None,
        };

        if let Err(e) = quota_service.update_quota_usage(update_info.tenant_id, request).await {
            error!(
                tenant_id = %update_info.tenant_id,
                quota_type = ?quota_type,
                error = %e,
                "更新配额使用量失败"
            );
        }
    }

    Ok(())
}

/// 检查和重置配额
#[instrument]
async fn check_and_reset_quotas(tenant_id: Uuid) -> Result<(), AiStudioError> {
    let db_manager = DatabaseManager::get()?;
    let db = db_manager.get_connection();
    let quota_service = QuotaService::new(db.clone());

    quota_service.reset_time_based_quotas(tenant_id).await?;
    Ok(())
}

/// 配额中间件配置辅助函数
pub struct QuotaMiddlewareConfig;

impl QuotaMiddlewareConfig {
    /// 配置 API 调用配额中间件
    pub fn api_calls() -> impl Fn(&mut ServiceConfig) {
        |_cfg| { }
    }

    /// 配置 AI 查询配额中间件
    pub fn ai_queries() -> impl Fn(&mut ServiceConfig) {
        |_cfg| { }
    }

    /// 配置存储配额中间件
    pub fn storage(bytes: u64) -> impl Fn(&mut ServiceConfig) {
        move |_cfg| { let _ = bytes; }
    }

    /// 配置文档配额中间件
    pub fn documents(count: u64) -> impl Fn(&mut ServiceConfig) {
        move |_cfg| { let _ = count; }
    }

    /// 配置完整的配额中间件栈
    pub fn full_stack() -> Vec<Box<dyn Fn(&mut ServiceConfig)>> {
        vec![Box::new(|_cfg| { }), Box::new(|_cfg| { }), Box::new(|_cfg| { })]
    }
}