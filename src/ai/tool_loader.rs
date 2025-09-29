// 工具加载器
// 实现动态工具加载和插件系统

use std::sync::Arc;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use tracing::{info, warn, error, debug};
use tokio::fs;

use crate::ai::{
    agent_runtime::{Tool, ToolMetadata, ToolEnum},
    tool_manager::{ToolManager, ToolPermissions},
    tools::ToolFactory,
};
use crate::errors::AiStudioError;

/// 工具加载器
pub struct ToolLoader {
    /// 工具管理器
    tool_manager: Arc<ToolManager>,
    /// 工具目录路径
    tools_directory: PathBuf,
    /// 加载配置
    config: ToolLoaderConfig,
}

/// 工具加载器配置
#[derive(Debug, Clone)]
pub struct ToolLoaderConfig {
    /// 是否启用自动加载
    pub auto_load: bool,
    /// 工具配置文件名
    pub config_file_name: String,
    /// 支持的工具类型
    pub supported_tool_types: Vec<ToolType>,
    /// 是否启用热重载
    pub hot_reload: bool,
    /// 扫描间隔（秒）
    pub scan_interval_seconds: u64,
}

impl Default for ToolLoaderConfig {
    fn default() -> Self {
        Self {
            auto_load: true,
            config_file_name: "tool.json".to_string(),
            supported_tool_types: vec![
                ToolType::Builtin,
                ToolType::Script,
                ToolType::Plugin,
            ],
            hot_reload: false,
            scan_interval_seconds: 60,
        }
    }
}

/// 工具类型
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolType {
    /// 内置工具
    Builtin,
    /// 脚本工具
    Script,
    /// 插件工具
    Plugin,
    /// 外部工具
    External,
}

/// 工具配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolConfig {
    /// 工具名称
    pub name: String,
    /// 工具类型
    pub tool_type: ToolType,
    /// 工具描述
    pub description: String,
    /// 工具版本
    pub version: String,
    /// 工具类别
    pub category: String,
    /// 是否需要权限
    pub requires_permission: bool,
    /// 权限配置
    pub permissions: Option<ToolPermissions>,
    /// 工具参数模式
    pub parameters_schema: serde_json::Value,
    /// 工具实现配置
    pub implementation: ToolImplementation,
    /// 是否启用
    pub enabled: bool,
}

/// 工具实现配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ToolImplementation {
    /// 内置工具实现
    Builtin {
        /// 工具类名
        class_name: String,
    },
    /// 脚本工具实现
    Script {
        /// 脚本路径
        script_path: String,
        /// 脚本语言
        language: ScriptLanguage,
        /// 环境变量
        environment: HashMap<String, String>,
    },
    /// 插件工具实现
    Plugin {
        /// 插件路径
        plugin_path: String,
        /// 入口函数
        entry_point: String,
    },
    /// 外部工具实现
    External {
        /// 命令行
        command: String,
        /// 参数模板
        args_template: String,
        /// 工作目录
        working_directory: Option<String>,
    },
}

/// 脚本语言
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ScriptLanguage {
    Python,
    JavaScript,
    Shell,
    PowerShell,
}

/// 工具加载结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolLoadResult {
    /// 加载的工具数
    pub loaded_count: usize,
    /// 失败的工具数
    pub failed_count: usize,
    /// 跳过的工具数
    pub skipped_count: usize,
    /// 失败的工具列表
    pub failed_tools: Vec<String>,
    /// 加载详情
    pub details: Vec<ToolLoadDetail>,
}

/// 工具加载详情
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolLoadDetail {
    /// 工具名称
    pub tool_name: String,
    /// 加载状态
    pub status: LoadStatus,
    /// 错误信息
    pub error: Option<String>,
    /// 加载时间（毫秒）
    pub load_time_ms: u64,
}

/// 加载状态
#[derive(Debug, Clone, PartialEq)]
pub enum LoadStatus {
    Success,
    Failed,
    Skipped,
}

impl ToolLoader {
    /// 创建新的工具加载器
    pub fn new(
        tool_manager: Arc<ToolManager>,
        tools_directory: PathBuf,
        config: Option<ToolLoaderConfig>,
    ) -> Self {
        Self {
            tool_manager,
            tools_directory,
            config: config.unwrap_or_default(),
        }
    }
    
    /// 加载所有工具
    pub async fn load_all_tools(&self) -> Result<ToolLoadResult, AiStudioError> {
        info!("开始加载工具目录: {}", self.tools_directory.display());
        
        let mut result = ToolLoadResult {
            loaded_count: 0,
            failed_count: 0,
            skipped_count: 0,
            failed_tools: Vec::new(),
            details: Vec::new(),
        };
        
        // 首先加载内置工具
        if self.config.supported_tool_types.contains(&ToolType::Builtin) {
            self.load_builtin_tools(&mut result).await?;
        }
        
        // 然后扫描工具目录
        if self.tools_directory.exists() {
            self.scan_tools_directory(&mut result).await?;
        } else {
            warn!("工具目录不存在: {}", self.tools_directory.display());
        }
        
        info!("工具加载完成: 成功={}, 失败={}, 跳过={}", 
              result.loaded_count, result.failed_count, result.skipped_count);
        
        Ok(result)
    }
    
    /// 加载内置工具
    async fn load_builtin_tools(&self, result: &mut ToolLoadResult) -> Result<(), AiStudioError> {
        debug!("加载内置工具");
        
        let builtin_tools = ToolFactory::create_basic_tools();
        
        for tool in builtin_tools {
            let start_time = std::time::Instant::now();
            let tool_name = tool.metadata().name.clone();
            
            match self.tool_manager.register_tool(tool, None).await {
                Ok(_) => {
                    result.loaded_count += 1;
                    result.details.push(ToolLoadDetail {
                        tool_name,
                        status: LoadStatus::Success,
                        error: None,
                        load_time_ms: start_time.elapsed().as_millis() as u64,
                    });
                }
                Err(e) => {
                    error!("加载内置工具失败: {} - {}", tool_name, e);
                    result.failed_count += 1;
                    result.details.push(ToolLoadDetail {
                        tool_name,
                        status: LoadStatus::Failed,
                        error: Some(e.to_string()),
                        load_time_ms: start_time.elapsed().as_millis() as u64,
                    });
                }
            }
        }
        
        Ok(())
    }
    
    /// 扫描工具目录
    async fn scan_tools_directory(&self, result: &mut ToolLoadResult) -> Result<(), AiStudioError> {
        debug!("扫描工具目录: {}", self.tools_directory.display());
        
        let mut entries = fs::read_dir(&self.tools_directory).await.map_err(|e| {
            error!("读取工具目录失败: {}", e);
            AiStudioError::internal(format!("读取工具目录失败: {}", e))
        })?;
        
        while let Some(entry) = entries.next_entry().await.map_err(|e| {
            error!("读取目录条目失败: {}", e);
            AiStudioError::internal(format!("读取目录条目失败: {}", e))
        })? {
            let path = entry.path();
            
            if path.is_dir() {
                // 检查是否包含工具配置文件
                let config_path = path.join(&self.config.config_file_name);
                if config_path.exists() {
                    self.load_tool_from_directory(&path, result).await;
                }
            }
        }
        
        Ok(())
    }
    
    /// 从目录加载工具
    async fn load_tool_from_directory(&self, tool_dir: &Path, result: &mut ToolLoadResult) {
        let start_time = std::time::Instant::now();
        let config_path = tool_dir.join(&self.config.config_file_name);
        
        debug!("加载工具配置: {}", config_path.display());
        
        // 读取工具配置
        let tool_config = match self.read_tool_config(&config_path).await {
            Ok(config) => config,
            Err(e) => {
                error!("读取工具配置失败: {} - {}", config_path.display(), e);
                result.failed_count += 1;
                result.details.push(ToolLoadDetail {
                    tool_name: tool_dir.file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string(),
                    status: LoadStatus::Failed,
                    error: Some(e.to_string()),
                    load_time_ms: start_time.elapsed().as_millis() as u64,
                });
                return;
            }
        };
        
        // 检查工具是否启用
        if !tool_config.enabled {
            debug!("跳过已禁用的工具: {}", tool_config.name);
            result.skipped_count += 1;
            result.details.push(ToolLoadDetail {
                tool_name: tool_config.name,
                status: LoadStatus::Skipped,
                error: Some("工具已禁用".to_string()),
                load_time_ms: start_time.elapsed().as_millis() as u64,
            });
            return;
        }
        
        // 检查工具类型是否支持
        if !self.config.supported_tool_types.contains(&tool_config.tool_type) {
            debug!("跳过不支持的工具类型: {} ({})", tool_config.name, 
                   serde_json::to_string(&tool_config.tool_type).unwrap_or_default());
            result.skipped_count += 1;
            result.details.push(ToolLoadDetail {
                tool_name: tool_config.name,
                status: LoadStatus::Skipped,
                error: Some("不支持的工具类型".to_string()),
                load_time_ms: start_time.elapsed().as_millis() as u64,
            });
            return;
        }
        
        // 创建工具实例
        match self.create_tool_instance(&tool_config, tool_dir).await {
            Ok(tool) => {
                // 注册工具
                match self.tool_manager.register_tool(tool, tool_config.permissions).await {
                    Ok(_) => {
                        info!("工具加载成功: {}", tool_config.name);
                        result.loaded_count += 1;
                        result.details.push(ToolLoadDetail {
                            tool_name: tool_config.name,
                            status: LoadStatus::Success,
                            error: None,
                            load_time_ms: start_time.elapsed().as_millis() as u64,
                        });
                    }
                    Err(e) => {
                        error!("工具注册失败: {} - {}", tool_config.name, e);
                        result.failed_count += 1;
                        result.details.push(ToolLoadDetail {
                            tool_name: tool_config.name,
                            status: LoadStatus::Failed,
                            error: Some(e.to_string()),
                            load_time_ms: start_time.elapsed().as_millis() as u64,
                        });
                    }
                }
            }
            Err(e) => {
                error!("创建工具实例失败: {} - {}", tool_config.name, e);
                result.failed_count += 1;
                result.details.push(ToolLoadDetail {
                    tool_name: tool_config.name,
                    status: LoadStatus::Failed,
                    error: Some(e.to_string()),
                    load_time_ms: start_time.elapsed().as_millis() as u64,
                });
            }
        }
    }
    
    /// 读取工具配置
    async fn read_tool_config(&self, config_path: &Path) -> Result<ToolConfig, AiStudioError> {
        let content = fs::read_to_string(config_path).await.map_err(|e| {
            AiStudioError::internal(format!("读取配置文件失败: {}", e))
        })?;
        
        let config: ToolConfig = serde_json::from_str(&content).map_err(|e| {
            AiStudioError::validation("config".to_string(), format!("解析配置文件失败: {}", e))
        })?;
        
        // 验证配置
        self.validate_tool_config(&config)?;
        
        Ok(config)
    }
    
    /// 验证工具配置
    fn validate_tool_config(&self, config: &ToolConfig) -> Result<(), AiStudioError> {
        if config.name.is_empty() {
            return Err(AiStudioError::validation("name", "工具名称不能为空"));
        }
        
        if config.version.is_empty() {
            return Err(AiStudioError::validation("version", "工具版本不能为空"));
        }
        
        // 验证参数模式
        if !config.parameters_schema.is_object() && !config.parameters_schema.is_null() {
            return Err(AiStudioError::validation("parameters_schema", "参数模式必须是对象或 null"));
        }
        
        Ok(())
    }
    
    /// 创建工具实例
    async fn create_tool_instance(
        &self,
        config: &ToolConfig,
        tool_dir: &Path,
    ) -> Result<ToolEnum, AiStudioError> {
        match &config.implementation {
            ToolImplementation::Builtin { class_name } => {
                self.create_builtin_tool(class_name)
            }
            ToolImplementation::Script { script_path, language, environment } => {
                self.create_script_tool(config, tool_dir, script_path, language, environment).await
            }
            ToolImplementation::Plugin { plugin_path, entry_point } => {
                self.create_plugin_tool(config, tool_dir, plugin_path, entry_point).await
            }
            ToolImplementation::External { command, args_template, working_directory } => {
                self.create_external_tool(config, command, args_template, working_directory.as_deref()).await
            }
        }
    }
    
    /// 创建内置工具
    fn create_builtin_tool(&self, class_name: &str) -> Result<ToolEnum, AiStudioError> {
        ToolFactory::create_tool(class_name)
            .ok_or_else(|| AiStudioError::not_found(&format!("未知的内置工具: {}", class_name)))
    }
    
    /// 创建脚本工具
    async fn create_script_tool(
        &self,
        config: &ToolConfig,
        tool_dir: &Path,
        script_path: &str,
        language: &ScriptLanguage,
        environment: &HashMap<String, String>,
    ) -> Result<ToolEnum, AiStudioError> {
        // TODO: 实现脚本工具创建
        // 这里需要实现一个通用的脚本工具包装器
        Err(AiStudioError::internal("脚本工具暂未实现"))
    }
    
    /// 创建插件工具
    async fn create_plugin_tool(
        &self,
        config: &ToolConfig,
        tool_dir: &Path,
        plugin_path: &str,
        entry_point: &str,
    ) -> Result<ToolEnum, AiStudioError> {
        // TODO: 实现插件工具创建
        // 这里需要实现动态库加载和插件接口
        Err(AiStudioError::internal("插件工具暂未实现"))
    }
    
    /// 创建外部工具
    async fn create_external_tool(
        &self,
        config: &ToolConfig,
        command: &str,
        args_template: &str,
        working_directory: Option<&str>,
    ) -> Result<ToolEnum, AiStudioError> {
        // TODO: 实现外部工具创建
        // 这里需要实现一个通用的外部命令工具包装器
        Err(AiStudioError::internal("外部工具暂未实现"))
    }
    
    /// 重新加载工具
    pub async fn reload_tool(&self, tool_name: &str) -> Result<(), AiStudioError> {
        info!("重新加载工具: {}", tool_name);
        
        // 先注销现有工具
        if let Err(e) = self.tool_manager.unregister_tool(tool_name).await {
            warn!("注销工具失败: {} - {}", tool_name, e);
        }
        
        // 重新扫描并加载工具
        let mut result = ToolLoadResult {
            loaded_count: 0,
            failed_count: 0,
            skipped_count: 0,
            failed_tools: Vec::new(),
            details: Vec::new(),
        };
        
        self.scan_tools_directory(&mut result).await?;
        
        // 检查是否成功重新加载
        let loaded = result.loaded_count > 0;
        
        if loaded {
            info!("工具重新加载成功: {}", tool_name);
            Ok(())
        } else {
            Err(AiStudioError::internal(&format!("工具重新加载失败: {}", tool_name)))
        }
    }
    
    /// 启动热重载监控
    pub async fn start_hot_reload(&self) -> Result<(), AiStudioError> {
        if !self.config.hot_reload {
            return Ok(());
        }
        
        info!("启动工具热重载监控");
        
        // TODO: 实现文件系统监控
        // 这里需要使用 notify 或类似的库来监控文件变化
        
        Ok(())
    }
}

/// 工具加载器工厂
pub struct ToolLoaderFactory;

impl ToolLoaderFactory {
    /// 创建工具加载器实例
    pub fn create(
        tool_manager: Arc<ToolManager>,
        tools_directory: PathBuf,
        config: Option<ToolLoaderConfig>,
    ) -> Arc<ToolLoader> {
        Arc::new(ToolLoader::new(tool_manager, tools_directory, config))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[test]
    fn test_tool_config_serialization() {
        let config = ToolConfig {
            name: "test_tool".to_string(),
            tool_type: ToolType::Builtin,
            description: "测试工具".to_string(),
            version: "1.0.0".to_string(),
            category: "test".to_string(),
            requires_permission: false,
            permissions: None,
            parameters_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "input": {"type": "string"}
                }
            }),
            implementation: ToolImplementation::Builtin {
                class_name: "TestTool".to_string(),
            },
            enabled: true,
        };
        
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: ToolConfig = serde_json::from_str(&json).unwrap();
        
        assert_eq!(config.name, deserialized.name);
        assert_eq!(config.tool_type, deserialized.tool_type);
    }
    
    #[tokio::test]
    async fn test_tool_loader_creation() {
        let temp_dir = TempDir::new().unwrap();
        let tool_manager = Arc::new(ToolManager::new(None));
        
        let loader = ToolLoader::new(
            tool_manager,
            temp_dir.path().to_path_buf(),
            None,
        );
        
        assert_eq!(loader.tools_directory, temp_dir.path());
    }
}