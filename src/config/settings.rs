// 应用程序设置和配置
// 定义配置结构体和加载逻辑

use serde::{Deserialize, Serialize};

/// 应用程序配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub ai: AiConfig,
    pub redis: RedisConfig,
}

/// 服务器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub workers: Option<usize>,
}

/// 数据库配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
}

/// AI 服务配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    pub model_endpoint: String,
    pub api_key: String,
    pub max_tokens: u32,
    pub temperature: f32,
}

/// Redis 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    pub url: String,
    pub max_connections: u32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 8080,
                workers: None,
            },
            database: DatabaseConfig {
                url: "postgresql://localhost/aionix".to_string(),
                max_connections: 10,
                min_connections: 1,
            },
            ai: AiConfig {
                model_endpoint: "http://localhost:11434".to_string(),
                api_key: "".to_string(),
                max_tokens: 2048,
                temperature: 0.7,
            },
            redis: RedisConfig {
                url: "redis://localhost:6379".to_string(),
                max_connections: 10,
            },
        }
    }
}