// 配置加载器
// 处理配置文件加载和环境变量解析

use crate::config::AppConfig;
use aionix_common::CommonError;
use config::ConfigError;
use dotenvy::dotenv;
use std::sync::OnceLock;
use tracing::{info, warn};

/// 全局配置实例
static CONFIG: OnceLock<AppConfig> = OnceLock::new();

/// 配置加载器
pub struct ConfigLoader;

impl ConfigLoader {
    /// 初始化配置
    pub fn init() -> Result<&'static AppConfig, CommonError> {
        // 加载 .env 文件
        if let Err(e) = dotenv() {
            warn!("无法加载 .env 文件: {}", e);
        }

        // 加载配置
        let config = AppConfig::load()
            .map_err(convert_config_error)?;

        // 验证配置
        config.validate()?;

        // 存储到全局变量
        CONFIG.set(config).map_err(|_| {
            CommonError::internal("配置已经初始化")
        })?;

        let config = CONFIG.get().unwrap();
        
        info!("配置加载成功");
        info!("环境: {}", config.environment.name);
        info!("版本: {}", config.environment.version);
        info!("服务器: {}:{}", config.server.host, config.server.port);
        
        Ok(config)
    }

    /// 获取配置
    pub fn get() -> &'static AppConfig {
        CONFIG.get().expect("配置未初始化，请先调用 ConfigLoader::init()")
    }

    /// 重新加载配置
    pub fn reload() -> Result<&'static AppConfig, CommonError> {
        warn!("重新加载配置...");
        
        // 注意：这里不能真正重新加载，因为 OnceLock 只能设置一次
        // 在生产环境中，重新加载配置通常需要重启应用程序
        Err(CommonError::configuration(
            "配置重新加载需要重启应用程序"
        ))
    }

    /// 验证环境变量
    pub fn validate_env() -> Result<(), CommonError> {
        let required_vars = [
            "DATABASE_URL",
            "JWT_SECRET",
        ];

        for var in &required_vars {
            if std::env::var(var).is_err() {
                return Err(CommonError::configuration(
                    format!("缺少必需的环境变量: {}", var)
                ));
            }
        }

        Ok(())
    }

    /// 打印配置摘要
    pub fn print_summary() {
        let config = Self::get();
        
        println!("=== Aionix AI Studio 配置摘要 ===");
        println!("环境: {}", config.environment.name);
        println!("版本: {}", config.environment.version);
        println!("调试模式: {}", config.environment.debug);
        println!("服务器: {}:{}", config.server.host, config.server.port);
        println!("工作线程: {:?}", config.server.workers);
        println!("数据库连接池: {}-{}", config.database.min_connections, config.database.max_connections);
        
        #[cfg(feature = "redis")]
        println!("Redis 连接池: {}", config.redis.max_connections);
        
        println!("AI 端点: {}", config.ai.model_endpoint);
        println!("存储路径: {}", config.storage.path);
        println!("日志级别: {}", config.logging.level);
        println!("向量维度: {}", config.vector.dimension);
        println!("================================");
    }
}

/// 配置错误转换辅助函数
pub fn convert_config_error(err: ConfigError) -> CommonError {
    CommonError::configuration(format!("配置错误: {}", err))
}