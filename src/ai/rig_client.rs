// 基于 Rig 框架的 AI 客户端实现
// 使用 rig-core 0.20 版本

use crate::config::AiConfig;
use crate::errors::AiStudioError;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, info, warn};

#[cfg(feature = "ai")]
use rig_core::{
    completion::{CompletionModel, Prompt},
    embeddings::{EmbeddingModel, Embed},
    providers::{openai, ollama},
    agent::Agent,
};

/// Rig 基础的 AI 客户端
pub struct RigAiClient {
    config: Arc<AiConfig>,
    #[cfg(feature = "ai")]
    completion_model: Box<dyn CompletionModel + Send + Sync>,
    #[cfg(feature = "ai")]
    embedding_model: Box<dyn EmbeddingModel + Send + Sync>,
}

/// 生成响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RigGenerationResponse {
    pub text: String,
    pub model: String,
    pub tokens_used: Option<u32>,
    pub finish_reason: Option<String>,
    pub metadata: serde_json::Value,
}

/// 嵌入响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RigEmbeddingResponse {
    pub embedding: Vec<f32>,
    pub model: String,
    pub tokens_used: Option<u32>,
    pub metadata: serde_json::Value,
}

/// 健康状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RigHealthStatus {
    pub status: String,
    pub model: String,
    pub version: Option<String>,
    pub latency_ms: u64,
}

impl RigAiClient {
    /// 创建新的 Rig AI 客户端
    #[cfg(feature = "ai")]
    pub async fn new(config: AiConfig) -> Result<Self, AiStudioError> {
        let config = Arc::new(config);
        
        // 根据配置创建相应的模型
        let (completion_model, embedding_model) = if config.model_endpoint.contains("openai") {
            Self::create_openai_models(&config).await?
        } else if config.model_endpoint.contains("ollama") {
            Self::create_ollama_models(&config).await?
        } else {
            return Err(AiStudioError::ai("不支持的 AI 提供商"));
        };
        
        info!("Rig AI 客户端初始化完成，端点: {}", config.model_endpoint);
        
        Ok(Self {
            config,
            #[cfg(feature = "ai")]
            completion_model,
            #[cfg(feature = "ai")]
            embedding_model,
        })
    }
    
    #[cfg(not(feature = "ai"))]
    pub async fn new(_config: AiConfig) -> Result<Self, AiStudioError> {
        Err(AiStudioError::ai("AI 功能未启用"))
    }
    
    #[cfg(feature = "ai")]
    async fn create_openai_models(
        config: &AiConfig,
    ) -> Result<(Box<dyn CompletionModel + Send + Sync>, Box<dyn EmbeddingModel + Send + Sync>), AiStudioError> {
        // 创建 OpenAI 客户端
        let client = openai::Client::new(&config.api_key);
        
        // 创建完成模型
        let completion_model = client
            .model("gpt-3.5-turbo")
            .with_temperature(config.temperature as f64)
            .with_max_tokens(config.max_tokens as u32);
        
        // 创建嵌入模型
        let embedding_model = client.embedding_model("text-embedding-ada-002");
        
        Ok((
            Box::new(completion_model),
            Box::new(embedding_model),
        ))
    }
    
    #[cfg(feature = "ai")]
    async fn create_ollama_models(
        config: &AiConfig,
    ) -> Result<(Box<dyn CompletionModel + Send + Sync>, Box<dyn EmbeddingModel + Send + Sync>), AiStudioError> {
        // 创建 Ollama 客户端
        let client = ollama::Client::from_url(&config.model_endpoint);
        
        // 创建完成模型
        let completion_model = client
            .model("llama2")
            .with_temperature(config.temperature as f64)
            .with_max_tokens(config.max_tokens as u32);
        
        // 创建嵌入模型
        let embedding_model = client.embedding_model("nomic-embed-text");
        
        Ok((
            Box::new(completion_model),
            Box::new(embedding_model),
        ))
    }
    
    /// 生成文本
    pub async fn generate_text(&self, prompt: &str) -> Result<RigGenerationResponse, AiStudioError> {
        debug!("使用 Rig 生成文本，提示词长度: {}", prompt.len());
        
        #[cfg(feature = "ai")]
        {
            let response = self
                .completion_model
                .prompt(prompt)
                .await
                .map_err(|e| AiStudioError::ai(format!("Rig 文本生成失败: {}", e)))?;
            
            Ok(RigGenerationResponse {
                text: response.choice.message.content,
                model: response.model.unwrap_or_default(),
                tokens_used: response.usage.map(|u| u.total_tokens as u32),
                finish_reason: Some(response.choice.finish_reason.unwrap_or_default()),
                metadata: serde_json::json!({
                    "rig_response": true,
                    "usage": response.usage
                }),
            })
        }
        
        #[cfg(not(feature = "ai"))]
        {
            Err(AiStudioError::ai("AI 功能未启用"))
        }
    }
    
    /// 生成嵌入向量
    pub async fn generate_embedding(&self, text: &str) -> Result<RigEmbeddingResponse, AiStudioError> {
        debug!("使用 Rig 生成嵌入向量，文本长度: {}", text.len());
        
        #[cfg(feature = "ai")]
        {
            let embeddings = self
                .embedding_model
                .embed_documents(vec![text.to_string()])
                .await
                .map_err(|e| AiStudioError::ai(format!("Rig 嵌入生成失败: {}", e)))?;
            
            if embeddings.is_empty() {
                return Err(AiStudioError::ai("未生成嵌入向量"));
            }
            
            Ok(RigEmbeddingResponse {
                embedding: embeddings[0].clone(),
                model: self.get_embedding_model_name(),
                tokens_used: None, // Rig 可能不提供 token 使用信息
                metadata: serde_json::json!({
                    "rig_response": true,
                    "dimension": embeddings[0].len()
                }),
            })
        }
        
        #[cfg(not(feature = "ai"))]
        {
            Err(AiStudioError::ai("AI 功能未启用"))
        }
    }
    
    /// 批量生成嵌入向量
    pub async fn generate_embeddings(&self, texts: &[String]) -> Result<Vec<RigEmbeddingResponse>, AiStudioError> {
        debug!("使用 Rig 批量生成嵌入向量，文本数量: {}", texts.len());
        
        #[cfg(feature = "ai")]
        {
            let embeddings = self
                .embedding_model
                .embed_documents(texts.clone())
                .await
                .map_err(|e| AiStudioError::ai(format!("Rig 批量嵌入生成失败: {}", e)))?;
            
            let model_name = self.get_embedding_model_name();
            let results = embeddings
                .into_iter()
                .map(|embedding| RigEmbeddingResponse {
                    embedding,
                    model: model_name.clone(),
                    tokens_used: None,
                    metadata: serde_json::json!({
                        "rig_response": true,
                        "dimension": embedding.len()
                    }),
                })
                .collect();
            
            Ok(results)
        }
        
        #[cfg(not(feature = "ai"))]
        {
            Err(AiStudioError::ai("AI 功能未启用"))
        }
    }
    
    /// 健康检查
    pub async fn health_check(&self) -> Result<RigHealthStatus, AiStudioError> {
        let start_time = std::time::Instant::now();
        
        // 尝试生成一个简单的文本来测试连接
        let test_result = self.generate_text("Hello").await;
        let latency_ms = start_time.elapsed().as_millis() as u64;
        
        match test_result {
            Ok(_) => Ok(RigHealthStatus {
                status: "healthy".to_string(),
                model: self.get_completion_model_name(),
                version: Some("rig-0.20".to_string()),
                latency_ms,
            }),
            Err(e) => {
                warn!("Rig 健康检查失败: {}", e);
                Err(AiStudioError::ai(format!("Rig 服务不可用: {}", e)))
            }
        }
    }
    
    /// 创建 Agent
    #[cfg(feature = "ai")]
    pub fn create_agent(&self, system_prompt: &str) -> Agent {
        Agent::new(self.completion_model.clone(), system_prompt)
    }
    
    #[cfg(not(feature = "ai"))]
    pub fn create_agent(&self, _system_prompt: &str) -> Result<(), AiStudioError> {
        Err(AiStudioError::ai("AI 功能未启用"))
    }
    
    /// 获取完成模型名称
    fn get_completion_model_name(&self) -> String {
        if self.config.model_endpoint.contains("openai") {
            "gpt-3.5-turbo".to_string()
        } else if self.config.model_endpoint.contains("ollama") {
            "llama2".to_string()
        } else {
            "unknown".to_string()
        }
    }
    
    /// 获取嵌入模型名称
    fn get_embedding_model_name(&self) -> String {
        if self.config.model_endpoint.contains("openai") {
            "text-embedding-ada-002".to_string()
        } else if self.config.model_endpoint.contains("ollama") {
            "nomic-embed-text".to_string()
        } else {
            "unknown".to_string()
        }
    }
    
    /// 获取配置
    pub fn config(&self) -> Arc<AiConfig> {
        self.config.clone()
    }
}

/// Rig AI 客户端管理器
#[derive(Clone)]
pub struct RigAiClientManager {
    client: Arc<RigAiClient>,
}

impl RigAiClientManager {
    /// 创建新的 Rig AI 客户端管理器
    pub async fn new(config: AiConfig) -> Result<Self, AiStudioError> {
        let client = Arc::new(RigAiClient::new(config).await?);
        
        Ok(Self { client })
    }
    
    /// 获取客户端
    pub fn client(&self) -> Arc<RigAiClient> {
        self.client.clone()
    }
    
    /// 生成文本
    pub async fn generate_text(&self, prompt: &str) -> Result<RigGenerationResponse, AiStudioError> {
        self.client.generate_text(prompt).await
    }
    
    /// 生成嵌入向量
    pub async fn generate_embedding(&self, text: &str) -> Result<RigEmbeddingResponse, AiStudioError> {
        self.client.generate_embedding(text).await
    }
    
    /// 批量生成嵌入向量
    pub async fn generate_embeddings(&self, texts: &[String]) -> Result<Vec<RigEmbeddingResponse>, AiStudioError> {
        self.client.generate_embeddings(texts).await
    }
    
    /// 健康检查
    pub async fn health_check(&self) -> Result<RigHealthStatus, AiStudioError> {
        self.client.health_check().await
    }
    
    /// 创建 Agent
    #[cfg(feature = "ai")]
    pub fn create_agent(&self, system_prompt: &str) -> Agent {
        self.client.create_agent(system_prompt)
    }
    
    #[cfg(not(feature = "ai"))]
    pub fn create_agent(&self, _system_prompt: &str) -> Result<(), AiStudioError> {
        self.client.create_agent(_system_prompt)
    }
    
    /// 执行带重试的操作
    pub async fn with_retry<F, T>(&self, operation: F) -> Result<T, AiStudioError>
    where
        F: Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, AiStudioError>> + Send>> + Send + Sync,
        T: Send,
    {
        let config = self.client.config();
        let mut last_error = None;
        
        for attempt in 1..=config.retry_attempts {
            match tokio::time::timeout(
                std::time::Duration::from_secs(config.timeout),
                operation()
            ).await {
                Ok(Ok(result)) => {
                    if attempt > 1 {
                        info!("操作在第 {} 次尝试后成功", attempt);
                    }
                    return Ok(result);
                }
                Ok(Err(e)) => {
                    warn!("第 {} 次尝试失败: {}", attempt, e);
                    last_error = Some(e);
                    
                    if attempt < config.retry_attempts {
                        let delay = std::time::Duration::from_millis(1000 * attempt as u64);
                        tokio::time::sleep(delay).await;
                    }
                }
                Err(_) => {
                    let timeout_error = AiStudioError::timeout(format!(
                        "AI 操作超时 ({}s)", 
                        config.timeout
                    ));
                    warn!("第 {} 次尝试超时", attempt);
                    last_error = Some(timeout_error);
                    
                    if attempt < config.retry_attempts {
                        let delay = std::time::Duration::from_millis(1000 * attempt as u64);
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }
        
        Err(last_error.unwrap_or_else(|| {
            AiStudioError::ai("所有重试尝试都失败了".to_string())
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AiConfig;
    
    fn create_test_config() -> AiConfig {
        AiConfig {
            model_endpoint: "mock://test".to_string(),
            api_key: "test_key".to_string(),
            max_tokens: 1000,
            temperature: 0.7,
            timeout: 30,
            retry_attempts: 3,
        }
    }
    
    #[tokio::test]
    async fn test_rig_client_creation() {
        let config = create_test_config();
        
        // 注意：这个测试在没有真实 AI 服务的情况下会失败
        // 在实际环境中需要配置真实的 AI 服务端点
        match RigAiClient::new(config).await {
            Ok(_) => {
                // 如果成功创建，说明 Rig 集成正常
                println!("Rig 客户端创建成功");
            }
            Err(e) => {
                // 预期的错误，因为没有真实的 AI 服务
                println!("预期的错误: {}", e);
            }
        }
    }
    
    #[tokio::test]
    async fn test_rig_client_manager() {
        let config = create_test_config();
        
        match RigAiClientManager::new(config).await {
            Ok(_manager) => {
                println!("Rig 客户端管理器创建成功");
            }
            Err(e) => {
                println!("预期的错误: {}", e);
            }
        }
    }
}