// API 路由定义
// 定义所有 API 端点的路由配置

use actix_web::web;

/// 配置 API 路由
pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/v1")
            // 健康检查端点将在这里添加
            // 其他 API 端点将在后续任务中添加
    );
}