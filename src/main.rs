use actix_web::{web, App, HttpServer, middleware::Logger};

mod health;
mod config;
mod errors;
mod logging;

use health::{health_check, index};
use config::ConfigLoader;
use errors::{ErrorHandlerMiddleware, RequestIdMiddleware};
use logging::LoggingSetup;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // 初始化配置
    let config = ConfigLoader::init()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
    
    // 初始化结构化日志系统
    LoggingSetup::init(&config.logging)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
    
    tracing::info!("🚀 启动 Aionix AI Studio v{}", config.environment.version);
    
    // 打印配置摘要
    ConfigLoader::print_summary();
    
    tracing::info!("🌐 服务器启动地址: http://{}:{}", config.server.host, config.server.port);
    tracing::info!("📋 健康检查: http://{}:{}/health", config.server.host, config.server.port);
    
    // 启动 HTTP 服务器
    let mut server = HttpServer::new(|| {
        App::new()
            // 添加请求 ID 中间件
            .wrap(RequestIdMiddleware)
            // 添加错误处理中间件
            .wrap(ErrorHandlerMiddleware)
            // 添加 tracing 中间件
            .wrap(tracing_actix_web::TracingLogger::default())
            // 根路径
            .route("/", web::get().to(index))
            // 健康检查端点
            .route("/health", web::get().to(health_check))
            // API 路由组
            .service(
                web::scope("/api/v1")
                    .route("/health", web::get().to(health_check))
            )
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