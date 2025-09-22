// Temporarily disabled middleware implementations
// These will be re-enabled once the lifetime issues are resolved

use actix_web::{
    dev::{Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpResponse,
};
use futures::future::{LocalBoxFuture, StdReady, std_ready};
use std::future::{Ready, ready};
use actix_web::body::BoxBody;
use actix_web::web::ServiceConfig;

// Simple pass-through middleware implementations
pub struct RequestIdMiddleware;

impl<S, B> Transform<S, ServiceRequest> for RequestIdMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: actix_web::body::MessageBody + 'static,
{
    type Response = ServiceResponse<BoxBody>;
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
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: actix_web::body::MessageBody + 'static,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&self, ctx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
        self.service.poll_ready(ctx)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let fut = self.service.call(req);
        Box::pin(async move {
            Ok(fut.await?.map_into_boxed_body())
        })
    }
}

// Simple middleware config
pub struct MiddlewareConfig;

impl MiddlewareConfig {
    pub fn api_standard() -> impl Fn(&mut ServiceConfig) {
        |_cfg| {
            // Temporarily disabled
        }
    }

    pub fn admin_only() -> impl Fn(&mut ServiceConfig) {
        |_cfg| {
            // Temporarily disabled
        }
    }

    pub fn public() -> impl Fn(&mut ServiceConfig) {
        |_cfg| {
            // Temporarily disabled
        }
    }

    pub fn with_permissions(_permissions: Vec<String>) -> impl Fn(&mut ServiceConfig) {
        |_cfg| {
            // Temporarily disabled
        }
    }

    pub fn with_roles(_roles: Vec<String>) -> impl Fn(&mut ServiceConfig) {
        |_cfg| {
            // Temporarily disabled
        }
    }

    pub fn jwt_auth(_secret_key: String) -> impl Fn(&mut ServiceConfig) {
        |_cfg| {
            // Temporarily disabled
        }
    }

    pub fn api_key_auth() -> impl Fn(&mut ServiceConfig) {
        |_cfg| {
            // Temporarily disabled
        }
    }

    pub fn tenant_identification() -> impl Fn(&mut ServiceConfig) {
        |_cfg| {
            // Temporarily disabled
        }
    }
}