// 插件接口规范
// 定义插件的标准接口和生命周期

use std::sync::Arc;
use std::collections::HashMap;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use async_trait::async_trait;

use crate::errors::AiStudioError;

/// 插件接口
/// 所有插件必须实现此接口
#[async_trait]
pub trait Plugin: Send + Sync {
    /// 获取插件元数据
    fn metadata(&self) -> PluginMetadata;
    
    /// 初始化插件
    async fn initialize(&mut self, config: PluginConfig) -> Result<(), AiStudioError>;
    
    /// 启动插件
    async fn start(&mut self) -> Result<(), AiStudioError>;
    
    /// 停止插件
    async fn stop(&mut self) -> Result<(), AiStudioError>;
    
    /// 卸载插件
    async fn shutdown(&mut self) -> Result<(), AiStudioError>;
    
    /// 获取插件状态
    fn status(&self) -> PluginStatus;
    
    /// 处理插件调用
    async fn handle_call(
        &self,
        method: &str,
        params: HashMap<String, serde_json::Value>,
        context: &PluginContext,
    ) -> Result<serde_json::Value, AiStudioError>;
    
    /// 获取插件健康状态
    async fn health_check(&self) -> Result<PluginHealth, AiStudioError>;
    
    /// 获取插件配置模式
    fn config_schema(&self) -> serde_json::Value;
    
    /// 验证配置
    fn validate_config(&self, config: &PluginConfig) -> Result<(), AiStudioError>;
}

/// 插件元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    /// 插件 ID
    pub id: String,
    /// 插件名称
    pub name: String,
    /// 插件版本
    pub version: String,
    /// 插件描述
    pub description: String,
    /// 插件作者
    pub author: String,
    /// 插件许可证
    pub license: String,
    /// 插件主页
    pub homepage: Option<String>,
    /// 插件仓库
    pub repository: Option<String>,
    /// 插件类型
    pub plugin_type: PluginType,
    /// 支持的 API 版本
    pub api_version: String,
    /// 最小系统版本要求
    pub min_system_version: String,
    /// 插件依赖
    pub dependencies: Vec<PluginDependency>,
    /// 插件权限要求
    pub permissions: Vec<PluginPermission>,
    /// 插件标签
    pub tags: Vec<String>,
    /// 插件图标
    pub icon: Option<String>,
    /// 创建时间
    pub created_at: DateTime<Utc>,
}

/// 插件类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PluginType {
    /// 工具插件
    Tool,
    /// Agent 插件
    Agent,
    /// 工作流插件
    Workflow,
    /// 数据源插件
    DataSource,
    /// 认证插件
    Authentication,
    /// 存储插件
    Storage,
    /// 通知插件
    Notification,
    /// 监控插件
    Monitoring,
    /// 自定义插件
    Custom,
}

/// 插件依赖
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginDependency {
    /// 依赖插件 ID
    pub plugin_id: String,
    /// 版本要求
    pub version_requirement: String,
    /// 是否可选
    pub optional: bool,
}

/// 插件权限
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PluginPermission {
    /// 文件系统访问
    FileSystem,
    /// 网络访问
    Network,
    /// 数据库访问
    Database,
    /// 系统信息访问
    SystemInfo,
    /// 用户数据访问
    UserData,
    /// 管理员权限
    Admin,
    /// 自定义权限
    Custom(String),
}

/// 插件配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    /// 插件 ID
    pub plugin_id: String,
    /// 配置参数
    pub parameters: HashMap<String, serde_json::Value>,
    /// 环境变量
    pub environment: HashMap<String, String>,
    /// 资源限制
    pub resource_limits: ResourceLimits,
    /// 安全设置
    pub security_settings: SecuritySettings,
}

/// 资源限制
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// 最大内存使用（MB）
    pub max_memory_mb: Option<u64>,
    /// 最大 CPU 使用率（百分比）
    pub max_cpu_percent: Option<f32>,
    /// 最大磁盘使用（MB）
    pub max_disk_mb: Option<u64>,
    /// 最大网络带宽（KB/s）
    pub max_network_kbps: Option<u64>,
    /// 最大执行时间（秒）
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

/// 安全设置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecuritySettings {
    /// 是否启用沙箱
    pub enable_sandbox: bool,
    /// 允许的网络域名
    pub allowed_domains: Vec<String>,
    /// 允许的文件路径
    pub allowed_paths: Vec<String>,
    /// 禁止的操作
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

/// 插件状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PluginStatus {
    /// 未初始化
    Uninitialized,
    /// 初始化中
    Initializing,
    /// 已初始化
    Initialized,
    /// 启动中
    Starting,
    /// 运行中
    Running,
    /// 停止中
    Stopping,
    /// 已停止
    Stopped,
    /// 错误状态
    Error,
    /// 卸载中
    Unloading,
    /// 已卸载
    Unloaded,
}

/// 插件上下文
#[derive(Debug, Clone)]
pub struct PluginContext {
    /// 租户 ID
    pub tenant_id: Uuid,
    /// 用户 ID
    pub user_id: Option<Uuid>,
    /// 会话 ID
    pub session_id: Option<Uuid>,
    /// 请求 ID
    pub request_id: Uuid,
    /// 上下文变量
    pub variables: HashMap<String, serde_json::Value>,
    /// 调用时间
    pub timestamp: DateTime<Utc>,
}

/// 插件健康状态
#[derive(Debug, Clone, Serialize)]
pub struct PluginHealth {
    /// 是否健康
    pub healthy: bool,
    /// 状态消息
    pub message: String,
    /// 详细信息
    pub details: HashMap<String, serde_json::Value>,
    /// 检查时间
    pub checked_at: DateTime<Utc>,
    /// 响应时间（毫秒）
    pub response_time_ms: u64,
}

/// 插件事件
#[derive(Debug, Clone, Serialize)]
pub struct PluginEvent {
    /// 事件 ID
    pub event_id: Uuid,
    /// 插件 ID
    pub plugin_id: String,
    /// 事件类型
    pub event_type: PluginEventType,
    /// 事件数据
    pub data: serde_json::Value,
    /// 事件时间
    pub timestamp: DateTime<Utc>,
}

/// 插件事件类型
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PluginEventType {
    /// 插件加载
    Loaded,
    /// 插件初始化
    Initialized,
    /// 插件启动
    Started,
    /// 插件停止
    Stopped,
    /// 插件卸载
    Unloaded,
    /// 插件错误
    Error,
    /// 插件调用
    Called,
    /// 配置更新
    ConfigUpdated,
    /// 健康检查
    HealthCheck,
}

/// 插件钩子接口
/// 用于插件系统的扩展点
#[async_trait]
pub trait PluginHook: Send + Sync {
    /// 钩子名称
    fn name(&self) -> &str;
    
    /// 执行钩子
    async fn execute(
        &self,
        event: &PluginEvent,
        context: &PluginContext,
    ) -> Result<(), AiStudioError>;
}

/// 插件工厂接口
/// 用于创建插件实例
pub trait PluginFactory: Send + Sync {
    /// 创建插件实例
    fn create_plugin(&self) -> Result<Box<dyn Plugin>, AiStudioError>;
    
    /// 获取插件元数据
    fn metadata(&self) -> PluginMetadata;
    
    /// 验证插件兼容性
    fn validate_compatibility(&self, system_version: &str) -> Result<(), AiStudioError>;
}

/// 插件 API 接口
/// 提供给插件使用的系统 API
#[async_trait]
pub trait PluginApi: Send + Sync {
    /// 记录日志
    async fn log(&self, level: LogLevel, message: &str, data: Option<serde_json::Value>);
    
    /// 获取配置
    async fn get_config(&self, key: &str) -> Result<Option<serde_json::Value>, AiStudioError>;
    
    /// 设置配置
    async fn set_config(&self, key: &str, value: serde_json::Value) -> Result<(), AiStudioError>;
    
    /// 调用其他插件
    async fn call_plugin(
        &self,
        plugin_id: &str,
        method: &str,
        params: HashMap<String, serde_json::Value>,
    ) -> Result<serde_json::Value, AiStudioError>;
    
    /// 发送事件
    async fn emit_event(&self, event: PluginEvent) -> Result<(), AiStudioError>;
    
    /// 订阅事件
    async fn subscribe_event(
        &self,
        event_type: PluginEventType,
        callback: Box<dyn Fn(PluginEvent) + Send + Sync>,
    ) -> Result<(), AiStudioError>;
    
    /// 获取系统信息
    async fn get_system_info(&self) -> Result<SystemInfo, AiStudioError>;
    
    /// 执行 HTTP 请求
    async fn http_request(
        &self,
        method: &str,
        url: &str,
        headers: Option<HashMap<String, String>>,
        body: Option<serde_json::Value>,
    ) -> Result<HttpResponse, AiStudioError>;
    
    /// 访问数据库
    async fn database_query(
        &self,
        query: &str,
        params: Vec<serde_json::Value>,
    ) -> Result<Vec<HashMap<String, serde_json::Value>>, AiStudioError>;
}

/// 日志级别
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

/// 系统信息
#[derive(Debug, Clone, Serialize)]
pub struct SystemInfo {
    /// 系统版本
    pub version: String,
    /// 系统名称
    pub name: String,
    /// 运行时间
    pub uptime_seconds: u64,
    /// 内存使用情况
    pub memory_usage: MemoryUsage,
    /// CPU 使用情况
    pub cpu_usage: CpuUsage,
    /// 磁盘使用情况
    pub disk_usage: DiskUsage,
}

/// 内存使用情况
#[derive(Debug, Clone, Serialize)]
pub struct MemoryUsage {
    /// 总内存（MB）
    pub total_mb: u64,
    /// 已使用内存（MB）
    pub used_mb: u64,
    /// 可用内存（MB）
    pub available_mb: u64,
    /// 使用率（百分比）
    pub usage_percent: f32,
}

/// CPU 使用情况
#[derive(Debug, Clone, Serialize)]
pub struct CpuUsage {
    /// CPU 核心数
    pub cores: u32,
    /// 平均使用率（百分比）
    pub usage_percent: f32,
    /// 负载平均值
    pub load_average: Vec<f32>,
}

/// 磁盘使用情况
#[derive(Debug, Clone, Serialize)]
pub struct DiskUsage {
    /// 总空间（MB）
    pub total_mb: u64,
    /// 已使用空间（MB）
    pub used_mb: u64,
    /// 可用空间（MB）
    pub available_mb: u64,
    /// 使用率（百分比）
    pub usage_percent: f32,
}

/// HTTP 响应
#[derive(Debug, Clone, Serialize)]
pub struct HttpResponse {
    /// 状态码
    pub status_code: u16,
    /// 响应头
    pub headers: HashMap<String, String>,
    /// 响应体
    pub body: String,
    /// 响应时间（毫秒）
    pub response_time_ms: u64,
}

/// 插件错误类型
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PluginErrorType {
    /// 初始化错误
    InitializationError,
    /// 配置错误
    ConfigurationError,
    /// 依赖错误
    DependencyError,
    /// 权限错误
    PermissionError,
    /// 资源限制错误
    ResourceLimitError,
    /// 执行错误
    ExecutionError,
    /// 通信错误
    CommunicationError,
    /// 版本不兼容错误
    VersionIncompatibilityError,
}

/// 插件错误
#[derive(Debug, Clone, Serialize)]
pub struct PluginError {
    /// 错误类型
    pub error_type: PluginErrorType,
    /// 错误消息
    pub message: String,
    /// 错误详情
    pub details: Option<serde_json::Value>,
    /// 插件 ID
    pub plugin_id: String,
    /// 错误时间
    pub timestamp: DateTime<Utc>,
}

impl std::fmt::Display for PluginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Plugin Error [{}]: {} (plugin: {})", 
               serde_json::to_string(&self.error_type).unwrap_or_default(),
               self.message, 
               self.plugin_id)
    }
}

impl std::error::Error for PluginError {}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_plugin_metadata_serialization() {
        let metadata = PluginMetadata {
            id: "test-plugin".to_string(),
            name: "Test Plugin".to_string(),
            version: "1.0.0".to_string(),
            description: "A test plugin".to_string(),
            author: "Test Author".to_string(),
            license: "MIT".to_string(),
            homepage: None,
            repository: None,
            plugin_type: PluginType::Tool,
            api_version: "1.0".to_string(),
            min_system_version: "1.0.0".to_string(),
            dependencies: Vec::new(),
            permissions: vec![PluginPermission::FileSystem],
            tags: vec!["test".to_string()],
            icon: None,
            created_at: Utc::now(),
        };
        
        let json = serde_json::to_string(&metadata).unwrap();
        let deserialized: PluginMetadata = serde_json::from_str(&json).unwrap();
        
        assert_eq!(metadata.id, deserialized.id);
        assert_eq!(metadata.plugin_type, deserialized.plugin_type);
    }
    
    #[test]
    fn test_plugin_status_transitions() {
        let status = PluginStatus::Uninitialized;
        assert_eq!(status, PluginStatus::Uninitialized);
        
        let status = PluginStatus::Running;
        assert_eq!(status, PluginStatus::Running);
    }
    
    #[test]
    fn test_resource_limits_default() {
        let limits = ResourceLimits::default();
        assert_eq!(limits.max_memory_mb, Some(512));
        assert_eq!(limits.max_cpu_percent, Some(50.0));
    }
}