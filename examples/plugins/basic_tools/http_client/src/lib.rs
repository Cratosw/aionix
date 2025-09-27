// HTTP 客户端插件示例
// 提供 HTTP 请求和响应处理功能

use std::collections::HashMap;
use std::time::Duration;
use async_trait::async_trait;
use serde_json;
use uuid::Uuid;
use chrono::Utc;
use reqwest;
use tracing::{debug, info, warn, error};

// 注意：在实际项目中，这些应该从 crate 导入
// use aionix_ai_studio::plugins::plugin_interface::*;
// use aionix_ai_studio::errors::AiStudioError;

// 为了示例，我们重用文件操作插件中的类型定义
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PluginStatus {
    Uninitialized,
    Initializing,
    Initialized,
    Starting,
    Running,
    Stopping,
    Stopped,
    Error,
    Unloading,
    Unloaded,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PluginType {
    Tool,
    Agent,
    Workflow,
    DataSource,
    Authentication,
    Storage,
    Notification,
    Monitoring,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PluginPermission {
    FileSystem,
    Network,
    Database,
    SystemInfo,
    UserData,
    Admin,
    Custom(String),
}

// 简化的错误类型
#[derive(Debug)]
pub struct AiStudioError {
    message: String,
}

impl AiStudioError {
    pub fn validation(msg: &str) -> Self {
        Self { message: msg.to_string() }
    }
    
    pub fn network(msg: String) -> Self {
        Self { message: msg }
    }
    
    pub fn timeout(msg: &str) -> Self {
        Self { message: msg.to_string() }
    }
    
    pub fn internal(msg: &str) -> Self {
        Self { message: msg.to_string() }
    }
}

impl std::fmt::Display for AiStudioError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for AiStudioError {}

// 插件接口定义（简化版）
#[derive(Debug, Clone, Serialize)]
pub struct PluginMetadata {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub license: String,
    pub homepage: Option<String>,
    pub repository: Option<String>,
    pub plugin_type: PluginType,
    pub api_version: String,
    pub min_system_version: String,
    pub dependencies: Vec<PluginDependency>,
    pub permissions: Vec<PluginPermission>,
    pub tags: Vec<String>,
    pub icon: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginDependency {
    pub plugin_id: String,
    pub version_requirement: String,
    pub optional: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    pub plugin_id: String,
    pub parameters: HashMap<String, serde_json::Value>,
    pub environment: HashMap<String, String>,
    pub resource_limits: ResourceLimits,
    pub security_settings: SecuritySettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    pub max_memory_mb: Option<u64>,
    pub max_cpu_percent: Option<f32>,
    pub max_disk_mb: Option<u64>,
    pub max_network_kbps: Option<u64>,
    pub max_execution_seconds: Option<u64>,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_memory_mb: Some(512),
            max_cpu_percent: Some(50.0),
            max_disk_mb: Some(1024),
            max_network_kbps: Some(1024),
            max_execution_seconds: Some(300),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecuritySettings {
    pub enable_sandbox: bool,
    pub allowed_domains: Vec<String>,
    pub allowed_paths: Vec<String>,
    pub forbidden_operations: Vec<String>,
}

impl Default for SecuritySettings {
    fn default() -> Self {
        Self {
            enable_sandbox: true,
            allowed_domains: Vec::new(),
            allowed_paths: Vec::new(),
            forbidden_operations: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PluginContext {
    pub tenant_id: Uuid,
    pub user_id: Option<Uuid>,
    pub session_id: Option<Uuid>,
    pub request_id: Uuid,
    pub variables: HashMap<String, serde_json::Value>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PluginHealth {
    pub healthy: bool,
    pub message: String,
    pub details: HashMap<String, serde_json::Value>,
    pub checked_at: chrono::DateTime<chrono::Utc>,
    pub response_time_ms: u64,
}

#[async_trait]
pub trait Plugin: Send + Sync {
    fn metadata(&self) -> PluginMetadata;
    async fn initialize(&mut self, config: PluginConfig) -> Result<(), AiStudioError>;
    async fn start(&mut self) -> Result<(), AiStudioError>;
    async fn stop(&mut self) -> Result<(), AiStudioError>;
    async fn shutdown(&mut self) -> Result<(), AiStudioError>;
    fn status(&self) -> PluginStatus;
    async fn handle_call(
        &self,
        method: &str,
        params: HashMap<String, serde_json::Value>,
        context: &PluginContext,
    ) -> Result<serde_json::Value, AiStudioError>;
    async fn health_check(&self) -> Result<PluginHealth, AiStudioError>;
    fn config_schema(&self) -> serde_json::Value;
    fn validate_config(&self, config: &PluginConfig) -> Result<(), AiStudioError>;
}

/// HTTP 响应结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: String,
    pub content_type: Option<String>,
    pub content_length: Option<u64>,
    pub response_time_ms: u64,
}

/// HTTP 客户端插件
pub struct HttpClientPlugin {
    status: PluginStatus,
    config: Option<PluginConfig>,
    client: Option<reqwest::Client>,
    default_timeout: Duration,
    allowed_domains: Vec<String>,
    max_response_size: usize,
}

impl HttpClientPlugin {
    /// 创建新的 HTTP 客户端插件实例
    pub fn new() -> Self {
        Self {
            status: PluginStatus::Uninitialized,
            config: None,
            client: None,
            default_timeout: Duration::from_secs(30),
            allowed_domains: Vec::new(),
            max_response_size: 10 * 1024 * 1024, // 10MB
        }
    }
    
    /// 验证 URL 是否被允许访问
    fn validate_url(&self, url: &str) -> Result<reqwest::Url, AiStudioError> {
        let parsed_url = reqwest::Url::parse(url)
            .map_err(|e| AiStudioError::validation(&format!("无效的 URL: {}", e)))?;
        
        // 检查协议
        match parsed_url.scheme() {
            "http" | "https" => {},
            _ => return Err(AiStudioError::validation("只支持 HTTP 和 HTTPS 协议")),
        }
        
        // 检查域名白名单
        if !self.allowed_domains.is_empty() {
            if let Some(host) = parsed_url.host_str() {
                let allowed = self.allowed_domains.iter().any(|domain| {
                    host == domain || host.ends_with(&format!(".{}", domain))
                });
                
                if !allowed {
                    return Err(AiStudioError::validation(&format!("域名不在允许列表中: {}", host)));
                }
            }
        }
        
        Ok(parsed_url)
    }
    
    /// 执行 GET 请求
    async fn get_request(&self, params: &HashMap<String, serde_json::Value>) -> Result<serde_json::Value, AiStudioError> {
        let url = params.get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AiStudioError::validation("缺少参数: url"))?;
        
        let headers = params.get("headers")
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_default();
        
        let timeout_secs = params.get("timeout")
            .and_then(|v| v.as_u64())
            .unwrap_or(self.default_timeout.as_secs());
        
        self.execute_request("GET", url, headers, None, timeout_secs).await
    }
    
    /// 执行 POST 请求
    async fn post_request(&self, params: &HashMap<String, serde_json::Value>) -> Result<serde_json::Value, AiStudioError> {
        let url = params.get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AiStudioError::validation("缺少参数: url"))?;
        
        let headers = params.get("headers")
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_default();
        
        let body = params.get("body")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        
        let timeout_secs = params.get("timeout")
            .and_then(|v| v.as_u64())
            .unwrap_or(self.default_timeout.as_secs());
        
        self.execute_request("POST", url, headers, body, timeout_secs).await
    }
    
    /// 执行 PUT 请求
    async fn put_request(&self, params: &HashMap<String, serde_json::Value>) -> Result<serde_json::Value, AiStudioError> {
        let url = params.get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AiStudioError::validation("缺少参数: url"))?;
        
        let headers = params.get("headers")
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_default();
        
        let body = params.get("body")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        
        let timeout_secs = params.get("timeout")
            .and_then(|v| v.as_u64())
            .unwrap_or(self.default_timeout.as_secs());
        
        self.execute_request("PUT", url, headers, body, timeout_secs).await
    }
    
    /// 执行 DELETE 请求
    async fn delete_request(&self, params: &HashMap<String, serde_json::Value>) -> Result<serde_json::Value, AiStudioError> {
        let url = params.get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AiStudioError::validation("缺少参数: url"))?;
        
        let headers = params.get("headers")
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_default();
        
        let timeout_secs = params.get("timeout")
            .and_then(|v| v.as_u64())
            .unwrap_or(self.default_timeout.as_secs());
        
        self.execute_request("DELETE", url, headers, None, timeout_secs).await
    }
    
    /// 执行通用 HTTP 请求
    async fn execute_request(
        &self,
        method: &str,
        url: &str,
        headers: serde_json::Map<String, serde_json::Value>,
        body: Option<String>,
        timeout_secs: u64,
    ) -> Result<serde_json::Value, AiStudioError> {
        let client = self.client.as_ref()
            .ok_or_else(|| AiStudioError::internal("HTTP 客户端未初始化"))?;
        
        let validated_url = self.validate_url(url)?;
        
        debug!("执行 {} 请求: {}", method, url);
        
        let start_time = std::time::Instant::now();
        
        // 构建请求
        let mut request_builder = match method {
            "GET" => client.get(validated_url),
            "POST" => client.post(validated_url),
            "PUT" => client.put(validated_url),
            "DELETE" => client.delete(validated_url),
            _ => return Err(AiStudioError::validation(&format!("不支持的 HTTP 方法: {}", method))),
        };
        
        // 设置超时
        request_builder = request_builder.timeout(Duration::from_secs(timeout_secs));
        
        // 添加请求头
        for (key, value) in headers {
            if let Some(value_str) = value.as_str() {
                request_builder = request_builder.header(&key, value_str);
            }
        }
        
        // 添加请求体
        if let Some(body_content) = body {
            request_builder = request_builder.body(body_content);
        }
        
        // 发送请求
        let response = request_builder.send().await
            .map_err(|e| {
                if e.is_timeout() {
                    AiStudioError::timeout("请求超时")
                } else {
                    AiStudioError::network(format!("网络请求失败: {}", e))
                }
            })?;
        
        let response_time = start_time.elapsed().as_millis() as u64;
        
        // 获取响应信息
        let status = response.status().as_u16();
        let mut response_headers = HashMap::new();
        
        for (name, value) in response.headers() {
            if let Ok(value_str) = value.to_str() {
                response_headers.insert(name.to_string(), value_str.to_string());
            }
        }
        
        let content_type = response.headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        
        let content_length = response.content_length();
        
        // 检查响应大小限制
        if let Some(length) = content_length {
            if length > self.max_response_size as u64 {
                return Err(AiStudioError::validation(&format!(
                    "响应大小超过限制: {} > {}", 
                    length, 
                    self.max_response_size
                )));
            }
        }
        
        // 读取响应体
        let response_body = response.text().await
            .map_err(|e| AiStudioError::network(format!("读取响应体失败: {}", e)))?;
        
        // 检查实际响应大小
        if response_body.len() > self.max_response_size {
            return Err(AiStudioError::validation(&format!(
                "响应大小超过限制: {} > {}", 
                response_body.len(), 
                self.max_response_size
            )));
        }
        
        info!("HTTP 请求完成: {} {} - 状态: {}, 耗时: {}ms", 
              method, url, status, response_time);
        
        let http_response = HttpResponse {
            status,
            headers: response_headers,
            body: response_body,
            content_type,
            content_length,
            response_time_ms: response_time,
        };
        
        Ok(serde_json::to_value(http_response)
            .map_err(|e| AiStudioError::internal(&format!("序列化响应失败: {}", e)))?)
    }
    
    /// 下载文件
    async fn download_file(&self, params: &HashMap<String, serde_json::Value>) -> Result<serde_json::Value, AiStudioError> {
        let url = params.get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AiStudioError::validation("缺少参数: url"))?;
        
        let max_size = params.get("max_size")
            .and_then(|v| v.as_u64())
            .unwrap_or(self.max_response_size as u64);
        
        let client = self.client.as_ref()
            .ok_or_else(|| AiStudioError::internal("HTTP 客户端未初始化"))?;
        
        let validated_url = self.validate_url(url)?;
        
        debug!("下载文件: {}", url);
        
        let start_time = std::time::Instant::now();
        
        let response = client.get(validated_url)
            .timeout(self.default_timeout)
            .send().await
            .map_err(|e| AiStudioError::network(format!("下载失败: {}", e)))?;
        
        if !response.status().is_success() {
            return Err(AiStudioError::network(format!("下载失败，状态码: {}", response.status())));
        }
        
        let content_length = response.content_length().unwrap_or(0);
        if content_length > max_size {
            return Err(AiStudioError::validation(&format!(
                "文件大小超过限制: {} > {}", 
                content_length, 
                max_size
            )));
        }
        
        let bytes = response.bytes().await
            .map_err(|e| AiStudioError::network(format!("读取文件数据失败: {}", e)))?;
        
        if bytes.len() > max_size as usize {
            return Err(AiStudioError::validation(&format!(
                "文件大小超过限制: {} > {}", 
                bytes.len(), 
                max_size
            )));
        }
        
        let response_time = start_time.elapsed().as_millis() as u64;
        
        info!("文件下载完成: {} - 大小: {} 字节, 耗时: {}ms", 
              url, bytes.len(), response_time);
        
        Ok(serde_json::json!({
            "success": true,
            "url": url,
            "size": bytes.len(),
            "data": base64::encode(&bytes),
            "response_time_ms": response_time
        }))
    }
}#[
async_trait]
impl Plugin for HttpClientPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: "http-client".to_string(),
            name: "HTTP 客户端插件".to_string(),
            version: "1.0.0".to_string(),
            description: "提供 HTTP 请求和响应处理功能，支持 GET、POST、PUT、DELETE 等方法".to_string(),
            author: "Aionix AI Studio".to_string(),
            license: "MIT".to_string(),
            homepage: Some("https://github.com/aionix/ai-studio".to_string()),
            repository: Some("https://github.com/aionix/ai-studio".to_string()),
            plugin_type: PluginType::Tool,
            api_version: "1.0".to_string(),
            min_system_version: "1.0.0".to_string(),
            dependencies: Vec::new(),
            permissions: vec![PluginPermission::Network],
            tags: vec![
                "http".to_string(),
                "client".to_string(),
                "network".to_string(),
                "api".to_string(),
                "web".to_string()
            ],
            icon: Some("🌐".to_string()),
            created_at: Utc::now(),
        }
    }
    
    async fn initialize(&mut self, config: PluginConfig) -> Result<(), AiStudioError> {
        info!("初始化 HTTP 客户端插件");
        
        // 验证配置
        self.validate_config(&config)?;
        
        // 设置超时时间
        if let Some(timeout) = config.parameters.get("default_timeout_seconds") {
            if let Some(timeout_val) = timeout.as_u64() {
                self.default_timeout = Duration::from_secs(timeout_val);
                info!("设置默认超时时间: {} 秒", timeout_val);
            }
        }
        
        // 设置允许的域名
        if let Some(domains) = config.parameters.get("allowed_domains") {
            if let Some(domains_array) = domains.as_array() {
                self.allowed_domains = domains_array
                    .iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect();
                info!("设置允许的域名: {:?}", self.allowed_domains);
            }
        }
        
        // 设置最大响应大小
        if let Some(max_size) = config.parameters.get("max_response_size_mb") {
            if let Some(size_val) = max_size.as_u64() {
                self.max_response_size = (size_val * 1024 * 1024) as usize;
                info!("设置最大响应大小: {} MB", size_val);
            }
        }
        
        // 创建 HTTP 客户端
        let mut client_builder = reqwest::Client::builder()
            .timeout(self.default_timeout)
            .user_agent("Aionix-AI-Studio-HTTP-Plugin/1.0");
        
        // 设置代理（如果配置了）
        if let Some(proxy_url) = config.parameters.get("proxy_url") {
            if let Some(proxy_str) = proxy_url.as_str() {
                if let Ok(proxy) = reqwest::Proxy::all(proxy_str) {
                    client_builder = client_builder.proxy(proxy);
                    info!("设置代理: {}", proxy_str);
                }
            }
        }
        
        // 设置 SSL 验证
        let verify_ssl = config.parameters.get("verify_ssl")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        
        if !verify_ssl {
            client_builder = client_builder.danger_accept_invalid_certs(true);
            warn!("已禁用 SSL 证书验证");
        }
        
        self.client = Some(client_builder.build()
            .map_err(|e| AiStudioError::internal(&format!("创建 HTTP 客户端失败: {}", e)))?);
        
        self.config = Some(config);
        self.status = PluginStatus::Initialized;
        
        Ok(())
    }
    
    async fn start(&mut self) -> Result<(), AiStudioError> {
        info!("启动 HTTP 客户端插件");
        self.status = PluginStatus::Running;
        Ok(())
    }
    
    async fn stop(&mut self) -> Result<(), AiStudioError> {
        info!("停止 HTTP 客户端插件");
        self.status = PluginStatus::Stopped;
        Ok(())
    }
    
    async fn shutdown(&mut self) -> Result<(), AiStudioError> {
        info!("关闭 HTTP 客户端插件");
        self.client = None;
        self.config = None;
        self.status = PluginStatus::Unloaded;
        Ok(())
    }
    
    fn status(&self) -> PluginStatus {
        self.status.clone()
    }
    
    async fn handle_call(
        &self,
        method: &str,
        params: HashMap<String, serde_json::Value>,
        _context: &PluginContext,
    ) -> Result<serde_json::Value, AiStudioError> {
        debug!("处理插件调用: method={}, params={:?}", method, params);
        
        match method {
            "get" => self.get_request(&params).await,
            "post" => self.post_request(&params).await,
            "put" => self.put_request(&params).await,
            "delete" => self.delete_request(&params).await,
            "download" => self.download_file(&params).await,
            _ => Err(AiStudioError::validation(&format!("未知方法: {}", method)))
        }
    }
    
    async fn health_check(&self) -> Result<PluginHealth, AiStudioError> {
        let start_time = std::time::Instant::now();
        
        let healthy = self.status == PluginStatus::Running && self.client.is_some();
        let mut details = HashMap::new();
        
        // 检查网络连接（可选的健康检查 URL）
        if let Some(ref config) = self.config {
            if let Some(health_url) = config.parameters.get("health_check_url") {
                if let Some(url_str) = health_url.as_str() {
                    if let Some(ref client) = self.client {
                        match client.get(url_str).timeout(Duration::from_secs(5)).send().await {
                            Ok(response) => {
                                details.insert("health_check_status".to_string(), 
                                    serde_json::Value::Number(response.status().as_u16().into()));
                                details.insert("health_check_success".to_string(), 
                                    serde_json::Value::Bool(response.status().is_success()));
                            },
                            Err(e) => {
                                details.insert("health_check_error".to_string(), 
                                    serde_json::Value::String(e.to_string()));
                            }
                        }
                    }
                }
            }
        }
        
        details.insert("status".to_string(), serde_json::json!(self.status));
        details.insert("client_initialized".to_string(), serde_json::Value::Bool(self.client.is_some()));
        details.insert("default_timeout_seconds".to_string(), 
            serde_json::Value::Number(self.default_timeout.as_secs().into()));
        details.insert("max_response_size_bytes".to_string(), 
            serde_json::Value::Number(self.max_response_size.into()));
        details.insert("allowed_domains_count".to_string(), 
            serde_json::Value::Number(self.allowed_domains.len().into()));
        
        let response_time = start_time.elapsed().as_millis() as u64;
        
        Ok(PluginHealth {
            healthy,
            message: if healthy {
                "HTTP 客户端插件运行正常".to_string()
            } else {
                format!("HTTP 客户端插件状态异常: {:?}", self.status)
            },
            details,
            checked_at: Utc::now(),
            response_time_ms: response_time,
        })
    }
    
    fn config_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "default_timeout_seconds": {
                    "type": "integer",
                    "description": "默认请求超时时间（秒）",
                    "minimum": 1,
                    "maximum": 300,
                    "default": 30
                },
                "max_response_size_mb": {
                    "type": "integer",
                    "description": "最大响应大小限制（MB）",
                    "minimum": 1,
                    "maximum": 100,
                    "default": 10
                },
                "allowed_domains": {
                    "type": "array",
                    "description": "允许访问的域名列表（空数组表示无限制）",
                    "items": {
                        "type": "string"
                    },
                    "default": []
                },
                "proxy_url": {
                    "type": "string",
                    "description": "代理服务器 URL",
                    "default": null
                },
                "verify_ssl": {
                    "type": "boolean",
                    "description": "是否验证 SSL 证书",
                    "default": true
                },
                "health_check_url": {
                    "type": "string",
                    "description": "健康检查 URL",
                    "default": null
                },
                "user_agent": {
                    "type": "string",
                    "description": "自定义 User-Agent",
                    "default": "Aionix-AI-Studio-HTTP-Plugin/1.0"
                }
            }
        })
    }
    
    fn validate_config(&self, config: &PluginConfig) -> Result<(), AiStudioError> {
        // 验证超时时间
        if let Some(timeout) = config.parameters.get("default_timeout_seconds") {
            if let Some(timeout_val) = timeout.as_u64() {
                if timeout_val == 0 || timeout_val > 300 {
                    return Err(AiStudioError::validation("default_timeout_seconds 必须在 1-300 之间"));
                }
            }
        }
        
        // 验证响应大小限制
        if let Some(max_size) = config.parameters.get("max_response_size_mb") {
            if let Some(size_val) = max_size.as_u64() {
                if size_val == 0 || size_val > 100 {
                    return Err(AiStudioError::validation("max_response_size_mb 必须在 1-100 之间"));
                }
            }
        }
        
        // 验证代理 URL
        if let Some(proxy_url) = config.parameters.get("proxy_url") {
            if let Some(proxy_str) = proxy_url.as_str() {
                if !proxy_str.is_empty() {
                    reqwest::Url::parse(proxy_str)
                        .map_err(|_| AiStudioError::validation("无效的代理 URL"))?;
                }
            }
        }
        
        // 验证健康检查 URL
        if let Some(health_url) = config.parameters.get("health_check_url") {
            if let Some(url_str) = health_url.as_str() {
                if !url_str.is_empty() {
                    reqwest::Url::parse(url_str)
                        .map_err(|_| AiStudioError::validation("无效的健康检查 URL"))?;
                }
            }
        }
        
        Ok(())
    }
}

// 插件工厂实现
pub struct HttpClientPluginFactory;

impl HttpClientPluginFactory {
    pub fn new() -> Self {
        Self
    }
    
    pub fn create_plugin(&self) -> Result<Box<dyn Plugin>, AiStudioError> {
        Ok(Box::new(HttpClientPlugin::new()))
    }
    
    pub fn metadata(&self) -> PluginMetadata {
        HttpClientPlugin::new().metadata()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    
    async fn create_test_plugin() -> HttpClientPlugin {
        let mut plugin = HttpClientPlugin::new();
        
        let config = PluginConfig {
            plugin_id: "test".to_string(),
            parameters: HashMap::new(),
            environment: HashMap::new(),
            resource_limits: Default::default(),
            security_settings: Default::default(),
        };
        
        plugin.initialize(config).await.unwrap();
        plugin.start().await.unwrap();
        
        plugin
    }
    
    #[tokio::test]
    async fn test_plugin_lifecycle() {
        let mut plugin = HttpClientPlugin::new();
        assert_eq!(plugin.status(), PluginStatus::Uninitialized);
        
        let config = PluginConfig {
            plugin_id: "test".to_string(),
            parameters: HashMap::new(),
            environment: HashMap::new(),
            resource_limits: Default::default(),
            security_settings: Default::default(),
        };
        
        plugin.initialize(config).await.unwrap();
        assert_eq!(plugin.status(), PluginStatus::Initialized);
        
        plugin.start().await.unwrap();
        assert_eq!(plugin.status(), PluginStatus::Running);
        
        plugin.stop().await.unwrap();
        assert_eq!(plugin.status(), PluginStatus::Stopped);
        
        plugin.shutdown().await.unwrap();
        assert_eq!(plugin.status(), PluginStatus::Unloaded);
    }
    
    #[tokio::test]
    async fn test_url_validation() {
        let plugin = create_test_plugin().await;
        
        // 测试有效 URL
        assert!(plugin.validate_url("https://httpbin.org/get").is_ok());
        assert!(plugin.validate_url("http://example.com").is_ok());
        
        // 测试无效 URL
        assert!(plugin.validate_url("ftp://example.com").is_err());
        assert!(plugin.validate_url("invalid-url").is_err());
    }
    
    #[tokio::test]
    async fn test_get_request() {
        let plugin = create_test_plugin().await;
        
        let context = PluginContext {
            tenant_id: Uuid::new_v4(),
            user_id: Some(Uuid::new_v4()),
            session_id: None,
            request_id: Uuid::new_v4(),
            variables: HashMap::new(),
            timestamp: Utc::now(),
        };
        
        let mut params = HashMap::new();
        params.insert("url".to_string(), json!("https://httpbin.org/get"));
        
        let result = plugin.handle_call("get", params, &context).await;
        
        // 注意：这个测试需要网络连接，在 CI 环境中可能失败
        // 在实际项目中，应该使用 mock 服务器进行测试
        match result {
            Ok(response) => {
                assert!(response["status"].as_u64().unwrap() == 200);
                assert!(response["body"].is_string());
            },
            Err(_) => {
                // 网络错误是可以接受的
                println!("网络请求失败（可能是网络问题）");
            }
        }
    }
    
    #[tokio::test]
    async fn test_health_check() {
        let plugin = create_test_plugin().await;
        
        let health = plugin.health_check().await.unwrap();
        assert!(health.healthy);
        assert!(health.response_time_ms < 1000);
        assert!(health.details.contains_key("client_initialized"));
    }
    
    #[tokio::test]
    async fn test_invalid_method() {
        let plugin = create_test_plugin().await;
        
        let context = PluginContext {
            tenant_id: Uuid::new_v4(),
            user_id: Some(Uuid::new_v4()),
            session_id: None,
            request_id: Uuid::new_v4(),
            variables: HashMap::new(),
            timestamp: Utc::now(),
        };
        
        let result = plugin.handle_call("invalid_method", HashMap::new(), &context).await;
        assert!(result.is_err());
    }
    
    #[tokio::test]
    async fn test_config_validation() {
        let plugin = HttpClientPlugin::new();
        
        // 测试有效配置
        let valid_config = PluginConfig {
            plugin_id: "test".to_string(),
            parameters: {
                let mut params = HashMap::new();
                params.insert("default_timeout_seconds".to_string(), json!(30));
                params.insert("max_response_size_mb".to_string(), json!(10));
                params
            },
            environment: HashMap::new(),
            resource_limits: Default::default(),
            security_settings: Default::default(),
        };
        
        assert!(plugin.validate_config(&valid_config).is_ok());
        
        // 测试无效配置
        let invalid_config = PluginConfig {
            plugin_id: "test".to_string(),
            parameters: {
                let mut params = HashMap::new();
                params.insert("default_timeout_seconds".to_string(), json!(0)); // 无效值
                params
            },
            environment: HashMap::new(),
            resource_limits: Default::default(),
            security_settings: Default::default(),
        };
        
        assert!(plugin.validate_config(&invalid_config).is_err());
    }
}