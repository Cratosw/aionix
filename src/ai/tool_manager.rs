// 工具管理器
// 实现工具注册、动态加载和安全调用系统

use std::sync::Arc;
use std::collections::HashMap;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{info, warn, error, debug};
use tokio::sync::RwLock;
use async_trait::async_trait;

use crate::ai::agent_runtime::{Tool, ToolResult, ToolMetadata, ExecutionContext};
use crate::errors::AiStudioError;

/// 工具管理器
pub struct ToolManager {
    /// 注册的工具
    tools: Arc<RwLock<HashMap<String, Arc<dyn Tool + Send + Sync>>>>,
    /// 工具元数据
    metadata: Arc<RwLock<HashMap<String, ToolMetadata>>>,
    /// 工具使用统计
    usage_stats: Arc<RwLock<HashMap<String, ToolUsageStats>>>,
    /// 工具权限配置
    permissions: Arc<RwLock<HashMap<String, ToolPermissions>>>,
    /// 工具配置
    config: ToolManagerConfig,
}

/// 工具管理器配置
#[derive(Debug, Clone)]
pub struct ToolManagerConfig {
    /// 是否启用工具权限检查
    pub enable_permission_check: bool,
    /// 工具调用超时时间（秒）
    pub default_timeout_seconds: u64,
    /// 最大并发工具调用数
    pub max_concurrent_calls: usize,
    /// 是否启用工具使用统计
    pub enable_usage_stats: bool,
    /// 工具调用日志级别
    pub log_level: ToolLogLevel,
}

impl Default for ToolManagerConfig {
    fn default() -> Self {
        Self {
            enable_permission_check: true,
            default_timeout_seconds: 30,
            max_concurrent_calls: 50,
            enable_usage_stats: true,
            log_level: ToolLogLevel::Info,
        }
    }
}

/// 工具日志级别
#[derive(Debug, Clone, PartialEq)]
pub enum ToolLogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

/// 工具使用统计
#[derive(Debug, Clone, Serialize)]
pub struct ToolUsageStats {
    /// 工具名称
    pub tool_name: String,
    /// 总调用次数
    pub total_calls: u64,
    /// 成功调用次数
    pub successful_calls: u64,
    /// 失败调用次数
    pub failed_calls: u64,
    /// 平均执行时间（毫秒）
    pub avg_execution_time_ms: f32,
    /// 最后调用时间
    pub last_called_at: Option<DateTime<Utc>>,
    /// 最快执行时间（毫秒）
    pub min_execution_time_ms: u64,
    /// 最慢执行时间（毫秒）
    pub max_execution_time_ms: u64,
}

impl Default for ToolUsageStats {
    fn default() -> Self {
        Self {
            tool_name: String::new(),
            total_calls: 0,
            successful_calls: 0,
            failed_calls: 0,
            avg_execution_time_ms: 0.0,
            last_called_at: None,
            min_execution_time_ms: u64::MAX,
            max_execution_time_ms: 0,
        }
    }
}

/// 工具权限配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolPermissions {
    /// 工具名称
    pub tool_name: String,
    /// 是否启用
    pub enabled: bool,
    /// 允许的租户 ID 列表（空表示允许所有）
    pub allowed_tenants: Vec<Uuid>,
    /// 禁止的租户 ID 列表
    pub blocked_tenants: Vec<Uuid>,
    /// 允许的用户 ID 列表（空表示允许所有）
    pub allowed_users: Vec<Uuid>,
    /// 禁止的用户 ID 列表
    pub blocked_users: Vec<Uuid>,
    /// 每小时调用限制
    pub hourly_limit: Option<u32>,
    /// 每天调用限制
    pub daily_limit: Option<u32>,
    /// 需要的权限级别
    pub required_permission_level: PermissionLevel,
}

impl Default for ToolPermissions {
    fn default() -> Self {
        Self {
            tool_name: String::new(),
            enabled: true,
            allowed_tenants: Vec::new(),
            blocked_tenants: Vec::new(),
            allowed_users: Vec::new(),
            blocked_users: Vec::new(),
            hourly_limit: None,
            daily_limit: None,
            required_permission_level: PermissionLevel::Basic,
        }
    }
}

/// 权限级别
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
#[serde(rename_all = "lowercase")]
pub enum PermissionLevel {
    Basic,
    Intermediate,
    Advanced,
    Admin,
}

/// 工具调用请求
#[derive(Debug, Clone)]
pub struct ToolCallRequest {
    /// 工具名称
    pub tool_name: String,
    /// 调用参数
    pub parameters: HashMap<String, serde_json::Value>,
    /// 执行上下文
    pub context: ExecutionContext,
    /// 调用 ID
    pub call_id: Uuid,
    /// 超时时间（秒）
    pub timeout_seconds: Option<u64>,
}

/// 工具调用响应
#[derive(Debug, Clone, Serialize)]
pub struct ToolCallResponse {
    /// 调用 ID
    pub call_id: Uuid,
    /// 工具名称
    pub tool_name: String,
    /// 执行结果
    pub result: ToolResult,
    /// 调用开始时间
    pub started_at: DateTime<Utc>,
    /// 调用完成时间
    pub completed_at: DateTime<Utc>,
    /// 执行时间（毫秒）
    pub execution_time_ms: u64,
}

/// 工具注册信息
#[derive(Debug, Clone, Serialize)]
pub struct ToolRegistration {
    /// 工具名称
    pub name: String,
    /// 工具元数据
    pub metadata: ToolMetadata,
    /// 注册时间
    pub registered_at: DateTime<Utc>,
    /// 权限配置
    pub permissions: ToolPermissions,
    /// 使用统计
    pub usage_stats: ToolUsageStats,
}

/// 工具列表响应
#[derive(Debug, Clone, Serialize)]
pub struct ToolListResponse {
    /// 工具列表
    pub tools: Vec<ToolRegistration>,
    /// 总数
    pub total: usize,
    /// 启用的工具数
    pub enabled: usize,
    /// 禁用的工具数
    pub disabled: usize,
}

impl ToolManager {
    /// 创建新的工具管理器
    pub fn new(config: Option<ToolManagerConfig>) -> Self {
        Self {
            tools: Arc::new(RwLock::new(HashMap::new())),
            metadata: Arc::new(RwLock::new(HashMap::new())),
            usage_stats: Arc::new(RwLock::new(HashMap::new())),
            permissions: Arc::new(RwLock::new(HashMap::new())),
            config: config.unwrap_or_default(),
        }
    }
    
    /// 注册工具
    pub async fn register_tool(
        &self,
        tool: Arc<dyn Tool + Send + Sync>,
        permissions: Option<ToolPermissions>,
    ) -> Result<(), AiStudioError> {
        let metadata = tool.metadata();
        let tool_name = metadata.name.clone();
        
        info!("注册工具: {}", tool_name);
        
        // 验证工具元数据
        self.validate_tool_metadata(&metadata)?;
        
        // 注册工具
        {
            let mut tools = self.tools.write().await;
            tools.insert(tool_name.clone(), tool);
        }
        
        // 保存元数据
        {
            let mut metadata_map = self.metadata.write().await;
            metadata_map.insert(tool_name.clone(), metadata);
        }
        
        // 初始化使用统计
        if self.config.enable_usage_stats {
            let mut stats = self.usage_stats.write().await;
            stats.insert(tool_name.clone(), ToolUsageStats {
                tool_name: tool_name.clone(),
                ..Default::default()
            });
        }
        
        // 设置权限
        {
            let mut permissions_map = self.permissions.write().await;
            let tool_permissions = permissions.unwrap_or_else(|| ToolPermissions {
                tool_name: tool_name.clone(),
                ..Default::default()
            });
            permissions_map.insert(tool_name.clone(), tool_permissions);
        }
        
        info!("工具注册成功: {}", tool_name);
        Ok(())
    }
    
    /// 注销工具
    pub async fn unregister_tool(&self, tool_name: &str) -> Result<(), AiStudioError> {
        info!("注销工具: {}", tool_name);
        
        // 检查工具是否存在
        {
            let tools = self.tools.read().await;
            if !tools.contains_key(tool_name) {
                return Err(AiStudioError::not_found(&format!("工具不存在: {}", tool_name)));
            }
        }
        
        // 移除工具
        {
            let mut tools = self.tools.write().await;
            tools.remove(tool_name);
        }
        
        // 移除元数据
        {
            let mut metadata = self.metadata.write().await;
            metadata.remove(tool_name);
        }
        
        // 保留使用统计（用于历史分析）
        
        // 移除权限配置
        {
            let mut permissions = self.permissions.write().await;
            permissions.remove(tool_name);
        }
        
        info!("工具注销成功: {}", tool_name);
        Ok(())
    }
    
    /// 调用工具
    pub async fn call_tool(&self, request: ToolCallRequest) -> Result<ToolCallResponse, AiStudioError> {
        let start_time = Utc::now();
        
        debug!("调用工具: {} (call_id={})", request.tool_name, request.call_id);
        
        // 权限检查
        if self.config.enable_permission_check {
            self.check_tool_permissions(&request).await?;
        }
        
        // 获取工具
        let tool = {
            let tools = self.tools.read().await;
            tools.get(&request.tool_name)
                .ok_or_else(|| AiStudioError::not_found(&format!("工具不存在: {}", request.tool_name)))?
                .clone()
        };
        
        // 验证参数
        tool.validate_parameters(&request.parameters)?;
        
        // 执行工具
        let execution_start = std::time::Instant::now();
        let result = match tokio::time::timeout(
            std::time::Duration::from_secs(
                request.timeout_seconds.unwrap_or(self.config.default_timeout_seconds)
            ),
            tool.execute(request.parameters.clone(), &request.context)
        ).await {
            Ok(Ok(result)) => result,
            Ok(Err(e)) => {
                error!("工具执行失败: {} - {}", request.tool_name, e);
                ToolResult {
                    success: false,
                    data: serde_json::Value::Null,
                    error: Some(e.to_string()),
                    execution_time_ms: execution_start.elapsed().as_millis() as u64,
                    message: Some("工具执行失败".to_string()),
                }
            }
            Err(_) => {
                error!("工具执行超时: {}", request.tool_name);
                ToolResult {
                    success: false,
                    data: serde_json::Value::Null,
                    error: Some("执行超时".to_string()),
                    execution_time_ms: execution_start.elapsed().as_millis() as u64,
                    message: Some("工具执行超时".to_string()),
                }
            }
        };
        
        let end_time = Utc::now();
        let execution_time_ms = execution_start.elapsed().as_millis() as u64;
        
        // 更新使用统计
        if self.config.enable_usage_stats {
            self.update_usage_stats(&request.tool_name, &result, execution_time_ms).await;
        }
        
        // 记录日志
        match self.config.log_level {
            ToolLogLevel::Debug => debug!("工具调用完成: {} - 成功: {} - 时间: {}ms", 
                                         request.tool_name, result.success, execution_time_ms),
            ToolLogLevel::Info => info!("工具调用: {} - 成功: {} - 时间: {}ms", 
                                       request.tool_name, result.success, execution_time_ms),
            ToolLogLevel::Warn if !result.success => warn!("工具调用失败: {} - 错误: {:?}", 
                                                           request.tool_name, result.error),
            ToolLogLevel::Error if !result.success => error!("工具调用失败: {} - 错误: {:?}", 
                                                             request.tool_name, result.error),
            _ => {}
        }
        
        Ok(ToolCallResponse {
            call_id: request.call_id,
            tool_name: request.tool_name,
            result,
            started_at: start_time,
            completed_at: end_time,
            execution_time_ms,
        })
    }
    
    /// 获取工具列表
    pub async fn list_tools(&self) -> Result<ToolListResponse, AiStudioError> {
        let tools = self.tools.read().await;
        let metadata = self.metadata.read().await;
        let permissions = self.permissions.read().await;
        let usage_stats = self.usage_stats.read().await;
        
        let mut tool_registrations = Vec::new();
        let mut enabled_count = 0;
        let mut disabled_count = 0;
        
        for (tool_name, _) in tools.iter() {
            let tool_metadata = metadata.get(tool_name).cloned()
                .unwrap_or_else(|| ToolMetadata {
                    name: tool_name.clone(),
                    description: "无描述".to_string(),
                    parameters_schema: serde_json::Value::Null,
                    category: "unknown".to_string(),
                    requires_permission: false,
                    version: "1.0.0".to_string(),
                });
            
            let tool_permissions = permissions.get(tool_name).cloned()
                .unwrap_or_else(|| ToolPermissions {
                    tool_name: tool_name.clone(),
                    ..Default::default()
                });
            
            let tool_usage_stats = usage_stats.get(tool_name).cloned()
                .unwrap_or_else(|| ToolUsageStats {
                    tool_name: tool_name.clone(),
                    ..Default::default()
                });
            
            if tool_permissions.enabled {
                enabled_count += 1;
            } else {
                disabled_count += 1;
            }
            
            tool_registrations.push(ToolRegistration {
                name: tool_name.clone(),
                metadata: tool_metadata,
                registered_at: Utc::now(), // TODO: 保存实际注册时间
                permissions: tool_permissions,
                usage_stats: tool_usage_stats,
            });
        }
        
        Ok(ToolListResponse {
            tools: tool_registrations,
            total: tools.len(),
            enabled: enabled_count,
            disabled: disabled_count,
        })
    }
    
    /// 获取工具元数据
    pub async fn get_tool_metadata(&self, tool_name: &str) -> Result<ToolMetadata, AiStudioError> {
        let metadata = self.metadata.read().await;
        metadata.get(tool_name)
            .cloned()
            .ok_or_else(|| AiStudioError::not_found(&format!("工具不存在: {}", tool_name)))
    }
    
    /// 更新工具权限
    pub async fn update_tool_permissions(
        &self,
        tool_name: &str,
        permissions: ToolPermissions,
    ) -> Result<(), AiStudioError> {
        let mut permissions_map = self.permissions.write().await;
        
        if !permissions_map.contains_key(tool_name) {
            return Err(AiStudioError::not_found(&format!("工具不存在: {}", tool_name)));
        }
        
        permissions_map.insert(tool_name.to_string(), permissions);
        info!("更新工具权限: {}", tool_name);
        
        Ok(())
    }
    
    /// 获取工具使用统计
    pub async fn get_tool_usage_stats(&self, tool_name: &str) -> Result<ToolUsageStats, AiStudioError> {
        let usage_stats = self.usage_stats.read().await;
        usage_stats.get(tool_name)
            .cloned()
            .ok_or_else(|| AiStudioError::not_found(&format!("工具不存在: {}", tool_name)))
    }
    
    /// 获取所有工具使用统计
    pub async fn get_all_usage_stats(&self) -> Result<Vec<ToolUsageStats>, AiStudioError> {
        let usage_stats = self.usage_stats.read().await;
        Ok(usage_stats.values().cloned().collect())
    }
    
    /// 验证工具元数据
    fn validate_tool_metadata(&self, metadata: &ToolMetadata) -> Result<(), AiStudioError> {
        if metadata.name.is_empty() {
            return Err(AiStudioError::validation("工具名称不能为空"));
        }
        
        if metadata.name.len() > 100 {
            return Err(AiStudioError::validation("工具名称长度不能超过 100 字符"));
        }
        
        if !metadata.name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
            return Err(AiStudioError::validation("工具名称只能包含字母、数字、下划线和连字符"));
        }
        
        if metadata.description.len() > 1000 {
            return Err(AiStudioError::validation("工具描述长度不能超过 1000 字符"));
        }
        
        Ok(())
    }
    
    /// 检查工具权限
    async fn check_tool_permissions(&self, request: &ToolCallRequest) -> Result<(), AiStudioError> {
        let permissions = self.permissions.read().await;
        let tool_permissions = permissions.get(&request.tool_name)
            .ok_or_else(|| AiStudioError::not_found(&format!("工具权限配置不存在: {}", request.tool_name)))?;
        
        // 检查工具是否启用
        if !tool_permissions.enabled {
            return Err(AiStudioError::permission_denied(&format!("工具已禁用: {}", request.tool_name)));
        }
        
        // 检查租户权限
        if let Some(tenant_id) = request.context.context_variables.get("tenant_id")
            .and_then(|v| v.as_str())
            .and_then(|s| Uuid::parse_str(s).ok()) {
            
            if !tool_permissions.allowed_tenants.is_empty() && 
               !tool_permissions.allowed_tenants.contains(&tenant_id) {
                return Err(AiStudioError::permission_denied("租户无权限使用此工具"));
            }
            
            if tool_permissions.blocked_tenants.contains(&tenant_id) {
                return Err(AiStudioError::permission_denied("租户被禁止使用此工具"));
            }
        }
        
        // 检查用户权限
        if let Some(user_id) = request.context.user_id {
            if !tool_permissions.allowed_users.is_empty() && 
               !tool_permissions.allowed_users.contains(&user_id) {
                return Err(AiStudioError::permission_denied("用户无权限使用此工具"));
            }
            
            if tool_permissions.blocked_users.contains(&user_id) {
                return Err(AiStudioError::permission_denied("用户被禁止使用此工具"));
            }
        }
        
        // TODO: 检查调用频率限制
        
        Ok(())
    }
    
    /// 更新使用统计
    async fn update_usage_stats(
        &self,
        tool_name: &str,
        result: &ToolResult,
        execution_time_ms: u64,
    ) {
        let mut usage_stats = self.usage_stats.write().await;
        let stats = usage_stats.entry(tool_name.to_string())
            .or_insert_with(|| ToolUsageStats {
                tool_name: tool_name.to_string(),
                ..Default::default()
            });
        
        stats.total_calls += 1;
        if result.success {
            stats.successful_calls += 1;
        } else {
            stats.failed_calls += 1;
        }
        
        // 更新执行时间统计
        let total_time = stats.avg_execution_time_ms * (stats.total_calls - 1) as f32 + execution_time_ms as f32;
        stats.avg_execution_time_ms = total_time / stats.total_calls as f32;
        
        if execution_time_ms < stats.min_execution_time_ms {
            stats.min_execution_time_ms = execution_time_ms;
        }
        
        if execution_time_ms > stats.max_execution_time_ms {
            stats.max_execution_time_ms = execution_time_ms;
        }
        
        stats.last_called_at = Some(Utc::now());
    }
}

/// 工具管理器工厂
pub struct ToolManagerFactory;

impl ToolManagerFactory {
    /// 创建工具管理器实例
    pub fn create(config: Option<ToolManagerConfig>) -> Arc<ToolManager> {
        Arc::new(ToolManager::new(config))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::tools::CalculatorTool;
    
    #[tokio::test]
    async fn test_tool_registration() {
        let manager = ToolManager::new(None);
        let tool = Arc::new(CalculatorTool::new());
        
        let result = manager.register_tool(tool, None).await;
        assert!(result.is_ok());
        
        let tools = manager.list_tools().await.unwrap();
        assert_eq!(tools.total, 1);
        assert_eq!(tools.enabled, 1);
    }
    
    #[tokio::test]
    async fn test_tool_call() {
        let manager = ToolManager::new(None);
        let tool = Arc::new(CalculatorTool::new());
        
        manager.register_tool(tool, None).await.unwrap();
        
        let mut parameters = HashMap::new();
        parameters.insert("operation".to_string(), serde_json::Value::String("add".to_string()));
        parameters.insert("a".to_string(), serde_json::Value::Number(serde_json::Number::from(5)));
        parameters.insert("b".to_string(), serde_json::Value::Number(serde_json::Number::from(3)));
        
        let request = ToolCallRequest {
            tool_name: "calculator".to_string(),
            parameters,
            context: ExecutionContext {
                current_task: None,
                execution_history: Vec::new(),
                context_variables: HashMap::new(),
                session_id: None,
                user_id: None,
            },
            call_id: Uuid::new_v4(),
            timeout_seconds: None,
        };
        
        let response = manager.call_tool(request).await.unwrap();
        assert!(response.result.success);
    }
}