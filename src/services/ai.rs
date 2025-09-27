// AI 服务模块
// 提供高级 AI 功能的服务层封装

use crate::ai::{RigAiClientManager, ModelManager, AiHealthChecker, HealthLevel};
use crate::config::AiConfig;
use crate::errors::AiStudioError;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// AI 服务特征
#[async_trait]
pub trait AiService: Send + Sync {
    /// 生成文本回复
    async fn generate_response(&self, prompt: &str, tenant_id: Uuid) -> Result<AiResponse, AiStudioError>;
    
    /// 生成文本嵌入
    async fn generate_embedding(&self, text: &str, tenant_id: Uuid) -> Result<Vec<f32>, AiStudioError>;
    
    /// 批量生成嵌入
    async fn generate_embeddings(&self, texts: &[String], tenant_id: Uuid) -> Result<Vec<Vec<f32>>, AiStudioError>;
    
    /// 检查服务健康状态
    async fn health_check(&self) -> Result<ServiceHealth, AiStudioError>;
}

/// AI 响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiResponse {
    pub text: String,
    pub model: String,
    pub tokens_used: u32,
    pub confidence: Option<f32>,
    pub metadata: serde_json::Value,
}

/// 服务健康状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceHealth {
    pub status: String,
    pub models_available: Vec<String>,
    pub latency_ms: u64,
    pub last_check: chrono::DateTime<chrono::Utc>,
}

/// AI 服务实现
pub struct AiServiceImpl {
    client_manager: Arc<RigAiClientManager>,
    model_manager: Arc<ModelManager>,
    health_checker: Arc<AiHealthChecker>,
}

impl AiServiceImpl {
    /// 创建新的 AI 服务实例
    pub async fn new(config: AiConfig) -> Result<Self, AiStudioError> {
        let client_manager = Arc::new(RigAiClientManager::new(config).await?);
        let model_manager = Arc::new(ModelManager::new());
        let health_checker = Arc::new(AiHealthChecker::new(
            client_manager.clone(),
            model_manager.clone(),
        ));
        
        info!("AI 服务初始化完成");
        
        Ok(Self {
            client_manager,
            model_manager,
            health_checker,
        })
    }
    
    /// 获取客户端管理器
    pub fn client_manager(&self) -> Arc<RigAiClientManager> {
        self.client_manager.clone()
    }
    
    /// 获取模型管理器
    pub fn model_manager(&self) -> Arc<ModelManager> {
        self.model_manager.clone()
    }
    
    /// 获取健康检查器
    pub fn health_checker(&self) -> Arc<AiHealthChecker> {
        self.health_checker.clone()
    }
    
    /// 启动后台健康检查
    pub async fn start_health_monitoring(&self) -> Result<(), AiStudioError> {
        let health_checker = self.health_checker.clone();
        
        tokio::spawn(async move {
            if let Err(e) = health_checker.start_periodic_checks().await {
                warn!("健康检查任务异常退出: {}", e);
            }
        });
        
        info!("AI 服务健康监控已启动");
        Ok(())
    }
}

#[async_trait]
impl AiService for AiServiceImpl {
    async fn generate_response(&self, prompt: &str, tenant_id: Uuid) -> Result<AiResponse, AiStudioError> {
        debug!("为租户 {} 生成 AI 响应，提示词长度: {}", tenant_id, prompt.len());
        
        // 使用重试机制执行生成
        let response = self.client_manager.with_retry(|| {
            let client_manager = self.client_manager.clone();
            let prompt = prompt.to_string();
            Box::pin(async move {
                client_manager.generate_text(&prompt).await
            })
        }).await?;
        
        Ok(AiResponse {
            text: response.text,
            model: response.model,
            tokens_used: response.tokens_used.unwrap_or(0),
            confidence: None, // 可以在后续版本中添加置信度计算
            metadata: response.metadata,
        })
    }
    
    async fn generate_embedding(&self, text: &str, tenant_id: Uuid) -> Result<Vec<f32>, AiStudioError> {
        debug!("为租户 {} 生成嵌入向量，文本长度: {}", tenant_id, text.len());
        
        // 使用重试机制执行嵌入生成
        let response = self.client_manager.with_retry(|| {
            let client_manager = self.client_manager.clone();
            let text = text.to_string();
            Box::pin(async move {
                client_manager.generate_embedding(&text).await
            })
        }).await?;
        
        Ok(response.embedding)
    }
    
    async fn generate_embeddings(&self, texts: &[String], tenant_id: Uuid) -> Result<Vec<Vec<f32>>, AiStudioError> {
        debug!("为租户 {} 批量生成嵌入向量，文本数量: {}", tenant_id, texts.len());
        
        // 使用重试机制执行批量嵌入生成
        let texts_owned = texts.to_vec();
        let responses = self.client_manager.with_retry(|| {
            let client_manager = self.client_manager.clone();
            let texts = texts_owned.clone();
            Box::pin(async move {
                client_manager.generate_embeddings(&texts).await
            })
        }).await?;
        
        Ok(responses.into_iter().map(|r| r.embedding).collect())
    }
    
    async fn health_check(&self) -> Result<ServiceHealth, AiStudioError> {
        let health_status = self.health_checker.check_health().await?;
        
        let models_available: Vec<String> = health_status.models
            .iter()
            .filter_map(|(model_id, status)| {
                if status.status == HealthLevel::Healthy {
                    Some(model_id.clone())
                } else {
                    None
                }
            })
            .collect();
        
        let overall_status = match health_status.overall_status {
            HealthLevel::Healthy => "healthy",
            HealthLevel::Degraded => "degraded",
            HealthLevel::Unhealthy => "unhealthy",
            HealthLevel::Unknown => "unknown",
        };
        
        Ok(ServiceHealth {
            status: overall_status.to_string(),
            models_available,
            latency_ms: health_status.check_duration_ms,
            last_check: health_status.last_check,
        })
    }
}

/// AI 服务配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiServiceConfig {
    pub ai: AiConfig,
    pub health_check_enabled: bool,
    pub health_check_interval_seconds: u64,
    pub max_concurrent_requests: usize,
    pub request_timeout_seconds: u64,
}

impl Default for AiServiceConfig {
    fn default() -> Self {
        Self {
            ai: AiConfig {
                model_endpoint: "http://localhost:11434".to_string(),
                api_key: "".to_string(),
                max_tokens: 2048,
                temperature: 0.7,
                timeout: 30,
                retry_attempts: 3,
            },
            health_check_enabled: true,
            health_check_interval_seconds: 30,
            max_concurrent_requests: 10,
            request_timeout_seconds: 60,
        }
    }
}

/// AI 服务工厂
pub struct AiServiceFactory;

impl AiServiceFactory {
    /// 创建 AI 服务实例
    pub async fn create(config: AiServiceConfig) -> Result<Arc<dyn AiService>, AiStudioError> {
        let service = AiServiceImpl::new(config.ai).await?;
        
        Ok(Arc::new(service))
    }
    
    /// 创建带健康监控的 AI 服务实例
    pub async fn create_with_monitoring(config: AiServiceConfig) -> Result<Arc<dyn AiService>, AiStudioError> {
        let service = AiServiceImpl::new(config.ai).await?;
        
        if config.health_check_enabled {
            service.start_health_monitoring().await?;
        }
        
        Ok(Arc::new(service))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AiConfig;
    
    fn create_test_config() -> AiServiceConfig {
        AiServiceConfig {
            ai: AiConfig {
                model_endpoint: "mock://test".to_string(),
                api_key: "test_key".to_string(),
                max_tokens: 1000,
                temperature: 0.7,
                timeout: 30,
                retry_attempts: 3,
            },
            health_check_enabled: false, // 测试时禁用
            health_check_interval_seconds: 30,
            max_concurrent_requests: 10,
            request_timeout_seconds: 60,
        }
    }
    
    #[tokio::test]
    async fn test_ai_service_creation() {
        let config = create_test_config();
        let service = AiServiceFactory::create(config).await;
        
        assert!(service.is_ok());
    }
    
    #[tokio::test]
    async fn test_ai_service_text_generation() {
        let config = create_test_config();
        let service = match AiServiceFactory::create(config).await {
            Ok(service) => service,
            Err(_) => return,
        };
        let tenant_id = Uuid::new_v4();
        
        let result = service.generate_response("Hello, world!", tenant_id).await;
        assert!(result.is_ok());
        
        let response = result.unwrap();
        assert!(!response.text.is_empty());
        assert!(!response.model.is_empty());
        assert!(response.tokens_used > 0);
    }
    
    #[tokio::test]
    async fn test_ai_service_embedding_generation() {
        let config = create_test_config();
        let service = match AiServiceFactory::create(config).await {
            Ok(service) => service,
            Err(_) => return,
        };
        let tenant_id = Uuid::new_v4();
        
        let result = service.generate_embedding("Test text", tenant_id).await;
        assert!(result.is_ok());
        
        let embedding = result.unwrap();
        assert!(!embedding.is_empty());
    }
    
    #[tokio::test]
    async fn test_ai_service_batch_embeddings() {
        let config = create_test_config();
        let service = match AiServiceFactory::create(config).await {
            Ok(service) => service,
            Err(_) => return,
        };
        let tenant_id = Uuid::new_v4();
        
        let texts = vec![
            "First text".to_string(),
            "Second text".to_string(),
        ];
        
        let result = service.generate_embeddings(&texts, tenant_id).await;
        assert!(result.is_ok());
        
        let embeddings = result.unwrap();
        assert_eq!(embeddings.len(), 2);
        assert!(!embeddings[0].is_empty());
        assert!(!embeddings[1].is_empty());
    }
    
    #[tokio::test]
    async fn test_ai_service_health_check() {
        let config = create_test_config();
        let service = match AiServiceFactory::create(config).await {
            Ok(service) => service,
            Err(_) => return,
        };
        
        let result = service.health_check().await;
        assert!(result.is_ok());
        
        let health = result.unwrap();
        assert!(!health.status.is_empty());
        assert!(health.latency_ms > 0);
    }
}