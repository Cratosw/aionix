use actix_web::{web, App, HttpServer, middleware::Logger};

mod health;
use health::{health_check, index};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // 初始化日志
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    
    println!("🚀 启动 Aionix AI Studio v{}", env!("CARGO_PKG_VERSION"));
    
    // 获取服务器配置
    let host = std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse::<u16>()
        .expect("PORT 必须是有效的端口号");
    
    println!("🌐 服务器启动地址: http://{}:{}", host, port);
    println!("📋 健康检查: http://{}:{}/health", host, port);
    
    // 启动 HTTP 服务器
    HttpServer::new(|| {
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
    })
    .bind((host, port))?
    .run()
    .await
}