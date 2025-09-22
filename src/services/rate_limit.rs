// 限流服务
// 实现基于 Redis 的 API 调用频率限制

use std::time::Duration;
use uuid::Uuid;
use chrono::{Utc, DateTime};
use serde::{Deserialize, Serialize};
use tracing::{info, warn, error, instrument, debug};
use utoipa::ToSchema;

use crate::errors::AiStudioError;

/// 限流策略
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RateLimitPolicy {
    /// 时间窗口（秒）
    pub window_seconds: u64,
    /// 最大请求数
    pub max_requests: u64,
    /// 策略名称
    pub name: String,
    /// 是否启用
    pub enabled: bool,
}

/// 限流结果
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RateLimitResult {
    /// 是否允许请求
    pub allowed: bool,
    /// 当前窗口内的请求数
    pub current_requests: u64,
    /// 最大请求数
    pub max_requests: u64,
    /// 剩余请求数
    pub remaining_requests: u64,
    /// 窗口重置时间
    pub reset_time: DateTime<Utc>,
    /// 重试建议时间（秒）
    pub retry_after: Option<u64>,
}

/// 限流键类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RateLimitKeyType {
    /// 基于租户的限流
    Tenant(Uuid),
    /// 基于用户的限流
    User(Uuid),
    /// 基于 API 密钥的限流
    ApiKey(Uuid),
    /// 基于 IP 的限流
    Ip(String),
    /// 全局限流
    Global,
    /// 自定义键
    Custom(String),
}

/// 限流配置
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Redis 连接字符串
    pub redis_url: String,
    /// 默认策略
    pub default_policies: Vec<RateLimitPolicy>,
    /// 键前缀
    pub key_prefix: String,
}

/// 限流服务
pub struct RateLimitService {
    #[cfg(feature = "redis")]
    redis_client: redis::Client,
    config: RateLimitConfig,
}

impl RateLimitService {
    /// 创建新的限流服务实例
    pub fn new(config: RateLimitConfig) -> Result<Self, AiStudioError> {
        #[cfg(feature = "redis")]
        {
            let redis_client = redis::Client::open(config.redis_url.as_str())
                .map_err(|e| AiStudioError::internal(format!("Redis 连接失败: {}", e)))?;

            Ok(Self {
                redis_client,
                config,
            })
        }

        #[cfg(not(feature = "redis"))]
        {
            warn!("Redis 功能未启用，限流服务将使用内存实现");
            Ok(Self { config })
        }
    }

    /// 检查限流
    #[instrument(skip(self))]
    pub async fn check_rate_limit(
        &self,
        key_type: RateLimitKeyType,
        policy: &RateLimitPolicy,
    ) -> Result<RateLimitResult, AiStudioError> {
        if !policy.enabled {
            return Ok(RateLimitResult {
                allowed: true,
                current_requests: 0,
                max_requests: policy.max_requests,
                remaining_requests: policy.max_requests,
                reset_time: Utc::now() + chrono::Duration::seconds(policy.window_seconds as i64),
                retry_after: None,
            });
        }

        #[cfg(feature = "redis")]
        {
            self.check_rate_limit_redis(key_type, policy).await
        }

        #[cfg(not(feature = "redis"))]
        {
            self.check_rate_limit_memory(key_type, policy).await
        }
    }

    /// 批量检查多个限流策略
    #[instrument(skip(self))]
    pub async fn batch_check_rate_limits(
        &self,
        key_type: RateLimitKeyType,
        policies: &[RateLimitPolicy],
    ) -> Result<Vec<RateLimitResult>, AiStudioError> {
        let mut results = Vec::new();
        
        for policy in policies {
            let result = self.check_rate_limit(key_type.clone(), policy).await?;
            results.push(result);
            
            // 如果任何一个策略不允许，就停止检查
            if !result.allowed {
                break;
            }
        }

        Ok(results)
    }

    /// 增加请求计数
    #[instrument(skip(self))]
    pub async fn increment_request_count(
        &self,
        key_type: RateLimitKeyType,
        policy: &RateLimitPolicy,
    ) -> Result<u64, AiStudioError> {
        if !policy.enabled {
            return Ok(0);
        }

        #[cfg(feature = "redis")]
        {
            self.increment_request_count_redis(key_type, policy).await
        }

        #[cfg(not(feature = "redis"))]
        {
            self.increment_request_count_memory(key_type, policy).await
        }
    }

    /// 获取当前请求统计
    #[instrument(skip(self))]
    pub async fn get_request_stats(
        &self,
        key_type: RateLimitKeyType,
        policy: &RateLimitPolicy,
    ) -> Result<RateLimitResult, AiStudioError> {
        #[cfg(feature = "redis")]
        {
            self.get_request_stats_redis(key_type, policy).await
        }

        #[cfg(not(feature = "redis"))]
        {
            self.get_request_stats_memory(key_type, policy).await
        }
    }

    /// 重置限流计数器
    #[instrument(skip(self))]
    pub async fn reset_rate_limit(
        &self,
        key_type: RateLimitKeyType,
        policy: &RateLimitPolicy,
    ) -> Result<(), AiStudioError> {
        #[cfg(feature = "redis")]
        {
            self.reset_rate_limit_redis(key_type, policy).await
        }

        #[cfg(not(feature = "redis"))]
        {
            self.reset_rate_limit_memory(key_type, policy).await
        }
    }

    // Redis 实现
    #[cfg(feature = "redis")]
    async fn check_rate_limit_redis(
        &self,
        key_type: RateLimitKeyType,
        policy: &RateLimitPolicy,
    ) -> Result<RateLimitResult, AiStudioError> {
        use redis::AsyncCommands;

        let key = self.build_redis_key(&key_type, &policy.name);
        let mut conn = self.redis_client.get_async_connection().await
            .map_err(|e| AiStudioError::internal(format!("获取 Redis 连接失败: {}", e)))?;

        let now = Utc::now().timestamp();
        let window_start = now - policy.window_seconds as i64;

        // 使用 Redis 的 ZREMRANGEBYSCORE 清理过期的请求记录
        let _: () = conn.zremrangebyscore(&key, 0, window_start).await
            .map_err(|e| AiStudioError::internal(format!("清理过期记录失败: {}", e)))?;

        // 获取当前窗口内的请求数
        let current_requests: u64 = conn.zcard(&key).await
            .map_err(|e| AiStudioError::internal(format!("获取请求计数失败: {}", e)))?;

        let allowed = current_requests < policy.max_requests;
        let remaining_requests = if current_requests < policy.max_requests {
            policy.max_requests - current_requests
        } else {
            0
        };

        let reset_time = Utc::now() + chrono::Duration::seconds(policy.window_seconds as i64);
        let retry_after = if !allowed {
            Some(policy.window_seconds)
        } else {
            None
        };

        // 如果允许请求，添加当前请求到计数器
        if allowed {
            let _: () = conn.zadd(&key, now, format!("req_{}", now)).await
                .map_err(|e| AiStudioError::internal(format!("添加请求记录失败: {}", e)))?;
            
            // 设置过期时间
            let _: () = conn.expire(&key, policy.window_seconds as usize).await
                .map_err(|e| AiStudioError::internal(format!("设置过期时间失败: {}", e)))?;
        }

        Ok(RateLimitResult {
            allowed,
            current_requests: if allowed { current_requests + 1 } else { current_requests },
            max_requests: policy.max_requests,
            remaining_requests: if allowed { remaining_requests - 1 } else { remaining_requests },
            reset_time,
            retry_after,
        })
    }

    #[cfg(feature = "redis")]
    async fn increment_request_count_redis(
        &self,
        key_type: RateLimitKeyType,
        policy: &RateLimitPolicy,
    ) -> Result<u64, AiStudioError> {
        use redis::AsyncCommands;

        let key = self.build_redis_key(&key_type, &policy.name);
        let mut conn = self.redis_client.get_async_connection().await
            .map_err(|e| AiStudioError::internal(format!("获取 Redis 连接失败: {}", e)))?;

        let now = Utc::now().timestamp();
        
        let _: () = conn.zadd(&key, now, format!("req_{}", now)).await
            .map_err(|e| AiStudioError::internal(format!("添加请求记录失败: {}", e)))?;
        
        let _: () = conn.expire(&key, policy.window_seconds as usize).await
            .map_err(|e| AiStudioError::internal(format!("设置过期时间失败: {}", e)))?;

        let count: u64 = conn.zcard(&key).await
            .map_err(|e| AiStudioError::internal(format!("获取请求计数失败: {}", e)))?;

        Ok(count)
    }

    #[cfg(feature = "redis")]
    async fn get_request_stats_redis(
        &self,
        key_type: RateLimitKeyType,
        policy: &RateLimitPolicy,
    ) -> Result<RateLimitResult, AiStudioError> {
        use redis::AsyncCommands;

        let key = self.build_redis_key(&key_type, &policy.name);
        let mut conn = self.redis_client.get_async_connection().await
            .map_err(|e| AiStudioError::internal(format!("获取 Redis 连接失败: {}", e)))?;

        let now = Utc::now().timestamp();
        let window_start = now - policy.window_seconds as i64;

        let _: () = conn.zremrangebyscore(&key, 0, window_start).await
            .map_err(|e| AiStudioError::internal(format!("清理过期记录失败: {}", e)))?;

        let current_requests: u64 = conn.zcard(&key).await
            .map_err(|e| AiStudioError::internal(format!("获取请求计数失败: {}", e)))?;

        let allowed = current_requests < policy.max_requests;
        let remaining_requests = if current_requests < policy.max_requests {
            policy.max_requests - current_requests
        } else {
            0
        };

        Ok(RateLimitResult {
            allowed,
            current_requests,
            max_requests: policy.max_requests,
            remaining_requests,
            reset_time: Utc::now() + chrono::Duration::seconds(policy.window_seconds as i64),
            retry_after: if !allowed { Some(policy.window_seconds) } else { None },
        })
    }

    #[cfg(feature = "redis")]
    async fn reset_rate_limit_redis(
        &self,
        key_type: RateLimitKeyType,
        policy: &RateLimitPolicy,
    ) -> Result<(), AiStudioError> {
        use redis::AsyncCommands;

        let key = self.build_redis_key(&key_type, &policy.name);
        let mut conn = self.redis_client.get_async_connection().await
            .map_err(|e| AiStudioError::internal(format!("获取 Redis 连接失败: {}", e)))?;

        let _: () = conn.del(&key).await
            .map_err(|e| AiStudioError::internal(format!("删除限流记录失败: {}", e)))?;

        Ok(())
    }

    // 内存实现（用于没有 Redis 的情况）
    #[cfg(not(feature = "redis"))]
    async fn check_rate_limit_memory(
        &self,
        _key_type: RateLimitKeyType,
        policy: &RateLimitPolicy,
    ) -> Result<RateLimitResult, AiStudioError> {
        // 简单的内存实现，实际生产环境应该使用 Redis
        warn!("使用内存限流实现，不适用于生产环境");
        
        Ok(RateLimitResult {
            allowed: true,
            current_requests: 0,
            max_requests: policy.max_requests,
            remaining_requests: policy.max_requests,
            reset_time: Utc::now() + chrono::Duration::seconds(policy.window_seconds as i64),
            retry_after: None,
        })
    }

    #[cfg(not(feature = "redis"))]
    async fn increment_request_count_memory(
        &self,
        _key_type: RateLimitKeyType,
        _policy: &RateLimitPolicy,
    ) -> Result<u64, AiStudioError> {
        Ok(1)
    }

    #[cfg(not(feature = "redis"))]
    async fn get_request_stats_memory(
        &self,
        _key_type: RateLimitKeyType,
        policy: &RateLimitPolicy,
    ) -> Result<RateLimitResult, AiStudioError> {
        Ok(RateLimitResult {
            allowed: true,
            current_requests: 0,
            max_requests: policy.max_requests,
            remaining_requests: policy.max_requests,
            reset_time: Utc::now() + chrono::Duration::seconds(policy.window_seconds as i64),
            retry_after: None,
        })
    }

    #[cfg(not(feature = "redis"))]
    async fn reset_rate_limit_memory(
        &self,
        _key_type: RateLimitKeyType,
        _policy: &RateLimitPolicy,
    ) -> Result<(), AiStudioError> {
        Ok(())
    }

    // 辅助方法

    /// 构建 Redis 键
    fn build_redis_key(&self, key_type: &RateLimitKeyType, policy_name: &str) -> String {
        let key_suffix = match key_type {
            RateLimitKeyType::Tenant(id) => format!("tenant:{}", id),
            RateLimitKeyType::User(id) => format!("user:{}", id),
            RateLimitKeyType::ApiKey(id) => format!("apikey:{}", id),
            RateLimitKeyType::Ip(ip) => format!("ip:{}", ip),
            RateLimitKeyType::Global => "global".to_string(),
            RateLimitKeyType::Custom(key) => format!("custom:{}", key),
        };

        format!("{}:ratelimit:{}:{}", self.config.key_prefix, policy_name, key_suffix)
    }
}

/// 预定义的限流策略
pub struct RateLimitPolicies;

impl RateLimitPolicies {
    /// API 密钥限流策略
    pub fn api_key_policies() -> Vec<RateLimitPolicy> {
        vec![
            RateLimitPolicy {
                window_seconds: 60,
                max_requests: 60,
                name: "api_key_per_minute".to_string(),
                enabled: true,
            },
            RateLimitPolicy {
                window_seconds: 3600,
                max_requests: 1000,
                name: "api_key_per_hour".to_string(),
                enabled: true,
            },
            RateLimitPolicy {
                window_seconds: 86400,
                max_requests: 10000,
                name: "api_key_per_day".to_string(),
                enabled: true,
            },
        ]
    }

    /// 租户限流策略
    pub fn tenant_policies() -> Vec<RateLimitPolicy> {
        vec![
            RateLimitPolicy {
                window_seconds: 60,
                max_requests: 1000,
                name: "tenant_per_minute".to_string(),
                enabled: true,
            },
            RateLimitPolicy {
                window_seconds: 3600,
                max_requests: 10000,
                name: "tenant_per_hour".to_string(),
                enabled: true,
            },
        ]
    }

    /// IP 限流策略
    pub fn ip_policies() -> Vec<RateLimitPolicy> {
        vec![
            RateLimitPolicy {
                window_seconds: 60,
                max_requests: 100,
                name: "ip_per_minute".to_string(),
                enabled: true,
            },
            RateLimitPolicy {
                window_seconds: 3600,
                max_requests: 1000,
                name: "ip_per_hour".to_string(),
                enabled: true,
            },
        ]
    }

    /// 全局限流策略
    pub fn global_policies() -> Vec<RateLimitPolicy> {
        vec![
            RateLimitPolicy {
                window_seconds: 1,
                max_requests: 1000,
                name: "global_per_second".to_string(),
                enabled: true,
            },
            RateLimitPolicy {
                window_seconds: 60,
                max_requests: 50000,
                name: "global_per_minute".to_string(),
                enabled: true,
            },
        ]
    }
}

/// 限流服务工厂
pub struct RateLimitServiceFactory;

impl RateLimitServiceFactory {
    /// 创建限流服务实例
    pub fn create(config: RateLimitConfig) -> Result<RateLimitService, AiStudioError> {
        RateLimitService::new(config)
    }

    /// 创建默认配置的限流服务
    pub fn create_default(redis_url: String) -> Result<RateLimitService, AiStudioError> {
        let config = RateLimitConfig {
            redis_url,
            default_policies: RateLimitPolicies::api_key_policies(),
            key_prefix: "aionix".to_string(),
        };

        Self::create(config)
    }
}