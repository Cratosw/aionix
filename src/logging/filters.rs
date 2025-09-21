// 日志过滤器

use tracing::{Level, Metadata};
use tracing_subscriber::filter::FilterFn;

/// 创建敏感信息过滤器
pub fn create_sensitive_filter() -> FilterFn<impl Fn(&Metadata<'_>) -> bool> {
    FilterFn::new(|metadata| {
        // 过滤掉包含敏感信息的日志
        let target = metadata.target();
        
        // 跳过密码、令牌等敏感字段的日志
        if target.contains("password") 
            || target.contains("token") 
            || target.contains("secret") 
            || target.contains("key") {
            return false;
        }

        true
    })
}

/// 创建性能过滤器
pub fn create_performance_filter() -> FilterFn<impl Fn(&Metadata<'_>) -> bool> {
    FilterFn::new(|metadata| {
        let target = metadata.target();
        let level = metadata.level();

        // 在生产环境中过滤掉过于详细的性能日志
        if cfg!(not(debug_assertions)) && *level == Level::TRACE {
            if target.contains("actix_web::middleware") 
                || target.contains("hyper") 
                || target.contains("tokio") {
                return false;
            }
        }

        true
    })
}

/// 创建模块过滤器
pub fn create_module_filter(allowed_modules: Vec<String>) -> FilterFn<impl Fn(&Metadata<'_>) -> bool> {
    FilterFn::new(move |metadata| {
        let target = metadata.target();
        
        // 只允许特定模块的日志
        allowed_modules.iter().any(|module| target.starts_with(module))
    })
}

/// 创建级别过滤器
pub fn create_level_filter(min_level: Level) -> FilterFn<impl Fn(&Metadata<'_>) -> bool> {
    FilterFn::new(move |metadata| {
        *metadata.level() <= min_level
    })
}

/// 创建开发环境过滤器
pub fn create_development_filter() -> FilterFn<impl Fn(&Metadata<'_>) -> bool> {
    FilterFn::new(|metadata| {
        let target = metadata.target();
        let level = metadata.level();

        // 在开发环境中显示更多详细信息
        if cfg!(debug_assertions) {
            return true;
        }

        // 生产环境中过滤掉一些噪音日志
        if *level == Level::DEBUG || *level == Level::TRACE {
            if target.starts_with("hyper") 
                || target.starts_with("tokio") 
                || target.starts_with("mio") 
                || target.starts_with("want") {
                return false;
            }
        }

        true
    })
}

/// 创建错误重点过滤器
pub fn create_error_focus_filter() -> FilterFn<impl Fn(&Metadata<'_>) -> bool> {
    FilterFn::new(|metadata| {
        let level = metadata.level();
        let target = metadata.target();

        // 总是记录错误和警告
        if *level <= Level::WARN {
            return true;
        }

        // 对于 INFO 级别，只记录应用程序相关的日志
        if *level == Level::INFO {
            return target.starts_with("aionix") || target.starts_with("actix_web");
        }

        // 对于 DEBUG 和 TRACE，只在调试模式下记录
        cfg!(debug_assertions)
    })
}

/// 创建请求过滤器
pub fn create_request_filter() -> FilterFn<impl Fn(&Metadata<'_>) -> bool> {
    FilterFn::new(|metadata| {
        let target = metadata.target();
        
        // 过滤掉健康检查请求的日志
        if target.contains("health") && metadata.level() > &Level::WARN {
            return false;
        }

        // 过滤掉静态资源请求的日志
        if target.contains("static") || target.contains("assets") {
            return metadata.level() <= &Level::WARN;
        }

        true
    })
}

/// 日志过滤器配置
pub struct LogFilterConfig {
    pub enable_sensitive_filter: bool,
    pub enable_performance_filter: bool,
    pub enable_development_filter: bool,
    pub enable_error_focus_filter: bool,
    pub enable_request_filter: bool,
    pub allowed_modules: Vec<String>,
    pub min_level: Level,
}

impl Default for LogFilterConfig {
    fn default() -> Self {
        Self {
            enable_sensitive_filter: true,
            enable_performance_filter: true,
            enable_development_filter: true,
            enable_error_focus_filter: false,
            enable_request_filter: true,
            allowed_modules: vec![
                "aionix".to_string(),
                "actix_web".to_string(),
                "sea_orm".to_string(),
            ],
            min_level: Level::INFO,
        }
    }
}

impl LogFilterConfig {
    /// 创建生产环境配置
    pub fn production() -> Self {
        Self {
            enable_sensitive_filter: true,
            enable_performance_filter: true,
            enable_development_filter: false,
            enable_error_focus_filter: true,
            enable_request_filter: true,
            allowed_modules: vec!["aionix".to_string()],
            min_level: Level::INFO,
        }
    }

    /// 创建开发环境配置
    pub fn development() -> Self {
        Self {
            enable_sensitive_filter: true,
            enable_performance_filter: false,
            enable_development_filter: true,
            enable_error_focus_filter: false,
            enable_request_filter: false,
            allowed_modules: vec![
                "aionix".to_string(),
                "actix_web".to_string(),
                "sea_orm".to_string(),
                "sqlx".to_string(),
            ],
            min_level: Level::DEBUG,
        }
    }

    /// 创建测试环境配置
    pub fn test() -> Self {
        Self {
            enable_sensitive_filter: true,
            enable_performance_filter: true,
            enable_development_filter: false,
            enable_error_focus_filter: true,
            enable_request_filter: true,
            allowed_modules: vec!["aionix".to_string()],
            min_level: Level::WARN,
        }
    }
}