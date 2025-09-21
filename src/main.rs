use actix_web::{web, App, HttpServer, middleware::Logger};

mod health;
mod config;

use health::{health_check, index};
use config::ConfigLoader;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // 初始化配置
    let config = ConfigLoader::init()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
    
    // 初始化日志
    env_logger::init_from_env(env_logger::Env::new().default_filter_or(&config.logging.level));
    
    println!("🚀 启动 Aionix AI Studio v{}", config.environment.version);
    
    // 打印配置摘要
    ConfigLoader::print_summary();
    
    println!("🌐 服务器启动地址: http://{}:{}", config.server.host, config.server.port);
    println!("📋 健康检查: http://{}:{}/health", config.server.host, config.server.port);
    
    // 启动 HTTP 服务器
    let mut server = HttpServer::new(|| {
        App::new()
            // 添加日志中间件
            .wrap(Logger::default())
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