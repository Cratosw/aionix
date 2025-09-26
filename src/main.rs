use actix_web::{web, App, HttpServer, HttpResponse, Result as ActixResult};
use actix_cors::Cors;
use chrono::Utc;

mod api;
mod config;
mod errors;
mod logging;
mod db;
mod health;
mod services;

use config::ConfigLoader;
use errors::ErrorHandlerMiddleware;
use logging::LoggingSetup;
use db::{DatabaseManager, MigrationManager};
use api::routes::ApiRouteConfig;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // 初始化配置
    let config = ConfigLoader::init()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
    
    // 初始化结构化日志系统
    LoggingSetup::init(&config.logging)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
    
    tracing::info!("🚀 启动 Aionix AI Studio v{}", config.environment.version);

    // 初始化数据库连接
    DatabaseManager::init(config.database.clone())
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

    // 初始化数据库迁移系统
    let db_manager = DatabaseManager::get()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
    let migration_manager = MigrationManager::new(db_manager.get_connection().clone());
    migration_manager.init()
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

    // 检查并应用待处理的迁移
    match migration_manager.migrate().await {
        Ok(applied) => {
            if !applied.is_empty() {
                tracing::info!("应用了 {} 个数据库迁移", applied.len());
            }
        }
        Err(e) => {
            tracing::warn!("数据库迁移检查失败: {}", e);
        }
    }
    
    // 打印配置摘要
    ConfigLoader::print_summary();
    
    tracing::info!("🌐 服务器启动地址: http://{}:{}", config.server.host, config.server.port);
    tracing::info!("📋 健康检查: http://{}:{}/health", config.server.host, config.server.port);
    
    // 启动 HTTP 服务器
    let mut server = HttpServer::new(move || {
        let app = App::new()
            // CORS 配置
            .wrap(
                Cors::default()
                    .allow_any_origin()
                    .allow_any_method()
                    .allow_any_header()
                    .max_age(3600)
            )
            // 添加错误处理中间件
            .wrap(ErrorHandlerMiddleware)
            // 添加 tracing 中间件
            .wrap(tracing_actix_web::TracingLogger::default())
            // 根路径
            .route("/", web::get().to(index))
            // 传统健康检查端点（向后兼容）
            .route("/health", web::get().to(health::health_check));

        // 根据环境配置不同的路由
        let app = if cfg!(debug_assertions) {
            app.configure(ApiRouteConfig::configure_dev)
        } else {
            app.configure(ApiRouteConfig::configure_prod)
        };

        app
    });

    // 配置服务器参数
    if let Some(workers) = config.server.workers {
        server = server.workers(workers);
    }

    server
        .bind((config.server.host.clone(), config.server.port))?
        .run()
        .await
}

/// 根路径处理器
async fn index() -> ActixResult<HttpResponse> {
    let info = serde_json::json!({
        "name": "Aionix AI Studio",
        "version": env!("CARGO_PKG_VERSION"),
        "description": "企业级 AI Studio - 基于 Rust 的多租户 AI 问答系统",
        "api": {
            "version": "v1",
            "base_url": "/api/v1",
            "documentation": "/api/v1/docs",
            "openapi": "/api/v1/openapi.json"
        },
        "health": {
            "simple": "/health",
            "detailed": "/api/v1/health/detailed",
            "ready": "/api/v1/ready",
            "live": "/api/v1/live"
        },
        "timestamp": chrono::Utc::now(),
        "environment": if cfg!(debug_assertions) { "development" } else { "production" }
    });

    Ok(HttpResponse::Ok().json(info))
}