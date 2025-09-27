// 文件操作工具实现

use std::collections::HashMap;
use std::path::Path;
use async_trait::async_trait;
use serde_json;
use tracing::{debug, error, warn};
use tokio::fs;

use crate::ai::agent_runtime::{Tool, ToolResult, ToolMetadata, ExecutionContext};
use crate::errors::AiStudioError;

/// 文件操作工具
pub struct FileTool {
    /// 工具配置
    config: FileToolConfig,
}

/// 文件工具配置
#[derive(Debug, Clone)]
pub struct FileToolConfig {
    /// 允许的文件扩展名
    pub allowed_extensions: Vec<String>,
    /// 最大文件大小（字节）
    pub max_file_size: u64,
    /// 允许的操作
    pub allowed_operations: Vec<String>,
    /// 基础目录（安全限制）
    pub base_directory: Option<String>,
}

impl Default for FileToolConfig {
    fn default() -> Self {
        Self {
            allowed_extensions: vec![
                "txt".to_string(),
                "md".to_string(),
                "json".to_string(),
                "csv".to_string(),
                "log".to_string(),
            ],
            max_file_size: 10 * 1024 * 1024, // 10MB
            allowed_operations: vec![
                "read".to_string(),
                "write".to_string(),
                "append".to_string(),
                "list".to_string(),
                "exists".to_string(),
                "size".to_string(),
            ],
            base_directory: None,
        }
    }
}

impl FileTool {
    /// 创建新的文件工具
    pub fn new() -> Self {
        Self {
            config: FileToolConfig::default(),
        }
    }
    
    /// 使用自定义配置创建文件工具
    pub fn with_config(config: FileToolConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl Tool for FileTool {
    async fn execute(
        &self,
        parameters: HashMap<String, serde_json::Value>,
        _context: &ExecutionContext,
    ) -> Result<ToolResult, AiStudioError> {
        debug!("执行文件工具");
        
        // 提取操作类型
        let operation = parameters.get("operation")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AiStudioError::validation("缺少必需参数: operation"))?;
        
        if !self.config.allowed_operations.contains(&operation.to_string()) {
            return Err(AiStudioError::validation(&format!("不允许的操作: {}", operation)));
        }
        
        debug!("文件操作: {}", operation);
        
        let start_time = std::time::Instant::now();
        
        // 执行文件操作
        let result = match operation {
            "read" => self.read_file(&parameters).await?,
            "write" => self.write_file(&parameters).await?,
            "append" => self.append_file(&parameters).await?,
            "list" => self.list_directory(&parameters).await?,
            "exists" => self.check_exists(&parameters).await?,
            "size" => self.get_file_size(&parameters).await?,
            _ => return Err(AiStudioError::validation(&format!("未实现的操作: {}", operation))),
        };
        
        let execution_time = start_time.elapsed().as_millis() as u64;
        
        Ok(ToolResult {
            success: true,
            data: result,
            error: None,
            execution_time_ms: execution_time,
            message: Some(format!("文件操作 '{}' 执行完成", operation)),
        })
    }
    
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            name: "file".to_string(),
            description: "执行文件系统操作（读取、写入、列表等）".to_string(),
            parameters_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "operation": {
                        "type": "string",
                        "description": "文件操作类型",
                        "enum": self.config.allowed_operations
                    },
                    "path": {
                        "type": "string",
                        "description": "文件或目录路径"
                    },
                    "content": {
                        "type": "string",
                        "description": "写入或追加的内容（write/append 操作需要）"
                    },
                    "encoding": {
                        "type": "string",
                        "description": "文件编码",
                        "default": "utf-8"
                    }
                },
                "required": ["operation", "path"]
            }),
            category: "filesystem".to_string(),
            requires_permission: true,
            version: "1.0.0".to_string(),
        }
    }
    
    fn validate_parameters(
        &self,
        parameters: &HashMap<String, serde_json::Value>,
    ) -> Result<(), AiStudioError> {
        // 验证操作参数
        let operation = parameters.get("operation")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AiStudioError::validation("缺少必需参数: operation"))?;
        
        if !self.config.allowed_operations.contains(&operation.to_string()) {
            return Err(AiStudioError::validation(&format!("不允许的操作: {}", operation)));
        }
        
        // 验证路径参数
        let path = parameters.get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AiStudioError::validation("缺少必需参数: path"))?;
        
        if path.is_empty() {
            return Err(AiStudioError::validation("路径不能为空"));
        }
        
        // 安全检查：防止路径遍历攻击
        if path.contains("..") || path.contains("~") {
            return Err(AiStudioError::validation("路径包含不安全字符"));
        }
        
        // 检查基础目录限制
        if let Some(ref base_dir) = self.config.base_directory {
            let full_path = Path::new(base_dir).join(path);
            if !full_path.starts_with(base_dir) {
                return Err(AiStudioError::validation("路径超出允许的基础目录"));
            }
        }
        
        // 检查文件扩展名
        if matches!(operation, "read" | "write" | "append") {
            if let Some(extension) = Path::new(path).extension() {
                let ext_str = extension.to_string_lossy().to_lowercase();
                if !self.config.allowed_extensions.contains(&ext_str) {
                    return Err(AiStudioError::validation(&format!("不允许的文件扩展名: {}", ext_str)));
                }
            }
        }
        
        // 验证内容参数（写入和追加操作需要）
        if matches!(operation, "write" | "append") {
            if !parameters.contains_key("content") {
                return Err(AiStudioError::validation(&format!("操作 {} 需要参数 content", operation)));
            }
            
            if !parameters.get("content").unwrap().is_string() {
                return Err(AiStudioError::validation("content 必须是字符串"));
            }
        }
        
        Ok(())
    }
}

impl FileTool {
    /// 读取文件
    async fn read_file(
        &self,
        parameters: &HashMap<String, serde_json::Value>,
    ) -> Result<serde_json::Value, AiStudioError> {
        let path = parameters.get("path").unwrap().as_str().unwrap();
        let full_path = self.resolve_path(path)?;
        
        debug!("读取文件: {}", full_path.display());
        
        // 检查文件是否存在
        if !full_path.exists() {
            return Err(AiStudioError::not_found(&format!("文件不存在: {}", path)));
        }
        
        // 检查文件大小
        let metadata = fs::metadata(&full_path).await.map_err(|e| {
            error!("获取文件元数据失败: {}", e);
            AiStudioError::io(format!("获取文件元数据失败: {}", e))
        })?;
        
        if metadata.len() > self.config.max_file_size {
            return Err(AiStudioError::validation(&format!(
                "文件太大: {} 字节，最大允许: {} 字节",
                metadata.len(),
                self.config.max_file_size
            )));
        }
        
        // 读取文件内容
        let content = fs::read_to_string(&full_path).await.map_err(|e| {
            error!("读取文件失败: {}", e);
            AiStudioError::io(format!("读取文件失败: {}", e))
        })?;
        
        Ok(serde_json::json!({
            "operation": "read",
            "path": path,
            "content": content,
            "size": metadata.len(),
            "modified": metadata.modified().ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
        }))
    }
    
    /// 写入文件
    async fn write_file(
        &self,
        parameters: &HashMap<String, serde_json::Value>,
    ) -> Result<serde_json::Value, AiStudioError> {
        let path = parameters.get("path").unwrap().as_str().unwrap();
        let content = parameters.get("content").unwrap().as_str().unwrap();
        let full_path = self.resolve_path(path)?;
        
        debug!("写入文件: {}", full_path.display());
        
        // 检查内容大小
        if content.len() > self.config.max_file_size as usize {
            return Err(AiStudioError::validation(&format!(
                "内容太大: {} 字节，最大允许: {} 字节",
                content.len(),
                self.config.max_file_size
            )));
        }
        
        // 创建父目录（如果不存在）
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).await.map_err(|e| {
                error!("创建目录失败: {}", e);
                AiStudioError::io(format!("创建目录失败: {}", e))
            })?;
        }
        
        // 写入文件
        fs::write(&full_path, content).await.map_err(|e| {
            error!("写入文件失败: {}", e);
            AiStudioError::io(format!("写入文件失败: {}", e))
        })?;
        
        Ok(serde_json::json!({
            "operation": "write",
            "path": path,
            "bytes_written": content.len(),
            "success": true
        }))
    }
    
    /// 追加文件
    async fn append_file(
        &self,
        parameters: &HashMap<String, serde_json::Value>,
    ) -> Result<serde_json::Value, AiStudioError> {
        let path = parameters.get("path").unwrap().as_str().unwrap();
        let content = parameters.get("content").unwrap().as_str().unwrap();
        let full_path = self.resolve_path(path)?;
        
        debug!("追加文件: {}", full_path.display());
        
        // 检查现有文件大小
        if full_path.exists() {
            let metadata = fs::metadata(&full_path).await.map_err(|e| {
                error!("获取文件元数据失败: {}", e);
                AiStudioError::io(format!("获取文件元数据失败: {}", e))
            })?;
            
            let new_size = metadata.len() + content.len() as u64;
            if new_size > self.config.max_file_size {
                return Err(AiStudioError::validation(&format!(
                    "追加后文件将太大: {} 字节，最大允许: {} 字节",
                    new_size,
                    self.config.max_file_size
                )));
            }
        }
        
        // 追加内容
        use tokio::io::AsyncWriteExt;
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&full_path)
            .await
            .map_err(|e| {
                error!("打开文件失败: {}", e);
                AiStudioError::io(format!("打开文件失败: {}", e))
            })?;
        
        file.write_all(content.as_bytes()).await.map_err(|e| {
            error!("追加文件失败: {}", e);
            AiStudioError::io(format!("追加文件失败: {}", e))
        })?;
        
        Ok(serde_json::json!({
            "operation": "append",
            "path": path,
            "bytes_appended": content.len(),
            "success": true
        }))
    }
    
    /// 列出目录
    async fn list_directory(
        &self,
        parameters: &HashMap<String, serde_json::Value>,
    ) -> Result<serde_json::Value, AiStudioError> {
        let path = parameters.get("path").unwrap().as_str().unwrap();
        let full_path = self.resolve_path(path)?;
        
        debug!("列出目录: {}", full_path.display());
        
        if !full_path.exists() {
            return Err(AiStudioError::not_found(&format!("目录不存在: {}", path)));
        }
        
        if !full_path.is_dir() {
            return Err(AiStudioError::validation(&format!("路径不是目录: {}", path)));
        }
        
        let mut entries = Vec::new();
        let mut dir = fs::read_dir(&full_path).await.map_err(|e| {
            error!("读取目录失败: {}", e);
            AiStudioError::io(format!("读取目录失败: {}", e))
        })?;
        
        while let Some(entry) = dir.next_entry().await.map_err(|e| {
            error!("读取目录条目失败: {}", e);
            AiStudioError::io(format!("读取目录条目失败: {}", e))
        })? {
            let metadata = entry.metadata().await.map_err(|e| {
                warn!("获取条目元数据失败: {}", e);
                continue;
            }).ok();
            
            let file_name = entry.file_name().to_string_lossy().to_string();
            let is_dir = entry.file_type().await.map(|ft| ft.is_dir()).unwrap_or(false);
            
            entries.push(serde_json::json!({
                "name": file_name,
                "is_directory": is_dir,
                "size": metadata.as_ref().map(|m| m.len()),
                "modified": metadata.as_ref()
                    .and_then(|m| m.modified().ok())
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs())
            }));
        }
        
        Ok(serde_json::json!({
            "operation": "list",
            "path": path,
            "entries": entries,
            "count": entries.len()
        }))
    }
    
    /// 检查文件是否存在
    async fn check_exists(
        &self,
        parameters: &HashMap<String, serde_json::Value>,
    ) -> Result<serde_json::Value, AiStudioError> {
        let path = parameters.get("path").unwrap().as_str().unwrap();
        let full_path = self.resolve_path(path)?;
        
        let exists = full_path.exists();
        let is_file = full_path.is_file();
        let is_dir = full_path.is_dir();
        
        Ok(serde_json::json!({
            "operation": "exists",
            "path": path,
            "exists": exists,
            "is_file": is_file,
            "is_directory": is_dir
        }))
    }
    
    /// 获取文件大小
    async fn get_file_size(
        &self,
        parameters: &HashMap<String, serde_json::Value>,
    ) -> Result<serde_json::Value, AiStudioError> {
        let path = parameters.get("path").unwrap().as_str().unwrap();
        let full_path = self.resolve_path(path)?;
        
        if !full_path.exists() {
            return Err(AiStudioError::not_found(&format!("文件不存在: {}", path)));
        }
        
        let metadata = fs::metadata(&full_path).await.map_err(|e| {
            error!("获取文件元数据失败: {}", e);
            AiStudioError::io(format!("获取文件元数据失败: {}", e))
        })?;
        
        Ok(serde_json::json!({
            "operation": "size",
            "path": path,
            "size": metadata.len(),
            "is_file": metadata.is_file(),
            "is_directory": metadata.is_dir(),
            "modified": metadata.modified().ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
        }))
    }
    
    /// 解析路径
    fn resolve_path(&self, path: &str) -> Result<std::path::PathBuf, AiStudioError> {
        let path_buf = if let Some(ref base_dir) = self.config.base_directory {
            Path::new(base_dir).join(path)
        } else {
            Path::new(path).to_path_buf()
        };
        
        // 规范化路径
        let canonical_path = path_buf.canonicalize().unwrap_or(path_buf);
        
        // 再次检查基础目录限制
        if let Some(ref base_dir) = self.config.base_directory {
            if !canonical_path.starts_with(base_dir) {
                return Err(AiStudioError::validation("路径超出允许的基础目录"));
            }
        }
        
        Ok(canonical_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_file_tool_validation() {
        let tool = FileTool::new();
        
        // 测试有效参数
        let mut valid_params = HashMap::new();
        valid_params.insert("operation".to_string(), serde_json::Value::String("read".to_string()));
        valid_params.insert("path".to_string(), serde_json::Value::String("test.txt".to_string()));
        assert!(tool.validate_parameters(&valid_params).is_ok());
        
        // 测试路径遍历攻击
        let mut invalid_params = HashMap::new();
        invalid_params.insert("operation".to_string(), serde_json::Value::String("read".to_string()));
        invalid_params.insert("path".to_string(), serde_json::Value::String("../etc/passwd".to_string()));
        assert!(tool.validate_parameters(&invalid_params).is_err());
    }
    
    #[tokio::test]
    async fn test_file_operations() {
        let temp_dir = TempDir::new().unwrap();
        let config = FileToolConfig {
            base_directory: Some(temp_dir.path().to_string_lossy().to_string()),
            ..Default::default()
        };
        let tool = FileTool::with_config(config);
        
        let context = ExecutionContext {
            current_task: None,
            execution_history: Vec::new(),
            context_variables: HashMap::new(),
            session_id: None,
            user_id: None,
        };
        
        // 测试写入文件
        let mut write_params = HashMap::new();
        write_params.insert("operation".to_string(), serde_json::Value::String("write".to_string()));
        write_params.insert("path".to_string(), serde_json::Value::String("test.txt".to_string()));
        write_params.insert("content".to_string(), serde_json::Value::String("Hello, World!".to_string()));
        
        let result = tool.execute(write_params, &context).await.unwrap();
        assert!(result.success);
        
        // 测试读取文件
        let mut read_params = HashMap::new();
        read_params.insert("operation".to_string(), serde_json::Value::String("read".to_string()));
        read_params.insert("path".to_string(), serde_json::Value::String("test.txt".to_string()));
        
        let result = tool.execute(read_params, &context).await.unwrap();
        assert!(result.success);
        assert_eq!(result.data.get("content").unwrap().as_str().unwrap(), "Hello, World!");
    }
}