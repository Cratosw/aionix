use actix_web::{HttpResponse, Result};
use serde_json::json;

/// 健康检查端点
pub async fn health_check() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(json!({
        "status": "healthy",
        "service": "aionix-ai-studio",
        "version": env!("CARGO_PKG_VERSION"),
        "timestamp": chrono::Utc::now().to_rfc3339()
    })))
}

/// 根路径处理器
pub async fn index() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(json!({
        "message": "欢迎使用 Aionix AI Studio",
        "version": env!("CARGO_PKG_VERSION"),
        "docs": "/swagger-ui/"
    })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[actix_web::test]
    async fn test_health_check() {
        let resp = health_check().await.unwrap();
        assert_eq!(resp.status(), 200);
    }

    #[actix_web::test]
    async fn test_index() {
        let resp = index().await.unwrap();
        assert_eq!(resp.status(), 200);
    }
}