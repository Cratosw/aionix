// 日志系统设置

use crate::config::LoggingConfig;
use anyhow::Result;

use tracing::Level;
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter, Layer,
};

/// 日志系统初始化器
pub struct LoggingSetup;

impl LoggingSetup {
    /// 初始化日志系统
    pub fn init(config: &LoggingConfig) -> Result<()> {
        // 创建环境过滤器
        let env_filter = EnvFilter::try_from_default_env()
            .or_else(|_| EnvFilter::try_new(&config.level))
            .unwrap_or_else(|_| EnvFilter::new("info"));

        // 根据配置创建订阅器
        match config.format.as_str() {
            "json" => {
                let subscriber = tracing_subscriber::fmt()
                    .json()
                    .with_env_filter(env_filter)
                    .with_target(true)
                    .with_thread_ids(true)
                    .with_thread_names(true)
                    .with_file(true)
                    .with_line_number(true)
                    .finish();
                tracing::subscriber::set_global_default(subscriber)?;
            }
            "pretty" => {
                let subscriber = tracing_subscriber::fmt()
                    .pretty()
                    .with_env_filter(env_filter)
                    .with_target(true)
                    .with_thread_ids(true)
                    .with_thread_names(true)
                    .with_file(true)
                    .with_line_number(true)
                    .finish();
                tracing::subscriber::set_global_default(subscriber)?;
            }
            "compact" => {
                let subscriber = tracing_subscriber::fmt()
                    .compact()
                    .with_env_filter(env_filter)
                    .with_target(true)
                    .finish();
                tracing::subscriber::set_global_default(subscriber)?;
            }
            _ => {
                let subscriber = tracing_subscriber::fmt()
                    .with_env_filter(env_filter)
                    .with_target(true)
                    .with_thread_ids(true)
                    .with_thread_names(true)
                    .with_file(true)
                    .with_line_number(true)
                    .finish();
                tracing::subscriber::set_global_default(subscriber)?;
            }
        }

        tracing::info!("日志系统初始化完成");
        tracing::info!("日志级别: {}", config.level);
        tracing::info!("日志格式: {}", config.format);
        
        if config.file_enabled {
            tracing::info!("文件日志已启用: {:?}", config.file_path);
        }

        Ok(())
    }



    /// 解析日志级别
    pub fn parse_level(level: &str) -> Level {
        match level.to_lowercase().as_str() {
            "trace" => Level::TRACE,
            "debug" => Level::DEBUG,
            "info" => Level::INFO,
            "warn" => Level::WARN,
            "error" => Level::ERROR,
            _ => Level::INFO,
        }
    }

    /// 创建开发环境日志配置
    pub fn development_config() -> LoggingConfig {
        LoggingConfig {
            level: "debug".to_string(),
            format: "pretty".to_string(),
            file_enabled: false,
            file_path: None,
            max_file_size: None,
            max_files: None,
        }
    }

    /// 创建生产环境日志配置
    pub fn production_config() -> LoggingConfig {
        LoggingConfig {
            level: "info".to_string(),
            format: "json".to_string(),
            file_enabled: true,
            file_path: Some("./logs/aionix.log".to_string()),
            max_file_size: Some(100 * 1024 * 1024), // 100MB
            max_files: Some(10),
        }
    }

    /// 创建测试环境日志配置
    pub fn test_config() -> LoggingConfig {
        LoggingConfig {
            level: "warn".to_string(),
            format: "compact".to_string(),
            file_enabled: false,
            file_path: None,
            max_file_size: None,
            max_files: None,
        }
    }
}