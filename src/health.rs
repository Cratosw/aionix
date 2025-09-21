use actix_web::{HttpRequest, HttpResponse, Result};
use crate::errors::AiStudioError;
use crate::logging::RequestContext;
use serde_json::json;
use tracing::{info, instrument};

/// 健康检查端点
#[instrument(skip(req))]
pub async fn health_check(req: HttpRequest) -> Result<HttpResponse, AiStudioError> {
    let context = RequestContext::from_http_request(&req);
    
    info!(
        request_id = %context.request_id,
        "健康检查请求"
    );

    let response = json!({
        "status": "healthy",
        "service": "aionix-ai-studio",
        "version": env!("CARGO_PKG_VERSION"),
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "request_id": context.request_id
    });

    Ok(HttpResponse::Ok().json(response))
}

/// 根路径处理器
#[instrument(skip(req))]
pub async fn index(req: HttpRequest) -> Result<HttpResponse, AiStudioError> {
    let context = RequestContext::from_http_request(&req);
    
    info!(
        request_id = %context.request_id,
        "根路径访问"
    );

    let response = json!({
        "message": "欢迎使用 Aionix AI Studio",
        "version": env!("CARGO_PKG_VERSION"),
        "docs": "/swagger-ui/",
        "request_id": context.request_id
    });

    Ok(HttpResponse::Ok().json(response))
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::test;

    #[actix_web::test]
    async fn test_health_check() {
        let req = test::TestRequest::default().to_http_request();
        let resp = health_check(req).await.unwrap();
        assert_eq!(resp.status(), 200);
    }

    #[actix_web::test]
    async fn test_index() {
        let req = test::TestRequest::default().to_http_request();
        let resp = index(req).await.unwrap();
        assert_eq!(resp.status(), 200);
    }
}