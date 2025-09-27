// AI 模块集成测试

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AiConfig;
    use std::sync::Arc;
    
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
    async fn test_ai_client_manager_creation() {
        let config = create_test_config();
        let manager = AiClientManager::new(config);
        
        assert!(manager.is_ok());
        
        let manager = manager.unwrap();
        let client = manager.client();
        
        // 测试健康检查
        let health_result = client.health_check().await;
        assert!(health_result.is_ok());
    }
    
    #[tokio::test]
    async fn test_mock_client_text_generation() {
        let config = Arc::new(create_test_config());
        let client = MockAiClient::new(config);
        
        let result = client.generate_text("Hello, world!").await;
        assert!(result.is_ok());
        
        let response = result.unwrap();
        assert!(!response.text.is_empty());
        assert_eq!(response.model, "mock-model");
        assert!(response.tokens_used > 0);
    }
    
    #[tokio::test]
    async fn test_mock_client_embedding_generation() {
        let config = Arc::new(create_test_config());
        let client = MockAiClient::new(config);
        
        let result = client.generate_embedding("Test text for embedding").await;
        assert!(result.is_ok());
        
        let response = result.unwrap();
        assert!(!response.embedding.is_empty());
        assert_eq!(response.model, "mock-embedding");
    }
    
    #[tokio::test]
    async fn test_model_manager() {
        let manager = ModelManager::new();
        
        // 测试获取模型
        let text_models = manager.get_models_by_type(&ModelType::TextGeneration);
        assert!(!text_models.is_empty());
        
        let embedding_models = manager.get_models_by_type(&ModelType::Embedding);
        assert!(!embedding_models.is_empty());
        
        // 测试活跃模型
        let active_text_model = manager.get_active_model(&ModelType::TextGeneration);
        assert!(active_text_model.is_some());
        
        let active_embedding_model = manager.get_active_model(&ModelType::Embedding);
        assert!(active_embedding_model.is_some());
    }
    
    #[tokio::test]
    async fn test_health_checker() {
        let config = create_test_config();
        let client_manager = Arc::new(AiClientManager::new(config).unwrap());
        let model_manager = Arc::new(ModelManager::new());
        
        let health_checker = AiHealthChecker::new(client_manager, model_manager)
            .with_timeout(std::time::Duration::from_secs(5));
        
        let health_status = health_checker.check_health().await;
        assert!(health_status.is_ok());
        
        let status = health_status.unwrap();
        assert!(!status.models.is_empty());
        assert!(!status.services.is_empty());
        assert!(status.check_duration_ms > 0);
    }
    
    #[tokio::test]
    async fn test_batch_embedding_generation() {
        let config = Arc::new(create_test_config());
        let client = MockAiClient::new(config);
        
        let texts = vec![
            "First text".to_string(),
            "Second text".to_string(),
            "Third text".to_string(),
        ];
        
        let result = client.generate_embeddings(&texts).await;
        assert!(result.is_ok());
        
        let responses = result.unwrap();
        assert_eq!(responses.len(), 3);
        
        for response in responses {
            assert!(!response.embedding.is_empty());
            assert_eq!(response.model, "mock-embedding");
        }
    }
    
    #[test]
    fn test_model_switching() {
        let mut manager = ModelManager::new();
        
        // 测试切换到存在的模型
        let result = manager.set_active_model(
            ModelType::TextGeneration,
            "openai/gpt-3.5-turbo".to_string(),
        );
        assert!(result.is_ok());
        
        let active_model = manager.get_active_model(&ModelType::TextGeneration);
        assert!(active_model.is_some());
        assert_eq!(active_model.unwrap().id, "openai/gpt-3.5-turbo");
        
        // 测试切换到不存在的模型
        let result = manager.set_active_model(
            ModelType::TextGeneration,
            "nonexistent/model".to_string(),
        );
        assert!(result.is_err());
    }
    
    #[test]
    fn test_model_selection_strategies() {
        let manager = ModelManager::new();
        let criteria = SelectionCriteria::default();
        
        // 测试手动选择策略
        let selected = manager.select_best_model(&ModelType::TextGeneration, &criteria);
        assert!(selected.is_some());
        
        // 测试故障转移策略
        let mut manager = manager;
        manager.set_switch_strategy(ModelSwitchStrategy::Failover);
        
        let selected = manager.select_best_model(&ModelType::TextGeneration, &criteria);
        assert!(selected.is_some());
    }
}