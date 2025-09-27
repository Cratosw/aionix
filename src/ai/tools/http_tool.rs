// HTTP 请求工具实现

use std::collections::HashMap;
use std::time::Duration;
use async_trait::async_trait;
use serde_json;
use tracing::{debug, error, warn};
use reqwest::{Client, Method, Response};
use url::Url;

use crate::ai::agent_runtime::{Tool, ToolResult, ToolMetadata, ExecutionContext};
use crate::errors::AiStudioError;

/// HTTP 请求工具
#[derive(Debug, Clone)]
pub struct HttpTool {
    /// HTTP 客户端
    client: Client,
    /// 工具配置
    config: HttpToolConfig,
}

/// HTTP 工具配置
#[derive(Debug, Clone)]
pub struct HttpToolConfig {
    /// 请求超时时间（秒）
    pub timeout_seconds: u64,
    /// 允许的 HTTP 方法
    pub allowed_methods: Vec<String>,
    /// 允许的域名（白名单）
    pub allowed_domains: Vec<String>,
    /// 禁止的域名（黑名单）
    pub blocked_domains: Vec<String>,
    /// 最大响应大小（字节）
    pub max_response_size: u64,
    /// 是否允许重定向
    pub allow_redirects: bool,
    /// 最大重定向次数
    pub max_redirects: u32,
}

impl Default for HttpToolConfig {
    fn default() -> Self {
        Self {
            timeout_seconds: 30,
            allowed_methods: vec![
                "GET".to_string(),
                "POST".to_string(),
                "PUT".to_string(),
                "DELETE".to_string(),
                "PATCH".to_string(),
                "HEAD".to_string(),
            ],
            allowed_domains: Vec::new(), // 空表示允许所有域名
            blocked_domains: vec![
                "localhost".to_string(),
                "127.0.0.1".to_string(),
                "0.0.0.0".to_string(),
                "::1".to_string(),
            ],
            max_response_size: 10 * 1024 * 1024, // 10MB
            allow_redirects: true,
            max_redirects: 5,
        }
    }
}

impl HttpTool {
    /// 创建新的 HTTP 工具
    pub fn new() -> Result<Self, AiStudioError> {
        let config = HttpToolConfig::default();
        Self::with_config(config)
    }
    
    /// 使用自定义配置创建 HTTP 工具
    pub fn with_config(config: HttpToolConfig) -> Result<Self, AiStudioError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .redirect(if config.allow_redirects {
                reqwest::redirect::Policy::limited(config.max_redirects as usize)
            } else {
                reqwest::redirect::Policy::none()
            })
            .user_agent("AiStudio-Agent/1.0")
            .build()
            .map_err(|e| {
                error!("创建 HTTP 客户端失败: {}", e);
                AiStudioError::internal("创建 HTTP 客户端失败")
            })?;
        
        Ok(Self { client, config })
    }
}

#[async_trait]
impl Tool for HttpTool {
    async fn execute(
        &self,
        parameters: HashMap<String, serde_json::Value>,
        _context: &ExecutionContext,
    ) -> Result<ToolResult, AiStudioError> {
        debug!("执行 HTTP 工具");
        
        // 提取请求参数
        let url = parameters.get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AiStudioError::validation("缺少必需参数: url"))?;
        
        let method = parameters.get("method")
            .and_then(|v| v.as_str())
            .unwrap_or("GET");
        
        debug!("HTTP 请求: {} {}", method, url);
        
        let start_time = std::time::Instant::now();
        
        // 执行 HTTP 请求
        let response_data = self.make_request(url, method, &parameters).await?;
        
        let execution_time = start_time.elapsed().as_millis() as u64;
        
        Ok(ToolResult {
            success: true,
            data: response_data,
            error: None,
            execution_time_ms: execution_time,
            message: Some(format!("HTTP 请求完成: {} {}", method, url)),
        })
    }
    
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            name: "http".to_string(),
            description: "发送 HTTP 请求并获取响应".to_string(),
            parameters_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "请求的 URL",
                        "format": "uri"
                    },
                    "method": {
                        "type": "string",
                        "description": "HTTP 方法",
                        "enum": self.config.allowed_methods,
                        "default": "GET"
                    },
                    "headers": {
                        "type": "object",
                        "description": "请求头",
                        "additionalProperties": {
                            "type": "string"
                        }
                    },
                    "body": {
                        "type": "string",
                        "description": "请求体（POST/PUT/PATCH 需要）"
                    },
                    "json": {
                        "type": "object",
                        "description": "JSON 请求体"
                    },
                    "timeout": {
                        "type": "integer",
                        "description": "请求超时时间（秒）",
                        "minimum": 1,
                        "maximum": 300,
                        "default": 30
                    }
                },
                "required": ["url"]
            }),
            category: "network".to_string(),
            requires_permission: true,
            version: "1.0.0".to_string(),
        }
    }
    
    fn validate_parameters(
        &self,
        parameters: &HashMap<String, serde_json::Value>,
    ) -> Result<(), AiStudioError> {
        // 验证 URL 参数
        let url_str = parameters.get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AiStudioError::validation("缺少必需参数: url"))?;
        
        // 解析 URL
        let url = Url::parse(url_str).map_err(|e| {
            AiStudioError::validation(&format!("无效的 URL: {}", e))
        })?;
        
        // 检查协议
        if !matches!(url.scheme(), "http" | "https") {
            return Err(AiStudioError::validation("只支持 HTTP 和 HTTPS 协议"));
        }
        
        // 检查域名白名单
        if !self.config.allowed_domains.is_empty() {
            if let Some(host) = url.host_str() {
                if !self.config.allowed_domains.iter().any(|domain| host.contains(domain)) {
                    return Err(AiStudioError::validation(&format!("域名不在允许列表中: {}", host)));
                }
            }
        }
        
        // 检查域名黑名单
        if let Some(host) = url.host_str() {
            if self.config.blocked_domains.iter().any(|domain| host.contains(domain)) {
                return Err(AiStudioError::validation(&format!("域名在禁止列表中: {}", host)));
            }
        }
        
        // 验证 HTTP 方法
        if let Some(method) = parameters.get("method") {
            if let Some(method_str) = method.as_str() {
                if !self.config.allowed_methods.contains(&method_str.to_uppercase()) {
                    return Err(AiStudioError::validation(&format!("不允许的 HTTP 方法: {}", method_str)));
                }
            } else {
                return Err(AiStudioError::validation("method 必须是字符串"));
            }
        }
        
        // 验证请求头
        if let Some(headers) = parameters.get("headers") {
            if !headers.is_object() {
                return Err(AiStudioError::validation("headers 必须是对象"));
            }
            
            // 检查危险的请求头
            if let Some(headers_obj) = headers.as_object() {
                for (key, value) in headers_obj {
                    let key_lower = key.to_lowercase();
                    if matches!(key_lower.as_str(), "authorization" | "cookie" | "x-forwarded-for") {
                        warn!("检测到敏感请求头: {}", key);
                    }
                    
                    if !value.is_string() {
                        return Err(AiStudioError::validation(&format!("请求头 {} 的值必须是字符串", key)));
                    }
                }
            }
        }
        
        // 验证超时参数
        if let Some(timeout) = parameters.get("timeout") {
            if let Some(timeout_num) = timeout.as_u64() {
                if timeout_num == 0 || timeout_num > 300 {
                    return Err(AiStudioError::validation("timeout 必须在 1-300 秒之间"));
                }
            } else {
                return Err(AiStudioError::validation("timeout 必须是正整数"));
            }
        }
        
        Ok(())
    }
}

impl HttpTool {
    /// 发送 HTTP 请求
    async fn make_request(
        &self,
        url: &str,
        method: &str,
        parameters: &HashMap<String, serde_json::Value>,
    ) -> Result<serde_json::Value, AiStudioError> {
        // 解析 HTTP 方法
        let http_method = Method::from_bytes(method.as_bytes()).map_err(|e| {
            AiStudioError::validation("method".to_string(), &format!("无效的 HTTP 方法: {}", e))
        })?;
        
        // 构建请求
        let mut request_builder = self.client.request(http_method, url);
        
        // 添加请求头
        if let Some(headers) = parameters.get("headers") {
            if let Some(headers_obj) = headers.as_object() {
                for (key, value) in headers_obj {
                    if let Some(value_str) = value.as_str() {
                        request_builder = request_builder.header(key, value_str);
                    }
                }
            }
        }
        
        // 添加请求体
        if let Some(json_body) = parameters.get("json") {
            request_builder = request_builder.json(json_body);
        } else if let Some(body) = parameters.get("body") {
            if let Some(body_str) = body.as_str() {
                request_builder = request_builder.body(body_str.to_string());
            }
        }
        
        // 设置超时
        if let Some(timeout) = parameters.get("timeout") {
            if let Some(timeout_secs) = timeout.as_u64() {
                request_builder = request_builder.timeout(Duration::from_secs(timeout_secs));
            }
        }
        
        // 发送请求
        debug!("发送 HTTP 请求: {} {}", method, url);
        let response = request_builder.send().await.map_err(|e| {
            error!("HTTP 请求失败: {}", e);
            AiStudioError::external_service("http".to_string(), format!("HTTP 请求失败: {}", e))
        })?;
        
        // 处理响应
        self.process_response(response).await
    }
    
    /// 处理 HTTP 响应
    async fn process_response(&self, response: Response) -> Result<serde_json::Value, AiStudioError> {
        let status = response.status();
        let headers: HashMap<String, String> = response.headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();
        
        debug!("HTTP 响应状态: {}", status);
        
        // 检查内容长度
        if let Some(content_length) = response.content_length() {
            if content_length > self.config.max_response_size {
                return Err(AiStudioError::validation("response_size".to_string(), &format!(
                    "响应太大: {} 字节，最大允许: {} 字节",
                    content_length,
                    self.config.max_response_size
                )));
            }
        }
        
        // 获取响应体
        let response_bytes = response.bytes().await.map_err(|e| {
            error!("读取响应体失败: {}", e);
            AiStudioError::external_service("http".to_string(), format!("读取响应体失败: {}", e))
        })?;
        
        // 检查响应大小
        if response_bytes.len() > self.config.max_response_size as usize {
            return Err(AiStudioError::validation(&format!(
                "响应太大: {} 字节，最大允许: {} 字节",
                response_bytes.len(),
                self.config.max_response_size
            )));
        }
        
        // 尝试解析为文本
        let response_text = String::from_utf8_lossy(&response_bytes).to_string();
        
        // 尝试解析为 JSON
        let response_json = if let Ok(json) = serde_json::from_str::<serde_json::Value>(&response_text) {
            Some(json)
        } else {
            None
        };
        
        Ok(serde_json::json!({
            "status": status.as_u16(),
            "status_text": status.canonical_reason().unwrap_or(""),
            "headers": headers,
            "body": response_text,
            "json": response_json,
            "size": response_bytes.len(),
            "success": status.is_success()
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_http_tool_validation() {
        let tool = HttpTool::new().unwrap();
        
        // 测试有效参数
        let mut valid_params = HashMap::new();
        valid_params.insert("url".to_string(), serde_json::Value::String("https://httpbin.org/get".to_string()));
        valid_params.insert("method".to_string(), serde_json::Value::String("GET".to_string()));
        assert!(tool.validate_parameters(&valid_params).is_ok());
        
        // 测试无效 URL
        let mut invalid_params = HashMap::new();
        invalid_params.insert("url".to_string(), serde_json::Value::String("not-a-url".to_string()));
        assert!(tool.validate_parameters(&invalid_params).is_err());
        
        // 测试禁止的域名
        let mut blocked_params = HashMap::new();
        blocked_params.insert("url".to_string(), serde_json::Value::String("http://localhost:8080/test".to_string()));
        assert!(tool.validate_parameters(&blocked_params).is_err());
        
        // 测试不支持的协议
        let mut unsupported_params = HashMap::new();
        unsupported_params.insert("url".to_string(), serde_json::Value::String("ftp://example.com/file".to_string()));
        assert!(tool.validate_parameters(&unsupported_params).is_err());
    }
    
    #[tokio::test]
    async fn test_http_get_request() {
        let tool = HttpTool::new().unwrap();
        let mut parameters = HashMap::new();
        parameters.insert("url".to_string(), serde_json::Value::String("https://httpbin.org/get".to_string()));
        parameters.insert("method".to_string(), serde_json::Value::String("GET".to_string()));
        
        let context = ExecutionContext {
            current_task: None,
            execution_history: Vec::new(),
            context_variables: HashMap::new(),
            session_id: None,
            user_id: None,
        };
        
        // 注意：这个测试需要网络连接
        if let Ok(result) = tool.execute(parameters, &context).await {
            assert!(result.success);
            assert!(result.data.get("status").is_some());
        }
    }
}