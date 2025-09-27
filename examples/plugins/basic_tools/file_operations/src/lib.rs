// 文件操作插件示例
// 提供基础的文件和目录操作功能

use std::collections::HashMap;
use std::path::Path;
use async_trait::async_trait;
use serde_json;
use uuid::Uuid;
use chrono::Utc;
use tokio::fs;
use tracing::{debug, info, warn, error};

// 注意：在实际项目中，这些应该从 crate 导入
// use aionix_ai_studio::plugins::plugin_interface::*;
// use aionix_ai_studio::errors::AiStudioError;

// 为了示例，我们定义简化的类型
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
    
    pub fn io(msg: String) -> Self {
        Self { message: msg }
    }
    
    pub fn not_found(msg: &str) -> Self {
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

/// 文件操作插件
pub struct FileOperationsPlugin {
    status: PluginStatus,
    config: Option<PluginConfig>,
    base_path: Option<String>,
}

impl FileOperationsPlugin {
    /// 创建新的文件操作插件实例
    pub fn new() -> Self {
        Self {
            status: PluginStatus::Uninitialized,
            config: None,
            base_path: None,
        }
    }
    
    /// 验证文件路径安全性
    fn validate_path(&self, path: &str) -> Result<std::path::PathBuf, AiStudioError> {
        let path_buf = std::path::PathBuf::from(path);
        
        // 检查路径遍历攻击
        if path.contains("..") {
            return Err(AiStudioError::validation("路径不能包含 .."));
        }
        
        // 如果设置了基础路径，确保操作在基础路径内
        if let Some(ref base) = self.base_path {
            let base_path = std::path::PathBuf::from(base);
            let full_path = base_path.join(&path_buf);
            
            if !full_path.starts_with(&base_path) {
                return Err(AiStudioError::validation("路径超出允许范围"));
            }
            
            Ok(full_path)
        } else {
            Ok(path_buf)
        }
    }
    
    /// 读取文件内容
    async fn read_file(&self, params: &HashMap<String, serde_json::Value>) -> Result<serde_json::Value, AiStudioError> {
        let path = params.get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AiStudioError::validation("缺少参数: path"))?;
        
        let file_path = self.validate_path(path)?;
        
        debug!("读取文件: {:?}", file_path);
        
        let content = fs::read_to_string(&file_path).await
            .map_err(|e| AiStudioError::io(format!("读取文件失败: {}", e)))?;
        
        let metadata = fs::metadata(&file_path).await
            .map_err(|e| AiStudioError::io(format!("获取文件元数据失败: {}", e)))?;
        
        Ok(serde_json::json!({
            "content": content,
            "size": metadata.len(),
            "modified": metadata.modified().ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs()),
            "is_file": metadata.is_file(),
            "is_dir": metadata.is_dir()
        }))
    }
    
    /// 写入文件内容
    async fn write_file(&self, params: &HashMap<String, serde_json::Value>) -> Result<serde_json::Value, AiStudioError> {
        let path = params.get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AiStudioError::validation("缺少参数: path"))?;
        
        let content = params.get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AiStudioError::validation("缺少参数: content"))?;
        
        let file_path = self.validate_path(path)?;
        
        debug!("写入文件: {:?}", file_path);
        
        // 创建父目录（如果不存在）
        if let Some(parent) = file_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).await
                    .map_err(|e| AiStudioError::io(format!("创建目录失败: {}", e)))?;
            }
        }
        
        fs::write(&file_path, content).await
            .map_err(|e| AiStudioError::io(format!("写入文件失败: {}", e)))?;
        
        let metadata = fs::metadata(&file_path).await
            .map_err(|e| AiStudioError::io(format!("获取文件元数据失败: {}", e)))?;
        
        info!("文件写入成功: {:?}", file_path);
        
        Ok(serde_json::json!({
            "success": true,
            "path": path,
            "size": metadata.len(),
            "created": true
        }))
    }
}    ///
 删除文件或目录
    async fn delete_file(&self, params: &HashMap<String, serde_json::Value>) -> Result<serde_json::Value, AiStudioError> {
        let path = params.get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AiStudioError::validation("缺少参数: path"))?;
        
        let recursive = params.get("recursive")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        
        let file_path = self.validate_path(path)?;
        
        if !file_path.exists() {
            return Err(AiStudioError::not_found("文件或目录不存在"));
        }
        
        debug!("删除文件: {:?}, recursive: {}", file_path, recursive);
        
        if file_path.is_dir() {
            if recursive {
                fs::remove_dir_all(&file_path).await
                    .map_err(|e| AiStudioError::io(format!("删除目录失败: {}", e)))?;
            } else {
                fs::remove_dir(&file_path).await
                    .map_err(|e| AiStudioError::io(format!("删除目录失败: {}", e)))?;
            }
        } else {
            fs::remove_file(&file_path).await
                .map_err(|e| AiStudioError::io(format!("删除文件失败: {}", e)))?;
        }
        
        info!("文件删除成功: {:?}", file_path);
        
        Ok(serde_json::json!({
            "success": true,
            "path": path,
            "deleted": true
        }))
    }
    
    /// 列出目录内容
    async fn list_directory(&self, params: &HashMap<String, serde_json::Value>) -> Result<serde_json::Value, AiStudioError> {
        let path = params.get("path")
            .and_then(|v| v.as_str())
            .unwrap_or(".");
        
        let show_hidden = params.get("show_hidden")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        
        let dir_path = self.validate_path(path)?;
        
        if !dir_path.exists() {
            return Err(AiStudioError::not_found("目录不存在"));
        }
        
        if !dir_path.is_dir() {
            return Err(AiStudioError::validation("路径不是目录"));
        }
        
        debug!("列出目录: {:?}", dir_path);
        
        let mut entries = Vec::new();
        let mut dir_entries = fs::read_dir(&dir_path).await
            .map_err(|e| AiStudioError::io(format!("读取目录失败: {}", e)))?;
        
        while let Some(entry) = dir_entries.next_entry().await
            .map_err(|e| AiStudioError::io(format!("读取目录项失败: {}", e)))? {
            
            let file_name = entry.file_name().to_string_lossy().to_string();
            
            // 跳过隐藏文件（除非明确要求显示）
            if !show_hidden && file_name.starts_with('.') {
                continue;
            }
            
            let metadata = entry.metadata().await
                .map_err(|e| AiStudioError::io(format!("获取文件元数据失败: {}", e)))?;
            
            entries.push(serde_json::json!({
                "name": file_name,
                "path": entry.path().to_string_lossy(),
                "is_file": metadata.is_file(),
                "is_dir": metadata.is_dir(),
                "size": metadata.len(),
                "modified": metadata.modified().ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs())
            }));
        }
        
        Ok(serde_json::json!({
            "path": path,
            "entries": entries,
            "count": entries.len()
        }))
    }
    
    /// 创建目录
    async fn create_directory(&self, params: &HashMap<String, serde_json::Value>) -> Result<serde_json::Value, AiStudioError> {
        let path = params.get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AiStudioError::validation("缺少参数: path"))?;
        
        let recursive = params.get("recursive")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        
        let dir_path = self.validate_path(path)?;
        
        debug!("创建目录: {:?}, recursive: {}", dir_path, recursive);
        
        if recursive {
            fs::create_dir_all(&dir_path).await
                .map_err(|e| AiStudioError::io(format!("创建目录失败: {}", e)))?;
        } else {
            fs::create_dir(&dir_path).await
                .map_err(|e| AiStudioError::io(format!("创建目录失败: {}", e)))?;
        }
        
        info!("目录创建成功: {:?}", dir_path);
        
        Ok(serde_json::json!({
            "success": true,
            "path": path,
            "created": true
        }))
    }
    
    /// 复制文件或目录
    async fn copy_file(&self, params: &HashMap<String, serde_json::Value>) -> Result<serde_json::Value, AiStudioError> {
        let source = params.get("source")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AiStudioError::validation("缺少参数: source"))?;
        
        let destination = params.get("destination")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AiStudioError::validation("缺少参数: destination"))?;
        
        let source_path = self.validate_path(source)?;
        let dest_path = self.validate_path(destination)?;
        
        if !source_path.exists() {
            return Err(AiStudioError::not_found("源文件不存在"));
        }
        
        debug!("复制文件: {:?} -> {:?}", source_path, dest_path);
        
        if source_path.is_file() {
            // 创建目标目录（如果需要）
            if let Some(parent) = dest_path.parent() {
                if !parent.exists() {
                    fs::create_dir_all(parent).await
                        .map_err(|e| AiStudioError::io(format!("创建目录失败: {}", e)))?;
                }
            }
            
            fs::copy(&source_path, &dest_path).await
                .map_err(|e| AiStudioError::io(format!("复制文件失败: {}", e)))?;
        } else {
            return Err(AiStudioError::validation("目录复制暂不支持"));
        }
        
        info!("文件复制成功: {:?} -> {:?}", source_path, dest_path);
        
        Ok(serde_json::json!({
            "success": true,
            "source": source,
            "destination": destination,
            "copied": true
        }))
    }
    
    /// 获取文件信息
    async fn get_file_info(&self, params: &HashMap<String, serde_json::Value>) -> Result<serde_json::Value, AiStudioError> {
        let path = params.get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AiStudioError::validation("缺少参数: path"))?;
        
        let file_path = self.validate_path(path)?;
        
        if !file_path.exists() {
            return Err(AiStudioError::not_found("文件或目录不存在"));
        }
        
        let metadata = fs::metadata(&file_path).await
            .map_err(|e| AiStudioError::io(format!("获取文件元数据失败: {}", e)))?;
        
        Ok(serde_json::json!({
            "path": path,
            "exists": true,
            "is_file": metadata.is_file(),
            "is_dir": metadata.is_dir(),
            "size": metadata.len(),
            "readonly": metadata.permissions().readonly(),
            "modified": metadata.modified().ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs()),
            "created": metadata.created().ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs()),
            "accessed": metadata.accessed().ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
        }))
    }
}

#[async_trait]
impl Plugin for FileOperationsPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: "file-operations".to_string(),
            name: "文件操作插件".to_string(),
            version: "1.0.0".to_string(),
            description: "提供基础的文件和目录操作功能，包括读写、创建、删除、复制等操作".to_string(),
            author: "Aionix AI Studio".to_string(),
            license: "MIT".to_string(),
            homepage: Some("https://github.com/aionix/ai-studio".to_string()),
            repository: Some("https://github.com/aionix/ai-studio".to_string()),
            plugin_type: PluginType::Tool,
            api_version: "1.0".to_string(),
            min_system_version: "1.0.0".to_string(),
            dependencies: Vec::new(),
            permissions: vec![PluginPermission::FileSystem],
            tags: vec![
                "file".to_string(),
                "filesystem".to_string(),
                "io".to_string(),
                "utility".to_string()
            ],
            icon: Some("📁".to_string()),
            created_at: Utc::now(),
        }
    }
    
    async fn initialize(&mut self, config: PluginConfig) -> Result<(), AiStudioError> {
        info!("初始化文件操作插件");
        
        // 验证配置
        self.validate_config(&config)?;
        
        // 设置基础路径（如果配置了）
        if let Some(base_path) = config.parameters.get("base_path") {
            if let Some(path_str) = base_path.as_str() {
                self.base_path = Some(path_str.to_string());
                info!("设置基础路径: {}", path_str);
            }
        }
        
        self.config = Some(config);
        self.status = PluginStatus::Initialized;
        
        Ok(())
    }
    
    async fn start(&mut self) -> Result<(), AiStudioError> {
        info!("启动文件操作插件");
        self.status = PluginStatus::Running;
        Ok(())
    }
    
    async fn stop(&mut self) -> Result<(), AiStudioError> {
        info!("停止文件操作插件");
        self.status = PluginStatus::Stopped;
        Ok(())
    }
    
    async fn shutdown(&mut self) -> Result<(), AiStudioError> {
        info!("关闭文件操作插件");
        self.config = None;
        self.base_path = None;
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
            "read_file" => self.read_file(&params).await,
            "write_file" => self.write_file(&params).await,
            "delete_file" => self.delete_file(&params).await,
            "list_directory" => self.list_directory(&params).await,
            "create_directory" => self.create_directory(&params).await,
            "copy_file" => self.copy_file(&params).await,
            "get_file_info" => self.get_file_info(&params).await,
            _ => Err(AiStudioError::validation(&format!("未知方法: {}", method)))
        }
    }
    
    async fn health_check(&self) -> Result<PluginHealth, AiStudioError> {
        let start_time = std::time::Instant::now();
        
        let healthy = self.status == PluginStatus::Running;
        let mut details = HashMap::new();
        
        // 检查基础路径是否可访问（如果设置了）
        if let Some(ref base_path) = self.base_path {
            let path_accessible = Path::new(base_path).exists();
            details.insert("base_path_accessible".to_string(), serde_json::Value::Bool(path_accessible));
            details.insert("base_path".to_string(), serde_json::Value::String(base_path.clone()));
        }
        
        details.insert("status".to_string(), serde_json::json!(self.status));
        
        let response_time = start_time.elapsed().as_millis() as u64;
        
        Ok(PluginHealth {
            healthy,
            message: if healthy {
                "文件操作插件运行正常".to_string()
            } else {
                format!("文件操作插件状态异常: {:?}", self.status)
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
                "base_path": {
                    "type": "string",
                    "description": "基础路径，所有文件操作将限制在此路径下",
                    "default": null
                },
                "max_file_size_mb": {
                    "type": "integer",
                    "description": "最大文件大小限制（MB）",
                    "minimum": 1,
                    "maximum": 1024,
                    "default": 100
                },
                "allowed_extensions": {
                    "type": "array",
                    "description": "允许的文件扩展名列表",
                    "items": {
                        "type": "string"
                    },
                    "default": []
                },
                "forbidden_paths": {
                    "type": "array",
                    "description": "禁止访问的路径列表",
                    "items": {
                        "type": "string"
                    },
                    "default": []
                }
            }
        })
    }
    
    fn validate_config(&self, config: &PluginConfig) -> Result<(), AiStudioError> {
        // 验证基础路径
        if let Some(base_path) = config.parameters.get("base_path") {
            if let Some(path_str) = base_path.as_str() {
                let path = Path::new(path_str);
                if !path.exists() {
                    return Err(AiStudioError::validation(&format!("基础路径不存在: {}", path_str)));
                }
                if !path.is_dir() {
                    return Err(AiStudioError::validation(&format!("基础路径不是目录: {}", path_str)));
                }
            }
        }
        
        // 验证文件大小限制
        if let Some(max_size) = config.parameters.get("max_file_size_mb") {
            if let Some(size_val) = max_size.as_u64() {
                if size_val == 0 || size_val > 1024 {
                    return Err(AiStudioError::validation("max_file_size_mb 必须在 1-1024 之间"));
                }
            }
        }
        
        Ok(())
    }
}

// 插件工厂实现
pub struct FileOperationsPluginFactory;

impl FileOperationsPluginFactory {
    pub fn new() -> Self {
        Self
    }
    
    pub fn create_plugin(&self) -> Result<Box<dyn Plugin>, AiStudioError> {
        Ok(Box::new(FileOperationsPlugin::new()))
    }
    
    pub fn metadata(&self) -> PluginMetadata {
        FileOperationsPlugin::new().metadata()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    async fn create_test_plugin() -> (FileOperationsPlugin, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let mut plugin = FileOperationsPlugin::new();
        
        let mut config_params = HashMap::new();
        config_params.insert(
            "base_path".to_string(),
            serde_json::Value::String(temp_dir.path().to_string_lossy().to_string())
        );
        
        let config = PluginConfig {
            plugin_id: "test".to_string(),
            parameters: config_params,
            environment: HashMap::new(),
            resource_limits: Default::default(),
            security_settings: Default::default(),
        };
        
        plugin.initialize(config).await.unwrap();
        plugin.start().await.unwrap();
        
        (plugin, temp_dir)
    }
    
    #[tokio::test]
    async fn test_plugin_lifecycle() {
        let mut plugin = FileOperationsPlugin::new();
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
    async fn test_write_and_read_file() {
        let (plugin, _temp_dir) = create_test_plugin().await;
        
        let context = PluginContext {
            tenant_id: Uuid::new_v4(),
            user_id: Some(Uuid::new_v4()),
            session_id: None,
            request_id: Uuid::new_v4(),
            variables: HashMap::new(),
            timestamp: Utc::now(),
        };
        
        // 写入文件
        let mut write_params = HashMap::new();
        write_params.insert("path".to_string(), serde_json::Value::String("test.txt".to_string()));
        write_params.insert("content".to_string(), serde_json::Value::String("Hello, World!".to_string()));
        
        let write_result = plugin.handle_call("write_file", write_params, &context).await;
        assert!(write_result.is_ok());
        
        // 读取文件
        let mut read_params = HashMap::new();
        read_params.insert("path".to_string(), serde_json::Value::String("test.txt".to_string()));
        
        let read_result = plugin.handle_call("read_file", read_params, &context).await;
        assert!(read_result.is_ok());
        
        let read_data = read_result.unwrap();
        assert_eq!(read_data["content"], "Hello, World!");
    }
    
    #[tokio::test]
    async fn test_create_and_list_directory() {
        let (plugin, _temp_dir) = create_test_plugin().await;
        
        let context = PluginContext {
            tenant_id: Uuid::new_v4(),
            user_id: Some(Uuid::new_v4()),
            session_id: None,
            request_id: Uuid::new_v4(),
            variables: HashMap::new(),
            timestamp: Utc::now(),
        };
        
        // 创建目录
        let mut create_params = HashMap::new();
        create_params.insert("path".to_string(), serde_json::Value::String("test_dir".to_string()));
        
        let create_result = plugin.handle_call("create_directory", create_params, &context).await;
        assert!(create_result.is_ok());
        
        // 列出根目录
        let list_params = HashMap::new();
        let list_result = plugin.handle_call("list_directory", list_params, &context).await;
        assert!(list_result.is_ok());
        
        let list_data = list_result.unwrap();
        assert!(list_data["entries"].as_array().unwrap().len() > 0);
    }
    
    #[tokio::test]
    async fn test_health_check() {
        let (plugin, _temp_dir) = create_test_plugin().await;
        
        let health = plugin.health_check().await.unwrap();
        assert!(health.healthy);
        assert!(health.response_time_ms < 1000);
    }
    
    #[tokio::test]
    async fn test_invalid_method() {
        let (plugin, _temp_dir) = create_test_plugin().await;
        
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
}