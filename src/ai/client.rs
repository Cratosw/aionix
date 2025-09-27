// AI 客户端模块
// 集成 Rig 框架和 LLM 客户端配置

use crate::config::AiConfig;
use crate::errors::AiStudioError;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;
use tracing::{debug, error, info, warn};

/// AI 客户端特征
#[async_trait]
pub trait AiClient: Send + Sync {
    /// 生成文本
    async fn generate_text(&self, prompt: &str) -> Result<GenerationResponse, AiStudioError>;
    
    /// 生成嵌入向量
    async fn generate_embedding(&self, text: &str) -> Result<EmbeddingResponse, AiStudioError>;
    
    /// 批量生成嵌入向量
    async fn generate_embeddings(&self, texts: &[String]) -> Result<Vec<EmbeddingResponse>, AiStudioError>;
    
    /// 检查模型健康状态
    async fn health_check(&self) -> Result<HealthStatus, AiStudioError>;
}

/// 生成响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationResponse {
    pub text: String,
    pub model: String,
    pub tokens_used: u32,
    pub finish_reason: String,
    pub metadata: serde_json::Value,
}

/// 嵌入响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingResponse {
    pub embedding: Vec<f32>,
    pub model: String,
    pub tokens_used: u32,
    pub metadata: serde_json::Value,
}

/// 健康状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub status: String,
    pub model: String,
    pub version: Option<String>,
    pub latency_ms: u64,
}

/// AI 客户端管理器
#[derive(Clone)]
pub struct AiClientManager {
    config: Arc<AiConfig>,
    client: Arc<dyn AiClient>,
}

impl AiClientManager {
    /// 创建新的 AI 客户端管理器
    pub fn new(config: AiConfig) -> Result<Self, AiStudioError> {
        let config = Arc::new(config);
        
        // 根据配置创建相应的客户端
        let client: Arc<dyn AiClient> = if config.model_endpoint.contains("ollama") {
            Arc::new(OllamaClient::new(config.clone())?)
        } else if config.model_endpoint.contains("openai") {
            Arc::new(OpenAiClient::new(config.clone())?)
        } else {
            // 默认使用 Mock 客户端用于测试
            Arc::new(MockAiClient::new(config.clone()))
        };
        
        info!("AI 客户端管理器初始化完成，端点: {}", config.model_endpoint);
        
        Ok(Self { config, client })
    }
    
    /// 获取客户端
    pub fn client(&self) -> Arc<dyn AiClient> {
        self.client.clone()
    }
    
    /// 获取配置
    pub fn config(&self) -> Arc<AiConfig> {
        self.config.clone()
    }
    
    /// 执行带重试的操作
    pub async fn with_retry<F, T>(&self, operation: F) -> Result<T, AiStudioError>
    where
        F: Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, AiStudioError>> + Send>> + Send + Sync,
        T: Send,
    {
        let mut last_error = None;
        
        for attempt in 1..=self.config.retry_attempts {
            match timeout(Duration::from_secs(self.config.timeout), operation()).await {
                Ok(Ok(result)) => {
                    if attempt > 1 {
                        info!("操作在第 {} 次尝试后成功", attempt);
                    }
                    return Ok(result);
                }
                Ok(Err(e)) => {
                    warn!("第 {} 次尝试失败: {}", attempt, e);
                    last_error = Some(e);
                    
                    if attempt < self.config.retry_attempts {
                        let delay = Duration::from_millis(1000 * attempt as u64);
                        tokio::time::sleep(delay).await;
                    }
                }
                Err(_) => {
                    let timeout_error = AiStudioError::timeout(format!(
                        "AI 操作超时 ({}s)", 
                        self.config.timeout
                    ));
                    warn!("第 {} 次尝试超时", attempt);
                    last_error = Some(timeout_error);
                    
                    if attempt < self.config.retry_attempts {
                        let delay = Duration::from_millis(1000 * attempt as u64);
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

/// Ollama 客户端实现
pub struct OllamaClient {
    config: Arc<AiConfig>,
    http_client: reqwest::Client,
}

impl OllamaClient {
    pub fn new(config: Arc<AiConfig>) -> Result<Self, AiStudioError> {
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.timeout))
            .build()
            .map_err(|e| AiStudioError::ai(format!("创建 HTTP 客户端失败: {}", e)))?;
        
        Ok(Self { config, http_client })
    }
}

#[async_trait]
impl AiClient for OllamaClient {
    async fn generate_text(&self, prompt: &str) -> Result<GenerationResponse, AiStudioError> {
        debug!("使用 Ollama 生成文本，提示词长度: {}", prompt.len());
        
        let request_body = serde_json::json!({
            "model": "llama2", // 可以从配置中获取
            "prompt": prompt,
            "stream": false,
            "options": {
                "temperature": self.config.temperature,
                "num_predict": self.config.max_tokens
            }
        });
        
        let response = self.http_client
            .post(&format!("{}/api/generate", self.config.model_endpoint))
            .json(&request_body)
            .send()
            .await
            .map_err(|e| AiStudioError::ai(format!("Ollama 请求失败: {}", e)))?;
        
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AiStudioError::ai(format!("Ollama 错误: {}", error_text)));
        }
        
        let response_json: serde_json::Value = response.json().await
            .map_err(|e| AiStudioError::ai(format!("解析 Ollama 响应失败: {}", e)))?;
        
        Ok(GenerationResponse {
            text: response_json["response"].as_str().unwrap_or("").to_string(),
            model: response_json["model"].as_str().unwrap_or("unknown").to_string(),
            tokens_used: response_json["eval_count"].as_u64().unwrap_or(0) as u32,
            finish_reason: "stop".to_string(),
            metadata: response_json,
        })
    }
    
    async fn generate_embedding(&self, text: &str) -> Result<EmbeddingResponse, AiStudioError> {
        debug!("使用 Ollama 生成嵌入向量，文本长度: {}", text.len());
        
        let request_body = serde_json::json!({
            "model": "nomic-embed-text", // 嵌入模型
            "prompt": text
        });
        
        let response = self.http_client
            .post(&format!("{}/api/embeddings", self.config.model_endpoint))
            .json(&request_body)
            .send()
            .await
            .map_err(|e| AiStudioError::ai(format!("Ollama 嵌入请求失败: {}", e)))?;
        
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AiStudioError::ai(format!("Ollama 嵌入错误: {}", error_text)));
        }
        
        let response_json: serde_json::Value = response.json().await
            .map_err(|e| AiStudioError::ai(format!("解析 Ollama 嵌入响应失败: {}", e)))?;
        
        let embedding: Vec<f32> = response_json["embedding"]
            .as_array()
            .ok_or_else(|| AiStudioError::ai("嵌入向量格式错误".to_string()))?
            .iter()
            .map(|v| v.as_f64().unwrap_or(0.0) as f32)
            .collect();
        
        Ok(EmbeddingResponse {
            embedding,
            model: "nomic-embed-text".to_string(),
            tokens_used: 0, // Ollama 不返回 token 数量
            metadata: response_json,
        })
    }
    
    async fn generate_embeddings(&self, texts: &[String]) -> Result<Vec<EmbeddingResponse>, AiStudioError> {
        let mut results = Vec::new();
        
        // 批量处理，每次处理 10 个文本
        for chunk in texts.chunks(10) {
            let mut chunk_results = Vec::new();
            
            for text in chunk {
                let embedding = self.generate_embedding(text).await?;
                chunk_results.push(embedding);
            }
            
            results.extend(chunk_results);
        }
        
        Ok(results)
    }
    
    async fn health_check(&self) -> Result<HealthStatus, AiStudioError> {
        let start_time = std::time::Instant::now();
        
        let response = self.http_client
            .get(&format!("{}/api/tags", self.config.model_endpoint))
            .send()
            .await
            .map_err(|e| AiStudioError::ai(format!("Ollama 健康检查失败: {}", e)))?;
        
        let latency_ms = start_time.elapsed().as_millis() as u64;
        
        if response.status().is_success() {
            Ok(HealthStatus {
                status: "healthy".to_string(),
                model: "ollama".to_string(),
                version: None,
                latency_ms,
            })
        } else {
            Err(AiStudioError::ai("Ollama 服务不可用".to_string()))
        }
    }
}

/// OpenAI 客户端实现
pub struct OpenAiClient {
    config: Arc<AiConfig>,
    http_client: reqwest::Client,
}

impl OpenAiClient {
    pub fn new(config: Arc<AiConfig>) -> Result<Self, AiStudioError> {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::AUTHORIZATION,
            reqwest::header::HeaderValue::from_str(&format!("Bearer {}", config.api_key))
                .map_err(|e| AiStudioError::ai(format!("无效的 API 密钥: {}", e)))?,
        );
        
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.timeout))
            .default_headers(headers)
            .build()
            .map_err(|e| AiStudioError::ai(format!("创建 HTTP 客户端失败: {}", e)))?;
        
        Ok(Self { config, http_client })
    }
}

#[async_trait]
impl AiClient for OpenAiClient {
    async fn generate_text(&self, prompt: &str) -> Result<GenerationResponse, AiStudioError> {
        debug!("使用 OpenAI 生成文本，提示词长度: {}", prompt.len());
        
        let request_body = serde_json::json!({
            "model": "gpt-3.5-turbo",
            "messages": [
                {"role": "user", "content": prompt}
            ],
            "max_tokens": self.config.max_tokens,
            "temperature": self.config.temperature
        });
        
        let response = self.http_client
            .post("https://api.openai.com/v1/chat/completions")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| AiStudioError::ai(format!("OpenAI 请求失败: {}", e)))?;
        
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AiStudioError::ai(format!("OpenAI 错误: {}", error_text)));
        }
        
        let response_json: serde_json::Value = response.json().await
            .map_err(|e| AiStudioError::ai(format!("解析 OpenAI 响应失败: {}", e)))?;
        
        let choice = &response_json["choices"][0];
        let message = &choice["message"];
        
        Ok(GenerationResponse {
            text: message["content"].as_str().unwrap_or("").to_string(),
            model: response_json["model"].as_str().unwrap_or("unknown").to_string(),
            tokens_used: response_json["usage"]["total_tokens"].as_u64().unwrap_or(0) as u32,
            finish_reason: choice["finish_reason"].as_str().unwrap_or("unknown").to_string(),
            metadata: response_json,
        })
    }
    
    async fn generate_embedding(&self, text: &str) -> Result<EmbeddingResponse, AiStudioError> {
        debug!("使用 OpenAI 生成嵌入向量，文本长度: {}", text.len());
        
        let request_body = serde_json::json!({
            "model": "text-embedding-ada-002",
            "input": text
        });
        
        let response = self.http_client
            .post("https://api.openai.com/v1/embeddings")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| AiStudioError::ai(format!("OpenAI 嵌入请求失败: {}", e)))?;
        
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AiStudioError::ai(format!("OpenAI 嵌入错误: {}", error_text)));
        }
        
        let response_json: serde_json::Value = response.json().await
            .map_err(|e| AiStudioError::ai(format!("解析 OpenAI 嵌入响应失败: {}", e)))?;
        
        let embedding_data = &response_json["data"][0];
        let embedding: Vec<f32> = embedding_data["embedding"]
            .as_array()
            .ok_or_else(|| AiStudioError::ai("嵌入向量格式错误".to_string()))?
            .iter()
            .map(|v| v.as_f64().unwrap_or(0.0) as f32)
            .collect();
        
        Ok(EmbeddingResponse {
            embedding,
            model: "text-embedding-ada-002".to_string(),
            tokens_used: response_json["usage"]["total_tokens"].as_u64().unwrap_or(0) as u32,
            metadata: response_json,
        })
    }
    
    async fn generate_embeddings(&self, texts: &[String]) -> Result<Vec<EmbeddingResponse>, AiStudioError> {
        // OpenAI 支持批量嵌入
        let request_body = serde_json::json!({
            "model": "text-embedding-ada-002",
            "input": texts
        });
        
        let response = self.http_client
            .post("https://api.openai.com/v1/embeddings")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| AiStudioError::ai(format!("OpenAI 批量嵌入请求失败: {}", e)))?;
        
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AiStudioError::ai(format!("OpenAI 批量嵌入错误: {}", error_text)));
        }
        
        let response_json: serde_json::Value = response.json().await
            .map_err(|e| AiStudioError::ai(format!("解析 OpenAI 批量嵌入响应失败: {}", e)))?;
        
        let data = response_json["data"]
            .as_array()
            .ok_or_else(|| AiStudioError::ai("批量嵌入响应格式错误".to_string()))?;
        
        let mut results = Vec::new();
        for item in data {
            let embedding: Vec<f32> = item["embedding"]
                .as_array()
                .ok_or_else(|| AiStudioError::ai("嵌入向量格式错误".to_string()))?
                .iter()
                .map(|v| v.as_f64().unwrap_or(0.0) as f32)
                .collect();
            
            results.push(EmbeddingResponse {
                embedding,
                model: "text-embedding-ada-002".to_string(),
                tokens_used: 0, // 总 token 数在响应的 usage 字段中
                metadata: item.clone(),
            });
        }
        
        Ok(results)
    }
    
    async fn health_check(&self) -> Result<HealthStatus, AiStudioError> {
        let start_time = std::time::Instant::now();
        
        let response = self.http_client
            .get("https://api.openai.com/v1/models")
            .send()
            .await
            .map_err(|e| AiStudioError::ai(format!("OpenAI 健康检查失败: {}", e)))?;
        
        let latency_ms = start_time.elapsed().as_millis() as u64;
        
        if response.status().is_success() {
            Ok(HealthStatus {
                status: "healthy".to_string(),
                model: "openai".to_string(),
                version: None,
                latency_ms,
            })
        } else {
            Err(AiStudioError::ai("OpenAI 服务不可用".to_string()))
        }
    }
}

/// Mock AI 客户端（用于测试）
pub struct MockAiClient {
    config: Arc<AiConfig>,
}

impl MockAiClient {
    pub fn new(config: Arc<AiConfig>) -> Self {
        Self { config }
    }
}

#[async_trait]
impl AiClient for MockAiClient {
    async fn generate_text(&self, prompt: &str) -> Result<GenerationResponse, AiStudioError> {
        debug!("使用 Mock 客户端生成文本，提示词长度: {}", prompt.len());
        
        // 模拟延迟
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        Ok(GenerationResponse {
            text: format!("这是对提示词的模拟回复: {}", prompt.chars().take(50).collect::<String>()),
            model: "mock-model".to_string(),
            tokens_used: prompt.len() as u32 / 4, // 粗略估算
            finish_reason: "stop".to_string(),
            metadata: serde_json::json!({"mock": true}),
        })
    }
    
    async fn generate_embedding(&self, text: &str) -> Result<EmbeddingResponse, AiStudioError> {
        debug!("使用 Mock 客户端生成嵌入向量，文本长度: {}", text.len());
        
        // 模拟延迟
        tokio::time::sleep(Duration::from_millis(50)).await;
        
        // 生成模拟的嵌入向量
        let embedding: Vec<f32> = (0..self.config.max_tokens.min(1536))
            .map(|i| (i as f32 * 0.001).sin())
            .collect();
        
        Ok(EmbeddingResponse {
            embedding,
            model: "mock-embedding".to_string(),
            tokens_used: text.len() as u32 / 4,
            metadata: serde_json::json!({"mock": true}),
        })
    }
    
    async fn generate_embeddings(&self, texts: &[String]) -> Result<Vec<EmbeddingResponse>, AiStudioError> {
        let mut results = Vec::new();
        
        for text in texts {
            let embedding = self.generate_embedding(text).await?;
            results.push(embedding);
        }
        
        Ok(results)
    }
    
    async fn health_check(&self) -> Result<HealthStatus, AiStudioError> {
        Ok(HealthStatus {
            status: "healthy".to_string(),
            model: "mock".to_string(),
            version: Some("1.0.0".to_string()),
            latency_ms: 1,
        })
    }
}