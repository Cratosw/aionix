// 中间件模块
// 定义各种中间件组件

pub mod access_control;
pub mod auth;
pub mod quota;
pub mod rate_limit;
pub mod tenant;

// 移除未使用的导入
pub use auth::*;
pub use quota::*;
pub use rate_limit::*;
pub use tenant::*;

/// 中间件配置助手
pub struct MiddlewareConfig;

impl MiddlewareConfig {
    /// 创建标准 API 中间件配置
    pub fn api_standard() -> impl Fn(&mut actix_web::web::ServiceConfig) {
        |_| {}
    }

    /// 创建管理员专用中间件配置
    pub fn admin_only() -> impl Fn(&mut actix_web::web::ServiceConfig) {
        |_| {}
    }

    /// 创建公开访问中间件配置
    pub fn public() -> impl Fn(&mut actix_web::web::ServiceConfig) {
        |_| {}
    }

    /// 创建带权限要求的中间件配置
    pub fn with_permissions(_permissions: Vec<String>) -> impl Fn(&mut actix_web::web::ServiceConfig) {
        |_| {}
    }

    /// 创建带角色要求的中间件配置
    pub fn with_roles(_roles: Vec<String>) -> impl Fn(&mut actix_web::web::ServiceConfig) {
        |_| {}
    }
}
