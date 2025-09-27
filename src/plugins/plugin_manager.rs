// 插件管理器
// 实现插件的注册、加载、卸载和管理

use std::sync::Arc;
use std::collections::HashMap;
use std::path::PathBuf;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{info, warn, error, debug};
use tokio::sync::RwLock;

use crate::plugins::{
    plugin_interface::{
        Plugin, PluginMetadata, PluginConfig, PluginStatus, PluginContext, PluginEvent, 
        PluginEventType, PluginApi, PluginHook, PluginFactory, PluginPermission
    },
    lifecycle::{PluginLifecycleManager, LifecycleConfig, PluginInstanceInfo},
    plugin_registry::{PluginRegistry, RegistryConfig},
    plugin_loader::{PluginLoader, LoaderConfig},
};
use crate::errors::AiStudioError;

/// 插件管理器
pub struct PluginManager {
    /// 生命周期管理器
    lifecycle_manager: Arc<PluginLifecycleManager>,
    /// 插件注册表
    registry: Arc<PluginRegistry>,
    /// 插件加载器
    loader: Arc<PluginLoader>,
    /// 插件 API 实现
    plugin_api: Arc<dyn PluginApi>,
    /// 插件钩子
    hooks: Arc<RwLock<HashMap<String, Vec<Arc<dyn PluginHook>>>>>,
    /// 管理器配置
    config: PluginManagerConfig,
}

/// 插件管理器配置
#[derive(Debug, Clone)]
pub struct PluginManagerConfig {
    /// 插件目录
    pub plugins_directory: PathBuf,
    /// 是否启用插件沙箱
    pub enable_sandbox: bool,
    /// 最大插件数量
    pub max_plugins: usize,
    /// 是否启用插件热重载
    pub enable_hot_reload: bool,
    /// 插件扫描间隔（秒）
    pub scan_interval_seconds: u64,
    /// 是否启用插件验证
    pub enable_plugin_verification: bool,
    /// 允许的插件权限
    pub allowed_permissions: Vec<PluginPermission>,
}

impl Default for PluginManagerConfig {
    fn default() -> Self {
        Self {
            plugins_directory: PathBuf::from("plugins"),
            enable_sandbox: true,
            max_plugins: 100,
            enable_hot_reload: true,
            scan_interval_seconds: 60,
            enable_plugin_verification: true,
            allowed_permissions: vec![
                PluginPermission::FileSystem,
                PluginPermission::Network,
                PluginPermission::UserData,
            ],
        }
    }
}

/// 插件安装请求
#[derive(Debug, Clone, Deserialize)]
pub struct InstallPluginRequest {
    /// 插件包路径或 URL
    pub source: String,
    /// 安装配置
    pub config: Option<PluginConfig>,
    /// 是否自动启动
    pub auto_start: bool,
}

/// 插件安装响应
#[derive(Debug, Clone, Serialize)]
pub struct InstallPluginResponse {
    /// 插件 ID
    pub plugin_id: String,
    /// 安装状态
    pub status: InstallationStatus,
    /// 安装时间
    pub installed_at: DateTime<Utc>,
    /// 错误信息
    pub error: Option<String>,
}

/// 安装状态
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum InstallationStatus {
    Success,
    Failed,
    Pending,
}

/// 插件列表响应
#[derive(Debug, Clone, Serialize)]
pub struct PluginListResponse {
    /// 插件列表
    pub plugins: Vec<PluginInfo>,
    /// 总数
    pub total: usize,
    /// 运行中的插件数
    pub running: usize,
    /// 错误状态的插件数
    pub error: usize,
}

/// 插件信息
#[derive(Debug, Clone, Serialize)]
pub struct PluginInfo {
    /// 插件 ID
    pub plugin_id: String,
    /// 插件元数据
    pub metadata: PluginMetadata,
    /// 当前状态
    pub status: PluginStatus,
    /// 实例信息
    pub instance_info: PluginInstanceInfo,
}

impl PluginManager {
    /// 创建新的插件管理器
    pub async fn new(
        plugin_api: Arc<dyn PluginApi>,
        config: Option<PluginManagerConfig>,
    ) -> Result<Self, AiStudioError> {
        let config = config.unwrap_or_default();
        
        // 创建生命周期管理器
        let lifecycle_manager = Arc::new(PluginLifecycleManager::new(Some(LifecycleConfig::default())));
        
        // 创建插件注册表
        let registry = Arc::new(PluginRegistry::new(Some(RegistryConfig::default())));
        
        // 创建插件加载器
        let loader = Arc::new(PluginLoader::new(
            config.plugins_directory.clone(),
            Some(LoaderConfig::default()),
        ));
        
        let manager = Self {
            lifecycle_manager,
            registry,
            loader,
            plugin_api,
            hooks: Arc::new(RwLock::new(HashMap::new())),
            config,
        };
        
        // 启动插件扫描
        if config.enable_hot_reload {
            manager.start_plugin_scanner().await;
        }
        
        Ok(manager)
    }
    
    /// 安装插件
    pub async fn install_plugin(
        &self,
        request: InstallPluginRequest,
    ) -> Result<InstallPluginResponse, AiStudioError> {
        info!("安装插件: {}", request.source);
        
        // 检查插件数量限制
        let current_count = self.registry.get_plugin_count().await;
        if current_count >= self.config.max_plugins {
            return Ok(InstallPluginResponse {
                plugin_id: String::new(),
                status: InstallationStatus::Failed,
                installed_at: Utc::now(),
                error: Some("达到最大插件数量限制".to_string()),
            });
        }
        
        // 加载插件
        let plugin_factory = match self.loader.load_plugin(&request.source).await {
            Ok(factory) => factory,
            Err(e) => {
                return Ok(InstallPluginResponse {
                    plugin_id: String::new(),
                    status: InstallationStatus::Failed,
                    installed_at: Utc::now(),
                    error: Some(e.to_string()),
                });
            }
        };
        
        let metadata = plugin_factory.metadata();
        let plugin_id = metadata.id.clone();
        
        // 验证插件
        if self.config.enable_plugin_verification {
            if let Err(e) = self.verify_plugin(&metadata).await {
                return Ok(InstallPluginResponse {
                    plugin_id,
                    status: InstallationStatus::Failed,
                    installed_at: Utc::now(),
                    error: Some(e.to_string()),
                });
            }
        }
        
        // 创建插件实例
        let plugin = match plugin_factory.create_plugin() {
            Ok(plugin) => plugin,
            Err(e) => {
                return Ok(InstallPluginResponse {
                    plugin_id,
                    status: InstallationStatus::Failed,
                    installed_at: Utc::now(),
                    error: Some(e.to_string()),
                });
            }
        };
        
        // 注册插件
        self.registry.register_plugin(metadata.clone()).await?;
        
        // 配置插件
        let config = request.config.unwrap_or_else(|| PluginConfig {
            plugin_id: plugin_id.clone(),
            parameters: HashMap::new(),
            environment: HashMap::new(),
            resource_limits: Default::default(),
            security_settings: Default::default(),
        });
        
        // 注册到生命周期管理器
        self.lifecycle_manager.register_plugin(plugin_id.clone(), plugin, config).await?;
        
        // 初始化插件
        if let Err(e) = self.lifecycle_manager.initialize_plugin(&plugin_id).await {
            warn!("插件初始化失败: {} - {}", plugin_id, e);
            return Ok(InstallPluginResponse {
                plugin_id,
                status: InstallationStatus::Failed,
                installed_at: Utc::now(),
                error: Some(e.to_string()),
            });
        }
        
        // 自动启动插件
        if request.auto_start {
            if let Err(e) = self.lifecycle_manager.start_plugin(&plugin_id).await {
                warn!("插件启动失败: {} - {}", plugin_id, e);
            }
        }
        
        info!("插件安装成功: {}", plugin_id);
        
        Ok(InstallPluginResponse {
            plugin_id,
            status: InstallationStatus::Success,
            installed_at: Utc::now(),
            error: None,
        })
    }
    
    /// 卸载插件
    pub async fn uninstall_plugin(&self, plugin_id: &str) -> Result<(), AiStudioError> {
        info!("卸载插件: {}", plugin_id);
        
        // 从生命周期管理器卸载
        self.lifecycle_manager.unload_plugin(plugin_id).await?;
        
        // 从注册表移除
        self.registry.unregister_plugin(plugin_id).await?;
        
        info!("插件卸载成功: {}", plugin_id);
        Ok(())
    }
    
    /// 启动插件
    pub async fn start_plugin(&self, plugin_id: &str) -> Result<(), AiStudioError> {
        self.lifecycle_manager.start_plugin(plugin_id).await
    }
    
    /// 停止插件
    pub async fn stop_plugin(&self, plugin_id: &str) -> Result<(), AiStudioError> {
        self.lifecycle_manager.stop_plugin(plugin_id).await
    }
    
    /// 重启插件
    pub async fn restart_plugin(&self, plugin_id: &str) -> Result<(), AiStudioError> {
        self.lifecycle_manager.restart_plugin(plugin_id).await
    }
    
    /// 调用插件
    pub async fn call_plugin(
        &self,
        plugin_id: &str,
        method: &str,
        params: HashMap<String, serde_json::Value>,
        context: PluginContext,
    ) -> Result<serde_json::Value, AiStudioError> {
        debug!("调用插件: {} - {}", plugin_id, method);
        
        // 检查插件状态
        let status = self.lifecycle_manager.get_plugin_status(plugin_id).await?;
        if status != PluginStatus::Running {
            return Err(AiStudioError::validation("插件未运行"));
        }
        
        // 执行插件调用
        // TODO: 实现实际的插件调用逻辑
        // 这里需要通过生命周期管理器获取插件实例并调用
        
        Ok(serde_json::json!({
            "plugin_id": plugin_id,
            "method": method,
            "result": "success"
        }))
    }
    
    /// 获取插件列表
    pub async fn list_plugins(&self) -> Result<PluginListResponse, AiStudioError> {
        let registered_plugins = self.registry.list_plugins().await?;
        let plugin_statuses = self.lifecycle_manager.get_all_plugin_status().await;
        
        let mut plugins = Vec::new();
        let mut running_count = 0;
        let mut error_count = 0;
        
        for metadata in registered_plugins {
            let plugin_id = &metadata.id;
            let status = plugin_statuses.get(plugin_id).cloned().unwrap_or(PluginStatus::Uninitialized);
            
            if status == PluginStatus::Running {
                running_count += 1;
            } else if status == PluginStatus::Error {
                error_count += 1;
            }
            
            let instance_info = self.lifecycle_manager.get_plugin_info(plugin_id).await
                .unwrap_or_else(|_| PluginInstanceInfo {
                    plugin_id: plugin_id.clone(),
                    status: status.clone(),
                    created_at: Utc::now(),
                    last_status_change: Utc::now(),
                    restart_count: 0,
                    error_count: 0,
                    event_count: 0,
                });
            
            plugins.push(PluginInfo {
                plugin_id: plugin_id.clone(),
                metadata,
                status,
                instance_info,
            });
        }
        
        Ok(PluginListResponse {
            total: plugins.len(),
            running: running_count,
            error: error_count,
            plugins,
        })
    }
    
    /// 获取插件信息
    pub async fn get_plugin_info(&self, plugin_id: &str) -> Result<PluginInfo, AiStudioError> {
        let metadata = self.registry.get_plugin_metadata(plugin_id).await?;
        let status = self.lifecycle_manager.get_plugin_status(plugin_id).await?;
        let instance_info = self.lifecycle_manager.get_plugin_info(plugin_id).await?;
        
        Ok(PluginInfo {
            plugin_id: plugin_id.to_string(),
            metadata,
            status,
            instance_info,
        })
    }
    
    /// 注册插件钩子
    pub async fn register_hook(&self, event_type: String, hook: Arc<dyn PluginHook>) -> Result<(), AiStudioError> {
        let mut hooks = self.hooks.write().await;
        hooks.entry(event_type).or_insert_with(Vec::new).push(hook);
        Ok(())
    }
    
    /// 触发插件钩子
    pub async fn trigger_hooks(&self, event: &PluginEvent, context: &PluginContext) -> Result<(), AiStudioError> {
        let hooks = self.hooks.read().await;
        let event_type_str = serde_json::to_string(&event.event_type).unwrap_or_default();
        
        if let Some(event_hooks) = hooks.get(&event_type_str) {
            for hook in event_hooks {
                if let Err(e) = hook.execute(event, context).await {
                    error!("插件钩子执行失败: {} - {}", hook.name(), e);
                }
            }
        }
        
        Ok(())
    }
    
    /// 验证插件
    async fn verify_plugin(&self, metadata: &PluginMetadata) -> Result<(), AiStudioError> {
        // 检查权限
        for permission in &metadata.permissions {
            if !self.config.allowed_permissions.contains(permission) {
                return Err(AiStudioError::permission_denied(&format!(
                    "插件请求的权限未被允许: {:?}", permission
                )));
            }
        }
        
        // 检查依赖
        for dependency in &metadata.dependencies {
            if !dependency.optional {
                let dep_status = self.lifecycle_manager.get_plugin_status(&dependency.plugin_id).await;
                if dep_status.is_err() || dep_status.unwrap() != PluginStatus::Running {
                    return Err(AiStudioError::validation(&format!(
                        "插件依赖未满足: {}", dependency.plugin_id
                    )));
                }
            }
        }
        
        // TODO: 验证插件签名和完整性
        
        Ok(())
    }
    
    /// 启动插件扫描器
    async fn start_plugin_scanner(&self) {
        let manager = self.clone();
        let interval = self.config.scan_interval_seconds;
        
        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(
                tokio::time::Duration::from_secs(interval)
            );
            
            loop {
                interval_timer.tick().await;
                
                if let Err(e) = manager.scan_plugins().await {
                    error!("插件扫描失败: {}", e);
                }
            }
        });
    }
    
    /// 扫描插件
    async fn scan_plugins(&self) -> Result<(), AiStudioError> {
        debug!("扫描插件目录");
        
        // TODO: 实现插件目录扫描逻辑
        // 1. 扫描插件目录
        // 2. 检测新插件
        // 3. 检测插件更新
        // 4. 自动加载新插件
        
        Ok(())
    }
    
    /// 更新插件配置
    pub async fn update_plugin_config(
        &self,
        plugin_id: &str,
        config: PluginConfig,
    ) -> Result<(), AiStudioError> {
        info!("更新插件配置: {}", plugin_id);
        
        // TODO: 实现配置更新逻辑
        // 1. 验证配置
        // 2. 更新插件配置
        // 3. 重启插件（如果需要）
        
        Ok(())
    }
    
    /// 获取插件日志
    pub async fn get_plugin_logs(
        &self,
        plugin_id: &str,
        limit: Option<usize>,
    ) -> Result<Vec<PluginEvent>, AiStudioError> {
        // TODO: 实现插件日志获取
        // 从生命周期管理器获取插件事件历史
        
        Ok(Vec::new())
    }
    
    /// 清理插件数据
    pub async fn cleanup_plugin_data(&self, plugin_id: &str) -> Result<(), AiStudioError> {
        info!("清理插件数据: {}", plugin_id);
        
        // 清理插件历史
        self.lifecycle_manager.cleanup_plugin_history(plugin_id, 50).await?;
        
        // TODO: 清理插件文件和数据
        
        Ok(())
    }
}

impl Clone for PluginManager {
    fn clone(&self) -> Self {
        Self {
            lifecycle_manager: self.lifecycle_manager.clone(),
            registry: self.registry.clone(),
            loader: self.loader.clone(),
            plugin_api: self.plugin_api.clone(),
            hooks: self.hooks.clone(),
            config: self.config.clone(),
        }
    }
}

/// 插件管理器工厂
pub struct PluginManagerFactory;

impl PluginManagerFactory {
    /// 创建插件管理器实例
    pub async fn create(
        plugin_api: Arc<dyn PluginApi>,
        config: Option<PluginManagerConfig>,
    ) -> Result<Arc<PluginManager>, AiStudioError> {
        let manager = PluginManager::new(plugin_api, config).await?;
        Ok(Arc::new(manager))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_plugin_manager_config_default() {
        let config = PluginManagerConfig::default();
        assert_eq!(config.max_plugins, 100);
        assert_eq!(config.enable_sandbox, true);
        assert_eq!(config.enable_hot_reload, true);
    }
    
    #[test]
    fn test_installation_status_serialization() {
        let status = InstallationStatus::Success;
        let json = serde_json::to_string(&status).unwrap();
        let deserialized: InstallationStatus = serde_json::from_str(&json).unwrap();
        
        assert_eq!(status, deserialized);
    }
}