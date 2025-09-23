// 错误处理中间件


use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpMessage,
};
use futures::future::LocalBoxFuture;
use std::{
    future::{ready, Ready},
    rc::Rc,
};
use tracing::{error, info, warn};
use uuid::Uuid;

/// 错误处理中间件
pub struct ErrorHandlerMiddleware;

impl<S, B> Transform<S, ServiceRequest> for ErrorHandlerMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = ErrorHandlerMiddlewareService<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(ErrorHandlerMiddlewareService {
            service: Rc::new(service),
        }))
    }
}

pub struct ErrorHandlerMiddlewareService<S> {
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for ErrorHandlerMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service = self.service.clone();

        Box::pin(async move {
            // 生成请求 ID
            let request_id = Uuid::new_v4().to_string();
            req.extensions_mut().insert(request_id.clone());

            // 记录请求开始
            let method = req.method().clone();
            let path = req.path().to_string();
            let start_time = std::time::Instant::now();

            info!(
                request_id = %request_id,
                method = %method,
                path = %path,
                "开始处理请求"
            );

            // 调用下一个服务
            let result = service.call(req).await;

            // 计算处理时间
            let duration = start_time.elapsed();

            match result {
                Ok(response) => {
                    let status = response.status();
                    
                    if status.is_success() {
                        info!(
                            request_id = %request_id,
                            method = %method,
                            path = %path,
                            status = %status.as_u16(),
                            duration_ms = %duration.as_millis(),
                            "请求处理成功"
                        );
                    } else if status.is_client_error() {
                        warn!(
                            request_id = %request_id,
                            method = %method,
                            path = %path,
                            status = %status.as_u16(),
                            duration_ms = %duration.as_millis(),
                            "客户端错误"
                        );
                    } else {
                        error!(
                            request_id = %request_id,
                            method = %method,
                            path = %path,
                            status = %status.as_u16(),
                            duration_ms = %duration.as_millis(),
                            "服务器错误"
                        );
                    }

                    Ok(response)
                }
                Err(err) => {
                    error!(
                        request_id = %request_id,
                        method = %method,
                        path = %path,
                        error = %err,
                        duration_ms = %duration.as_millis(),
                        "请求处理失败"
                    );

                    Err(err)
                }
            }
        })
    }
}

/// 请求 ID 中间件
pub struct RequestIdMiddleware;

impl<S, B> Transform<S, ServiceRequest> for RequestIdMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = RequestIdMiddlewareService<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(RequestIdMiddlewareService {
            service: Rc::new(service),
        }))
    }
}

pub struct RequestIdMiddlewareService<S> {
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for RequestIdMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service = self.service.clone();

        Box::pin(async move {
            // 从请求头获取或生成请求 ID
            let request_id = req
                .headers()
                .get("X-Request-ID")
                .and_then(|h| h.to_str().ok())
                .map(|s| s.to_string())
                .unwrap_or_else(|| Uuid::new_v4().to_string());

            // 存储请求 ID 到扩展中
            req.extensions_mut().insert(request_id.clone());

            // 调用下一个服务
            let mut response = service.call(req).await?;

            // 在响应头中添加请求 ID
            response.headers_mut().insert(
                actix_web::http::header::HeaderName::from_static("x-request-id"),
                actix_web::http::header::HeaderValue::from_str(&request_id).unwrap(),
            );

            Ok(response)
        })
    }
}

/// 从请求扩展中获取请求 ID
pub fn get_request_id(req: &ServiceRequest) -> Option<String> {
    req.extensions().get::<String>().cloned()
}

/// 从 HTTP 请求中获取请求 ID
pub fn get_request_id_from_http(req: &actix_web::HttpRequest) -> Option<String> {
    req.extensions().get::<String>().cloned()
}