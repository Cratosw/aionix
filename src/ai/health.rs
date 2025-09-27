// AI 服务健康检查模块
// 监控 AI 模型和服务的健康状态

use crate::ai::{AiClient, AiClientManager, RigAiClientManager, ModelManager, ModelType};
use crate::errors::AiStudioError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::timeout;
use tracing::{debug, error, info, warn};

/// AI 服务健康状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiHealthStatus {
    pub overall_status: HealthLevel,
    pub models: HashMap<String, ModelHealthStatus>,
    pub services: HashMap<String, ServiceHealthStatus>,
    pub last_check: chrono::DateTime<chrono::Utc>,
    pub check_duration_ms: u64,
}

/// 模型健康状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelHealthStatus {
    pub model_id: String,
    pub status: HealthLevel,
    pub latency_ms: u64,
    pub error_message: Option<String>,
    pub last_successful_request: Option<chrono::DateTime<chrono::Utc>>,
    pub consecutive_failures: u32,
    pub availability_percentage: f64,
}

/// 服务健康状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceHealthStatus {
    pub service_name: String,
    pub status: HealthLevel,
    pub endpoint: String,
    pub latency_ms: u64,
    pub error_message: Option<String>,
    pub version: Option<String>,
}

/// 健康等级
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HealthLevel {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

/// AI 健康检查器
pub struct AiHealthChecker {
    client_manager: Arc<RigAiClientManager>,
    model_manager: Arc<ModelManager>,
    check_interval: Duration,
    timeout_duration: Duration,
    failure_threshold: u32,
}

impl AiHealthChecker {
    /// 创建新的健康检查器
    pub fn new(
        client_manager: Arc<RigAiClientManager>,
        model_manager: Arc<ModelManager>,
    ) -> Self {
        Self {
            client_manager,
            model_manager,
            check_interval: Duration::from_secs(30),
            timeout_duration: Duration::from_secs(10),
            failure_threshold: 3,
        }
    }
    
    /// 设置检查间隔
    pub fn with_check_interval(mut self, interval: Duration) -> Self {
        self.check_interval = interval;
        self
    }
    
    /// 设置超时时间
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout_duration = timeout;
        self
    }
    
    /// 设置失败阈值
    pub fn with_failure_threshold(mut self, threshold: u32) -> Self {
        self.failure_threshold = threshold;
        self
    }
    
    /// 执行完整的健康检查
    pub async fn check_health(&self) -> Result<AiHealthStatus, AiStudioError> {
        let start_time = Instant::now();
        info!("开始 AI 服务健康检查");
        
        let mut models = HashMap::new();
        let mut services = HashMap::new();
        
        // 检查所有注册的模型
        for (model_id, model_info) in self.model_manager.get_all_models() {
            debug!("检查模型健康状态: {}", model_id);
            
            let model_health = self.check_model_health(model_id, &model_info.model_type).await;
            models.insert(model_id.clone(), model_health);
        }
        
        // 检查 AI 服务
        let service_health = self.check_service_health().await;
        services.insert("ai_client".to_string(), service_health);
        
        let check_duration = start_time.elapsed();
        let overall_status = self.calculate_overall_status(&models, &services);
        
        let health_status = AiHealthStatus {
            overall_status,
            models,
            services,
            last_check: chrono::Utc::now(),
            check_duration_ms: check_duration.as_millis() as u64,
        };
        
        info!(
            "AI 服务健康检查完成，总体状态: {:?}，耗时: {}ms",
            health_status.overall_status,
            health_status.check_duration_ms
        );
        
        Ok(health_status)
    }
    
    /// 检查单个模型的健康状态
    async fn check_model_health(&self, model_id: &str, model_type: &ModelType) -> ModelHealthStatus {
        let start_time = Instant::now();
        
        let (status, error_message) = match self.perform_model_health_check(model_type).await {
            Ok(_) => (HealthLevel::Healthy, None),
            Err(e) => {
                warn!("模型 {} 健康检查失败: {}", model_id, e);
                (HealthLevel::Unhealthy, Some(e.to_string()))
            }
        };
        
        let latency_ms = start_time.elapsed().as_millis() as u64;
        
        ModelHealthStatus {
            model_id: model_id.to_string(),
            status: status.clone(),
            latency_ms,
            error_message,
            last_successful_request: if status == HealthLevel::Healthy {
                Some(chrono::Utc::now())
            } else {
                None
            },
            consecutive_failures: if status == HealthLevel::Healthy { 0 } else { 1 },
            availability_percentage: if status == HealthLevel::Healthy { 100.0 } else { 0.0 },
        }
    }
    
    /// 执行模型健康检查
    async fn perform_model_health_check(&self, model_type: &ModelType) -> Result<(), AiStudioError> {
        let client = self.client_manager.client();
        
        match model_type {
            ModelType::TextGeneration => {
                // 测试文本生成
                let test_prompt = "Hello, this is a health check.";
                timeout(self.timeout_duration, client.generate_text(test_prompt)).await
                    .map_err(|_| AiStudioError::timeout("文本生成健康检查"))?
                    .map(|_| ())
            }
            ModelType::Embedding => {
                // 测试嵌入生成
                let test_text = "This is a test for embedding generation.";
                timeout(self.timeout_duration, client.generate_embedding(test_text)).await
                    .map_err(|_| AiStudioError::timeout("嵌入生成健康检查"))?
                    .map(|_| ())
            }
            ModelType::Multimodal => {
                // 对于多模态模型，暂时只测试文本生成
                let test_prompt = "Describe this image: [health check]";
                timeout(self.timeout_duration, client.generate_text(test_prompt)).await
                    .map_err(|_| AiStudioError::timeout("多模态模型健康检查"))?
                    .map(|_| ())
            }
        }
    }
    
    /// 检查 AI 服务健康状态
    async fn check_service_health(&self) -> ServiceHealthStatus {
        let start_time = Instant::now();
        let client = self.client_manager.client();
        
        let (status, error_message, version) = match timeout(
            self.timeout_duration,
            client.health_check()
        ).await {
            Ok(Ok(health_status)) => (
                HealthLevel::Healthy,
                None,
                health_status.version,
            ),
            Ok(Err(e)) => (
                HealthLevel::Unhealthy,
                Some(e.to_string()),
                None,
            ),
            Err(_) => (
                HealthLevel::Unhealthy,
                Some("健康检查超时".to_string()),
                None,
            ),
        };
        
        let latency_ms = start_time.elapsed().as_millis() as u64;
        
        ServiceHealthStatus {
            service_name: "AI Client".to_string(),
            status,
            endpoint: self.client_manager.config().model_endpoint.clone(),
            latency_ms,
            error_message,
            version,
        }
    }
    
    /// 计算总体健康状态
    fn calculate_overall_status(
        &self,
        models: &HashMap<String, ModelHealthStatus>,
        services: &HashMap<String, ServiceHealthStatus>,
    ) -> HealthLevel {
        let mut healthy_count = 0;
        let mut degraded_count = 0;
        let mut unhealthy_count = 0;
        let mut total_count = 0;
        
        // 统计模型状态
        for model_health in models.values() {
            total_count += 1;
            match model_health.status {
                HealthLevel::Healthy => healthy_count += 1,
                HealthLevel::Degraded => degraded_count += 1,
                HealthLevel::Unhealthy => unhealthy_count += 1,
                HealthLevel::Unknown => {}
            }
        }
        
        // 统计服务状态
        for service_health in services.values() {
            total_count += 1;
            match service_health.status {
                HealthLevel::Healthy => healthy_count += 1,
                HealthLevel::Degraded => degraded_count += 1,
                HealthLevel::Unhealthy => unhealthy_count += 1,
                HealthLevel::Unknown => {}
            }
        }
        
        if total_count == 0 {
            return HealthLevel::Unknown;
        }
        
        let healthy_percentage = (healthy_count as f64 / total_count as f64) * 100.0;
        
        if healthy_percentage >= 80.0 {
            HealthLevel::Healthy
        } else if healthy_percentage >= 50.0 {
            HealthLevel::Degraded
        } else {
            HealthLevel::Unhealthy
        }
    }
    
    /// 启动定期健康检查
    pub async fn start_periodic_checks(&self) -> Result<(), AiStudioError> {
        info!("启动定期 AI 健康检查，间隔: {:?}", self.check_interval);
        
        let mut interval = tokio::time::interval(self.check_interval);
        
        loop {
            interval.tick().await;
            
            match self.check_health().await {
                Ok(health_status) => {
                    debug!("定期健康检查完成: {:?}", health_status.overall_status);
                    
                    // 如果状态不健康，记录警告
                    if health_status.overall_status != HealthLevel::Healthy {
                        warn!("AI 服务状态异常: {:?}", health_status);
                    }
                }
                Err(e) => {
                    error!("定期健康检查失败: {}", e);
                }
            }
        }
    }
    
    /// 检查特定模型是否可用
    pub async fn is_model_available(&self, model_id: &str) -> bool {
        if let Some(model_info) = self.model_manager.get_model(model_id) {
            self.perform_model_health_check(&model_info.model_type).await.is_ok()
        } else {
            false
        }
    }
    
    /// 获取可用的模型列表
    pub async fn get_available_models(&self, model_type: &ModelType) -> Vec<String> {
        let mut available_models = Vec::new();
        
        for model in self.model_manager.get_models_by_type(model_type) {
            if self.perform_model_health_check(&model.model_type).await.is_ok() {
                available_models.push(model.id.clone());
            }
        }
        
        available_models
    }
}

/// 健康检查配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    pub enabled: bool,
    pub check_interval_seconds: u64,
    pub timeout_seconds: u64,
    pub failure_threshold: u32,
    pub alert_on_failure: bool,
    pub alert_webhook_url: Option<String>,
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            check_interval_seconds: 30,
            timeout_seconds: 10,
            failure_threshold: 3,
            alert_on_failure: true,
            alert_webhook_url: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::{MockAiClient, AiConfig};
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_health_checker_creation() {
        let config = AiConfig {
            model_endpoint: "http://localhost:11434".to_string(),
            api_key: "test".to_string(),
            max_tokens: 1000,
            temperature: 0.7,
            timeout: 30,
            retry_attempts: 3,
        };
        
        let client_manager = Arc::new(AiClientManager::new(config).unwrap());
        let model_manager = Arc::new(ModelManager::new());
        
        let health_checker = AiHealthChecker::new(client_manager, model_manager);
        
        assert_eq!(health_checker.check_interval, Duration::from_secs(30));
        assert_eq!(health_checker.timeout_duration, Duration::from_secs(10));
        assert_eq!(health_checker.failure_threshold, 3);
    }
    
    #[tokio::test]
    async fn test_health_check_execution() {
        let config = AiConfig {
            model_endpoint: "mock://test".to_string(),
            api_key: "test".to_string(),
            max_tokens: 1000,
            temperature: 0.7,
            timeout: 30,
            retry_attempts: 3,
        };
        
        let client_manager = Arc::new(AiClientManager::new(config).unwrap());
        let model_manager = Arc::new(ModelManager::new());
        
        let health_checker = AiHealthChecker::new(client_manager, model_manager)
            .with_timeout(Duration::from_secs(5));
        
        let health_status = health_checker.check_health().await.unwrap();
        
        assert!(!health_status.models.is_empty());
        assert!(!health_status.services.is_empty());
        assert!(health_status.check_duration_ms > 0);
    }
    
    #[test]
    fn test_overall_status_calculation() {
        let config = AiConfig {
            model_endpoint: "mock://test".to_string(),
            api_key: "test".to_string(),
            max_tokens: 1000,
            temperature: 0.7,
            timeout: 30,
            retry_attempts: 3,
        };
        
        let client_manager = Arc::new(AiClientManager::new(config).unwrap());
        let model_manager = Arc::new(ModelManager::new());
        
        let health_checker = AiHealthChecker::new(client_manager, model_manager);
        
        let mut models = HashMap::new();
        let mut services = HashMap::new();
        
        // 所有服务健康
        models.insert("model1".to_string(), ModelHealthStatus {
            model_id: "model1".to_string(),
            status: HealthLevel::Healthy,
            latency_ms: 100,
            error_message: None,
            last_successful_request: Some(chrono::Utc::now()),
            consecutive_failures: 0,
            availability_percentage: 100.0,
        });
        
        services.insert("service1".to_string(), ServiceHealthStatus {
            service_name: "service1".to_string(),
            status: HealthLevel::Healthy,
            endpoint: "http://test".to_string(),
            latency_ms: 50,
            error_message: None,
            version: None,
        });
        
        let overall_status = health_checker.calculate_overall_status(&models, &services);
        assert_eq!(overall_status, HealthLevel::Healthy);
        
        // 部分服务不健康
        models.insert("model2".to_string(), ModelHealthStatus {
            model_id: "model2".to_string(),
            status: HealthLevel::Unhealthy,
            latency_ms: 0,
            error_message: Some("连接失败".to_string()),
            last_successful_request: None,
            consecutive_failures: 5,
            availability_percentage: 0.0,
        });
        
        let overall_status = health_checker.calculate_overall_status(&models, &services);
        assert_eq!(overall_status, HealthLevel::Degraded);
    }
}