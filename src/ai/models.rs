// AI 模型定义和管理
// 定义 AI 相关的数据结构和枚举

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// AI 模型类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ModelType {
    /// 文本生成模型
    TextGeneration,
    /// 嵌入模型
    Embedding,
    /// 多模态模型
    Multimodal,
}

/// AI 模型信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub model_type: ModelType,
    pub provider: String,
    pub version: Option<String>,
    pub description: Option<String>,
    pub max_tokens: u32,
    pub context_length: u32,
    pub embedding_dimension: Option<u32>,
    pub supported_languages: Vec<String>,
    pub capabilities: Vec<String>,
    pub pricing: Option<ModelPricing>,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// 模型定价信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPricing {
    pub input_tokens_per_1k: f64,
    pub output_tokens_per_1k: f64,
    pub currency: String,
}

/// AI 模型配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub model_id: String,
    pub temperature: f32,
    pub max_tokens: u32,
    pub top_p: Option<f32>,
    pub top_k: Option<u32>,
    pub frequency_penalty: Option<f32>,
    pub presence_penalty: Option<f32>,
    pub stop_sequences: Vec<String>,
    pub custom_parameters: HashMap<String, serde_json::Value>,
}

/// 模型切换策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModelSwitchStrategy {
    /// 手动切换
    Manual,
    /// 基于负载自动切换
    LoadBased,
    /// 基于成本自动切换
    CostBased,
    /// 基于质量自动切换
    QualityBased,
    /// 故障转移
    Failover,
}

/// 模型管理器
#[derive(Debug, Clone)]
pub struct ModelManager {
    models: HashMap<String, ModelInfo>,
    active_models: HashMap<ModelType, String>,
    switch_strategy: ModelSwitchStrategy,
}

impl ModelManager {
    /// 创建新的模型管理器
    pub fn new() -> Self {
        let mut manager = Self {
            models: HashMap::new(),
            active_models: HashMap::new(),
            switch_strategy: ModelSwitchStrategy::Manual,
        };
        
        // 注册默认模型
        manager.register_default_models();
        
        manager
    }
    
    /// 注册默认模型
    fn register_default_models(&mut self) {
        // Ollama 模型
        self.register_model(ModelInfo {
            id: "ollama/llama2".to_string(),
            name: "Llama 2".to_string(),
            model_type: ModelType::TextGeneration,
            provider: "Ollama".to_string(),
            version: Some("7B".to_string()),
            description: Some("Meta 的开源大语言模型".to_string()),
            max_tokens: 4096,
            context_length: 4096,
            embedding_dimension: None,
            supported_languages: vec!["en".to_string(), "zh".to_string()],
            capabilities: vec!["text-generation".to_string(), "conversation".to_string()],
            pricing: None,
            metadata: HashMap::new(),
        });
        
        self.register_model(ModelInfo {
            id: "ollama/nomic-embed-text".to_string(),
            name: "Nomic Embed Text".to_string(),
            model_type: ModelType::Embedding,
            provider: "Ollama".to_string(),
            version: Some("v1".to_string()),
            description: Some("高质量文本嵌入模型".to_string()),
            max_tokens: 8192,
            context_length: 8192,
            embedding_dimension: Some(768),
            supported_languages: vec!["en".to_string(), "zh".to_string()],
            capabilities: vec!["text-embedding".to_string()],
            pricing: None,
            metadata: HashMap::new(),
        });
        
        // OpenAI 模型
        self.register_model(ModelInfo {
            id: "openai/gpt-3.5-turbo".to_string(),
            name: "GPT-3.5 Turbo".to_string(),
            model_type: ModelType::TextGeneration,
            provider: "OpenAI".to_string(),
            version: Some("0613".to_string()),
            description: Some("OpenAI 的高效对话模型".to_string()),
            max_tokens: 4096,
            context_length: 16384,
            embedding_dimension: None,
            supported_languages: vec!["en".to_string(), "zh".to_string(), "ja".to_string(), "ko".to_string()],
            capabilities: vec!["text-generation".to_string(), "conversation".to_string(), "function-calling".to_string()],
            pricing: Some(ModelPricing {
                input_tokens_per_1k: 0.0015,
                output_tokens_per_1k: 0.002,
                currency: "USD".to_string(),
            }),
            metadata: HashMap::new(),
        });
        
        self.register_model(ModelInfo {
            id: "openai/text-embedding-ada-002".to_string(),
            name: "Text Embedding Ada 002".to_string(),
            model_type: ModelType::Embedding,
            provider: "OpenAI".to_string(),
            version: Some("002".to_string()),
            description: Some("OpenAI 的文本嵌入模型".to_string()),
            max_tokens: 8191,
            context_length: 8191,
            embedding_dimension: Some(1536),
            supported_languages: vec!["en".to_string(), "zh".to_string()],
            capabilities: vec!["text-embedding".to_string()],
            pricing: Some(ModelPricing {
                input_tokens_per_1k: 0.0001,
                output_tokens_per_1k: 0.0,
                currency: "USD".to_string(),
            }),
            metadata: HashMap::new(),
        });
        
        // 设置默认活跃模型
        self.active_models.insert(ModelType::TextGeneration, "ollama/llama2".to_string());
        self.active_models.insert(ModelType::Embedding, "ollama/nomic-embed-text".to_string());
    }
    
    /// 注册模型
    pub fn register_model(&mut self, model: ModelInfo) {
        self.models.insert(model.id.clone(), model);
    }
    
    /// 获取模型信息
    pub fn get_model(&self, model_id: &str) -> Option<&ModelInfo> {
        self.models.get(model_id)
    }
    
    /// 获取所有模型
    pub fn get_all_models(&self) -> &HashMap<String, ModelInfo> {
        &self.models
    }
    
    /// 获取指定类型的模型
    pub fn get_models_by_type(&self, model_type: &ModelType) -> Vec<&ModelInfo> {
        self.models
            .values()
            .filter(|model| &model.model_type == model_type)
            .collect()
    }
    
    /// 获取活跃模型
    pub fn get_active_model(&self, model_type: &ModelType) -> Option<&ModelInfo> {
        self.active_models
            .get(model_type)
            .and_then(|model_id| self.models.get(model_id))
    }
    
    /// 设置活跃模型
    pub fn set_active_model(&mut self, model_type: ModelType, model_id: String) -> Result<(), String> {
        if !self.models.contains_key(&model_id) {
            return Err(format!("模型 {} 不存在", model_id));
        }
        
        let model = &self.models[&model_id];
        if model.model_type != model_type {
            return Err(format!("模型类型不匹配，期望 {:?}，实际 {:?}", model_type, model.model_type));
        }
        
        self.active_models.insert(model_type, model_id);
        Ok(())
    }
    
    /// 设置切换策略
    pub fn set_switch_strategy(&mut self, strategy: ModelSwitchStrategy) {
        self.switch_strategy = strategy;
    }
    
    /// 获取切换策略
    pub fn get_switch_strategy(&self) -> &ModelSwitchStrategy {
        &self.switch_strategy
    }
    
    /// 根据策略选择最佳模型
    pub fn select_best_model(&self, model_type: &ModelType, criteria: &SelectionCriteria) -> Option<&ModelInfo> {
        let candidates = self.get_models_by_type(model_type);
        
        if candidates.is_empty() {
            return None;
        }
        
        match &self.switch_strategy {
            ModelSwitchStrategy::Manual => self.get_active_model(model_type),
            ModelSwitchStrategy::LoadBased => {
                // 选择负载最低的模型
                candidates.into_iter()
                    .min_by_key(|model| criteria.load_metrics.get(&model.id).unwrap_or(&0))
            }
            ModelSwitchStrategy::CostBased => {
                // 选择成本最低的模型
                candidates.into_iter()
                    .filter(|model| model.pricing.is_some())
                    .min_by(|a, b| {
                        let cost_a = a.pricing.as_ref().unwrap().input_tokens_per_1k;
                        let cost_b = b.pricing.as_ref().unwrap().input_tokens_per_1k;
                        cost_a.partial_cmp(&cost_b).unwrap_or(std::cmp::Ordering::Equal)
                    })
            }
            ModelSwitchStrategy::QualityBased => {
                // 选择质量最高的模型（基于评分）
                candidates.into_iter()
                    .max_by_key(|model| criteria.quality_scores.get(&model.id).unwrap_or(&0))
            }
            ModelSwitchStrategy::Failover => {
                // 选择第一个可用的模型
                let available_model = candidates.iter()
                    .find(|model| *criteria.availability.get(&model.id).unwrap_or(&false))
                    .copied();
                
                available_model.or_else(|| candidates.first().copied())
            }
        }
    }
}

/// 模型选择标准
#[derive(Debug, Clone, Default)]
pub struct SelectionCriteria {
    pub load_metrics: HashMap<String, u32>,
    pub quality_scores: HashMap<String, u32>,
    pub availability: HashMap<String, bool>,
    pub latency_ms: HashMap<String, u64>,
    pub cost_budget: Option<f64>,
}

impl Default for ModelManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 模型使用统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelUsageStats {
    pub model_id: String,
    pub tenant_id: Option<Uuid>,
    pub total_requests: u64,
    pub total_tokens: u64,
    pub total_cost: f64,
    pub average_latency_ms: f64,
    pub error_count: u64,
    pub last_used: chrono::DateTime<chrono::Utc>,
}

/// 模型性能指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMetrics {
    pub model_id: String,
    pub requests_per_second: f64,
    pub average_latency_ms: f64,
    pub p95_latency_ms: f64,
    pub p99_latency_ms: f64,
    pub error_rate: f64,
    pub throughput_tokens_per_second: f64,
    pub memory_usage_mb: f64,
    pub cpu_usage_percent: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_model_manager_creation() {
        let manager = ModelManager::new();
        
        // 检查默认模型是否注册
        assert!(manager.get_model("ollama/llama2").is_some());
        assert!(manager.get_model("openai/gpt-3.5-turbo").is_some());
        
        // 检查活跃模型是否设置
        assert!(manager.get_active_model(&ModelType::TextGeneration).is_some());
        assert!(manager.get_active_model(&ModelType::Embedding).is_some());
    }
    
    #[test]
    fn test_model_type_filtering() {
        let manager = ModelManager::new();
        
        let text_models = manager.get_models_by_type(&ModelType::TextGeneration);
        let embedding_models = manager.get_models_by_type(&ModelType::Embedding);
        
        assert!(!text_models.is_empty());
        assert!(!embedding_models.is_empty());
        
        // 验证模型类型正确
        for model in text_models {
            assert_eq!(model.model_type, ModelType::TextGeneration);
        }
        
        for model in embedding_models {
            assert_eq!(model.model_type, ModelType::Embedding);
        }
    }
    
    #[test]
    fn test_active_model_switching() {
        let mut manager = ModelManager::new();
        
        // 切换到 OpenAI 模型
        let result = manager.set_active_model(
            ModelType::TextGeneration,
            "openai/gpt-3.5-turbo".to_string(),
        );
        assert!(result.is_ok());
        
        let active_model = manager.get_active_model(&ModelType::TextGeneration);
        assert!(active_model.is_some());
        assert_eq!(active_model.unwrap().id, "openai/gpt-3.5-turbo");
    }
    
    #[test]
    fn test_invalid_model_switching() {
        let mut manager = ModelManager::new();
        
        // 尝试切换到不存在的模型
        let result = manager.set_active_model(
            ModelType::TextGeneration,
            "nonexistent/model".to_string(),
        );
        assert!(result.is_err());
        
        // 尝试切换到错误类型的模型
        let result = manager.set_active_model(
            ModelType::TextGeneration,
            "ollama/nomic-embed-text".to_string(), // 这是嵌入模型
        );
        assert!(result.is_err());
    }
}