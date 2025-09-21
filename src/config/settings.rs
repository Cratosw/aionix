// 应用程序设置和配置
// 定义配置结构体和加载逻辑

use config::{Config, ConfigError, Environment, File};
use serde::{Deserialize, Serialize};
use std::env;
use std::path::Path;
use aionix_common::CommonError;

/// 应用程序配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub ai: AiConfig,
    #[cfg(feature = "redis")]
    pub redis: RedisConfig,
    pub security: SecurityConfig,
    pub storage: StorageConfig,
    pub logging: LoggingConfig,
    pub vector: VectorConfig,
    pub environment: EnvironmentConfig,
}

/// 服务器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub workers: Option<usize>,
    pub keep_alive: u64,
    pub client_timeout: u64,
    pub client_shutdown: u64,
}

/// 数据库配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub connect_timeout: u64,
    pub idle_timeout: u64,
    pub max_lifetime: u64,
}

/// AI 服务配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    pub model_endpoint: String,
    pub api_key: String,
    pub max_tokens: u32,
    pub temperature: f32,
    pub timeout: u64,
    pub retry_attempts: u32,
}

/// Redis 配置
#[cfg(feature = "redis")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    pub url: String,
    pub max_connections: u32,
    pub connection_timeout: u64,
    pub response_timeout: u64,
}

/// 安全配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub jwt_secret: String,
    pub jwt_expiration: u64,
    pub bcrypt_cost: u32,
    pub cors_origins: Vec<String>,
    pub rate_limit_requests: u32,
    pub rate_limit_window: u64,
}

/// 存储配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub path: String,
    pub max_file_size: u64,
    pub allowed_extensions: Vec<String>,
}

/// 日志配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub format: String,
    pub file_enabled: bool,
    pub file_path: Option<String>,
    pub max_file_size: Option<u64>,
    pub max_files: Option<u32>,
}

/// 向量数据库配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorConfig {
    pub dimension: u32,
    pub similarity_threshold: f32,
    pub index_type: String,
    pub ef_construction: u32,
    pub m: u32,
}

/// 环境配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentConfig {
    pub name: String,
    pub debug: bool,
    pub version: String,
}

impl AppConfig {
    /// 从环境变量和配置文件加载配置
    pub fn load() -> Result<Self, ConfigError> {
        let mut config = Config::builder();

        // 1. 加载默认配置
        config = config.add_source(Config::try_from(&AppConfig::default())?);

        // 2. 尝试加载配置文件
        if Path::new("config.toml").exists() {
            config = config.add_source(File::with_name("config"));
        }

        // 3. 加载环境变量（优先级最高）
        config = config.add_source(
            Environment::with_prefix("AIONIX")
                .prefix_separator("_")
                .separator("__")
        );

        // 4. 构建配置
        let config = config.build()?;
        
        // 5. 反序列化为结构体
        let mut app_config: AppConfig = config.try_deserialize()?;
        
        // 6. 设置版本信息
        app_config.environment.version = env!("CARGO_PKG_VERSION").to_string();
        
        Ok(app_config)
    }

    /// 验证配置
    pub fn validate(&self) -> Result<(), CommonError> {
        use crate::config::ConfigValidator;
        
        match ConfigValidator::validate_all(self) {
            Ok(()) => Ok(()),
            Err(errors) => {
                let error_messages: Vec<String> = errors.iter()
                    .map(|e| e.to_string())
                    .collect();
                Err(CommonError::configuration(
                    format!("配置验证失败: {}", error_messages.join("; "))
                ))
            }
        }
    }

    /// 获取环境类型
    pub fn is_development(&self) -> bool {
        self.environment.name == "development"
    }

    /// 获取环境类型
    pub fn is_production(&self) -> bool {
        self.environment.name == "production"
    }

    /// 获取环境类型
    pub fn is_test(&self) -> bool {
        self.environment.name == "test"
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 8080,
                workers: None,
                keep_alive: 75,
                client_timeout: 5000,
                client_shutdown: 5000,
            },
            database: DatabaseConfig {
                url: "postgresql://localhost/aionix".to_string(),
                max_connections: 10,
                min_connections: 1,
                connect_timeout: 30,
                idle_timeout: 600,
                max_lifetime: 1800,
            },
            ai: AiConfig {
                model_endpoint: "http://localhost:11434".to_string(),
                api_key: "".to_string(),
                max_tokens: 2048,
                temperature: 0.7,
                timeout: 30,
                retry_attempts: 3,
            },
            #[cfg(feature = "redis")]
            redis: RedisConfig {
                url: "redis://localhost:6379".to_string(),
                max_connections: 10,
                connection_timeout: 5,
                response_timeout: 5,
            },
            security: SecurityConfig {
                jwt_secret: "your-super-secret-jwt-key-change-this-in-production".to_string(),
                jwt_expiration: 3600,
                bcrypt_cost: 12,
                cors_origins: vec!["*".to_string()],
                rate_limit_requests: 100,
                rate_limit_window: 60,
            },
            storage: StorageConfig {
                path: "./storage".to_string(),
                max_file_size: 10 * 1024 * 1024, // 10MB
                allowed_extensions: vec![
                    "pdf".to_string(),
                    "txt".to_string(),
                    "md".to_string(),
                    "doc".to_string(),
                    "docx".to_string(),
                ],
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                format: "json".to_string(),
                file_enabled: false,
                file_path: None,
                max_file_size: Some(100 * 1024 * 1024), // 100MB
                max_files: Some(10),
            },
            vector: VectorConfig {
                dimension: 1536,
                similarity_threshold: 0.8,
                index_type: "hnsw".to_string(),
                ef_construction: 200,
                m: 16,
            },
            environment: EnvironmentConfig {
                name: "development".to_string(),
                debug: true,
                version: "0.1.0".to_string(),
            },
        }
    }
}