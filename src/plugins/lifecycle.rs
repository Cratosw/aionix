// 插件生命周期管理
// 实现插件的生命周期状态管理和转换

use std::sync::Arc;
use std::collections::HashMap;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{info, warn, error, debug};
use tokio::sync::RwLock;

use crate::plugins::plugin_interface::{
    Plugin, PluginStatus, PluginConfig, PluginEvent, PluginEventType, PluginContext, PluginError, PluginErrorType
};
use crate::errors::AiStudioError;

/// 插件生命周期管理器
pub struct PluginLifecycleManager {
    /// 插件实例
    plugins: Arc<RwLock<HashMap<String, PluginInstance>>>,
    /// 生命周期配置
    config: LifecycleConfig,
}

/// 生命周期配置
#[derive(Debug, Clone)]
pub struct LifecycleConfig {
    /// 初始化超时时间（秒）
    pub initialization_timeout_seconds: u64,
    /// 启动超时时间（秒）
    pub startup_timeout_seconds: u64,
    /// 停止超时时间（秒）
    pub shutdown_timeout_seconds: u64,
    /// 健康检查间隔（秒）
    pub health_check_interval_seconds: u64,
    /// 是否启用自动重启
    pub enable_auto_restart: bool,
    /// 最大重启次数
    pub max_restart_attempts: u32,
}

impl Default for LifecycleConfig {
    fn default() -> Self {
        Self {
            initialization_timeout_seconds: 30,
            startup_timeout_seconds: 60,
            shutdown_timeout_seconds: 30,
            health_check_interval_seconds: 60,
            enable_auto_restart: true,
            max_restart_attempts: 3,
        }
    }
}

/// 插件实例
#[derive(Debug, Clone)]
pub struct PluginInstance {
    /// 插件 ID
    pub plugin_id: String,
    /// 插件实现
    pub plugin: Box<dyn Plugin>,
    /// 当前状态
    pub status: PluginStatus,
    /// 配置
    pub config: PluginConfig,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 最后状态变更时间
    pub last_status_change: DateTime<Utc>,
    /// 重启次数
    pub restart_count: u32,
    /// 错误历史
    pub error_history: Vec<PluginError>,
    /// 生命周期事件历史
    pub event_history: Vec<PluginEvent>,
}

/// 生命周期状态转换
#[derive(Debug, Clone, Serialize)]
pub struct StatusTransition {
    /// 插件 ID
    pub plugin_id: String,
    /// 从状态
    pub from_status: PluginStatus,
    /// 到状态
    pub to_status: PluginStatus,
    /// 转换时间
    pub timestamp: DateTime<Utc>,
    /// 转换原因
    pub reason: String,
    /// 是否成功
    pub success: bool,
    /// 错误信息
    pub error: Option<String>,
}

impl PluginLifecycleManager {
    /// 创建新的生命周期管理器
    pub fn new(config: Option<LifecycleConfig>) -> Self {
        Self {
            plugins: Arc::new(RwLock::new(HashMap::new())),
            config: config.unwrap_or_default(),
        }
    }

    /// 注册插件
    pub async fn register_plugin(
        &self,
        plugin_id: String,
        plugin: Box<dyn Plugin>,
        config: PluginConfig,
    ) -> Result<(), AiStudioError> {
        info!("注册插件: {}", plugin_id);

        let instance = PluginInstance {
            plugin_id: plugin_id.clone(),
            plugin,
            status: PluginStatus::Uninitialized,
            config,
            created_at: Utc::now(),
            last_status_change: Utc::now(),
            restart_count: 0,
            error_history: Vec::new(),
            event_history: Vec::new(),
        };

        let mut plugins = self.plugins.write().await;
        plugins.insert(plugin_id.clone(), instance);

        // 发送注册事件
        self.emit_event(&plugin_id, PluginEventType::Loaded, serde_json::Value::Null).await;

        Ok(())
    }

    /// 初始化插件
    pub async fn initialize_plugin(&self, plugin_id: &str) -> Result<(), AiStudioError> {
        info!("初始化插件: {}", plugin_id);

        self.transition_status(plugin_id, PluginStatus::Initializing, "开始初始化").await?;

        let result = {
            let mut plugins = self.plugins.write().await;
            let instance = plugins.get_mut(plugin_id)
                .ok_or_else(|| AiStudioError::not_found("插件不存在"))?;

            // 执行初始化
            tokio::time::timeout(
                tokio::time::Duration::from_secs(self.config.initialization_timeout_seconds),
                instance.plugin.initialize(instance.config.clone())
            ).await
        };

        match result {
            Ok(Ok(_)) => {
                self.transition_status(plugin_id, PluginStatus::Initialized, "初始化成功").await?;
                self.emit_event(plugin_id, PluginEventType::Initialized, serde_json::Value::Null).await;
                Ok(())
            }
            Ok(Err(e)) => {
                self.handle_plugin_error(plugin_id, PluginErrorType::InitializationError, &e.to_string()).await;
                Err(e)
            }
            Err(_) => {
                let error = AiStudioError::timeout("插件初始化超时");
                self.handle_plugin_error(plugin_id, PluginErrorType::InitializationError, &error.to_string()).await;
                Err(error)
            }
        }
    }

    /// 启动插件
    pub async fn start_plugin(&self, plugin_id: &str) -> Result<(), AiStudioError> {
        info!("启动插件: {}", plugin_id);

        // 检查当前状态
        {
            let plugins = self.plugins.read().await;
            let instance = plugins.get(plugin_id)
                .ok_or_else(|| AiStudioError::not_found("插件不存在"))?;

            if instance.status != PluginStatus::Initialized {
                return Err(AiStudioError::validation("插件未初始化", "插件必须先初始化才能启动"));
            }
        }

        self.transition_status(plugin_id, PluginStatus::Starting, "开始启动").await?;

        let result = {
            let mut plugins = self.plugins.write().await;
            let instance = plugins.get_mut(plugin_id)
                .ok_or_else(|| AiStudioError::not_found("插件不存在"))?;

            // 执行启动
            tokio::time::timeout(
                tokio::time::Duration::from_secs(self.config.startup_timeout_seconds),
                instance.plugin.start()
            ).await
        };

        match result {
            Ok(Ok(_)) => {
                self.transition_status(plugin_id, PluginStatus::Running, "启动成功").await?;
                self.emit_event(plugin_id, PluginEventType::Started, serde_json::Value::Null).await;

                // 启动健康检查
                if self.config.health_check_interval_seconds > 0 {
                    self.start_health_check(plugin_id).await;
                }

                Ok(())
            }
            Ok(Err(e)) => {
                self.handle_plugin_error(plugin_id, PluginErrorType::ExecutionError, &e.to_string()).await;
                Err(e)
            }
            Err(_) => {
                let error = AiStudioError::timeout("插件启动超时");
                self.handle_plugin_error(plugin_id, PluginErrorType::ExecutionError, &error.to_string()).await;
                Err(error)
            }
        }
    }

    /// 停止插件
    pub async fn stop_plugin(&self, plugin_id: &str) -> Result<(), AiStudioError> {
        info!("停止插件: {}", plugin_id);

        self.transition_status(plugin_id, PluginStatus::Stopping, "开始停止").await?;

        let result = {
            let mut plugins = self.plugins.write().await;
            let instance = plugins.get_mut(plugin_id)
                .ok_or_else(|| AiStudioError::not_found("插件不存在"))?;

            // 执行停止
            tokio::time::timeout(
                tokio::time::Duration::from_secs(self.config.shutdown_timeout_seconds),
                instance.plugin.stop()
            ).await
        };

        match result {
            Ok(Ok(_)) => {
                self.transition_status(plugin_id, PluginStatus::Stopped, "停止成功").await?;
                self.emit_event(plugin_id, PluginEventType::Stopped, serde_json::Value::Null).await;
                Ok(())
            }
            Ok(Err(e)) => {
                self.handle_plugin_error(plugin_id, PluginErrorType::ExecutionError, &e.to_string()).await;
                Err(e)
            }
            Err(_) => {
                let error = AiStudioError::timeout("插件停止超时");
                self.handle_plugin_error(plugin_id, PluginErrorType::ExecutionError, &error.to_string()).await;
                Err(error)
            }
        }
    }

    /// 卸载插件
    pub async fn unload_plugin(&self, plugin_id: &str) -> Result<(), AiStudioError> {
        info!("卸载插件: {}", plugin_id);

        // 先停止插件
        if let Ok(status) = self.get_plugin_status(plugin_id).await {
            if status == PluginStatus::Running {
                self.stop_plugin(plugin_id).await?;
            }
        }

        self.transition_status(plugin_id, PluginStatus::Unloading, "开始卸载").await?;

        let result = {
            let mut plugins = self.plugins.write().await;
            let instance = plugins.get_mut(plugin_id)
                .ok_or_else(|| AiStudioError::not_found("插件不存在"))?;

            // 执行卸载
            instance.plugin.shutdown().await
        };

        match result {
            Ok(_) => {
                // 从管理器中移除插件
                let mut plugins = self.plugins.write().await;
                plugins.remove(plugin_id);

                self.emit_event(plugin_id, PluginEventType::Unloaded, serde_json::Value::Null).await;
                Ok(())
            }
            Err(e) => {
                self.handle_plugin_error(plugin_id, PluginErrorType::ExecutionError, &e.to_string()).await;
                Err(e)
            }
        }
    }

    /// 重启插件
    pub async fn restart_plugin(&self, plugin_id: &str) -> Result<(), AiStudioError> {
        info!("重启插件: {}", plugin_id);

        // 增加重启计数
        {
            let mut plugins = self.plugins.write().await;
            if let Some(instance) = plugins.get_mut(plugin_id) {
                instance.restart_count += 1;

                // 检查重启次数限制
                if instance.restart_count > self.config.max_restart_attempts {
                    return Err(AiStudioError::rate_limit(None));
                }
            }
        }

        // 停止插件
        if let Err(e) = self.stop_plugin(plugin_id).await {
            warn!("停止插件失败: {} - {}", plugin_id, e);
        }

        // 重新启动
        self.start_plugin(plugin_id).await
    }

    /// 获取插件状态
    pub async fn get_plugin_status(&self, plugin_id: &str) -> Result<PluginStatus, AiStudioError> {
        let plugins = self.plugins.read().await;
        let instance = plugins.get(plugin_id)
            .ok_or_else(|| AiStudioError::not_found("插件不存在"))?;

        Ok(instance.status.clone())
    }

    /// 获取所有插件状态
    pub async fn get_all_plugin_status(&self) -> HashMap<String, PluginStatus> {
        let plugins = self.plugins.read().await;
        plugins.iter()
            .map(|(id, instance)| (id.clone(), instance.status.clone()))
            .collect()
    }

    /// 状态转换
    async fn transition_status(
        &self,
        plugin_id: &str,
        new_status: PluginStatus,
        reason: &str,
    ) -> Result<(), AiStudioError> {
        let mut plugins = self.plugins.write().await;
        let instance = plugins.get_mut(plugin_id)
            .ok_or_else(|| AiStudioError::not_found("插件不存在"))?;

        let old_status = instance.status.clone();
        instance.status = new_status.clone();
        instance.last_status_change = Utc::now();

        debug!("插件状态转换: {} - {:?} -> {:?} ({})",
               plugin_id, old_status, new_status, reason);

        // 记录状态转换事件
        let transition = StatusTransition {
            plugin_id: plugin_id.to_string(),
            from_status: old_status,
            to_status: new_status,
            timestamp: Utc::now(),
            reason: reason.to_string(),
            success: true,
            error: None,
        };

        instance.event_history.push(PluginEvent {
            event_id: Uuid::new_v4(),
            plugin_id: plugin_id.to_string(),
            event_type: PluginEventType::ConfigUpdated, // 使用配置更新作为状态变更事件
            data: serde_json::to_value(transition).unwrap_or_default(),
            timestamp: Utc::now(),
        });

        Ok(())
    }

    /// 处理插件错误
    async fn handle_plugin_error(
        &self,
        plugin_id: &str,
        error_type: PluginErrorType,
        message: &str,
    ) {
        error!("插件错误: {} - {} - {}", plugin_id,
               serde_json::to_string(&error_type).unwrap_or_default(), message);

        let plugin_error = PluginError {
            error_type,
            message: message.to_string(),
            details: None,
            plugin_id: plugin_id.to_string(),
            timestamp: Utc::now(),
        };

        // 记录错误
        {
            let mut plugins = self.plugins.write().await;
            if let Some(instance) = plugins.get_mut(plugin_id) {
                instance.status = PluginStatus::Error;
                instance.error_history.push(plugin_error.clone());
                instance.last_status_change = Utc::now();
            }
        }

        // 发送错误事件
        self.emit_event(plugin_id, PluginEventType::Error,
                       serde_json::to_value(plugin_error).unwrap_or_default()).await;

        // 如果启用自动重启，尝试重启插件
        if self.config.enable_auto_restart {
            if let Err(e) = self.restart_plugin(plugin_id).await {
                error!("自动重启插件失败: {} - {}", plugin_id, e);
            }
        }
    }

    /// 发送事件
    async fn emit_event(
        &self,
        plugin_id: &str,
        event_type: PluginEventType,
        data: serde_json::Value,
    ) {
        let event = PluginEvent {
            event_id: Uuid::new_v4(),
            plugin_id: plugin_id.to_string(),
            event_type,
            data,
            timestamp: Utc::now(),
        };

        // 记录事件到插件实例
        {
            let mut plugins = self.plugins.write().await;
            if let Some(instance) = plugins.get_mut(plugin_id) {
                instance.event_history.push(event.clone());

                // 限制事件历史长度
                if instance.event_history.len() > 100 {
                    instance.event_history.remove(0);
                }
            }
        }

        debug!("插件事件: {} - {:?}", plugin_id, event.event_type);
    }

    /// 启动健康检查
    async fn start_health_check(&self, plugin_id: &str) {
        let plugin_id = plugin_id.to_string();
        let manager = self.clone();
        let interval = self.config.health_check_interval_seconds;

        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(
                tokio::time::Duration::from_secs(interval)
            );

            loop {
                interval_timer.tick().await;

                // 检查插件是否仍在运行
                if let Ok(status) = manager.get_plugin_status(&plugin_id).await {
                    if status != PluginStatus::Running {
                        break;
                    }
                } else {
                    break;
                }

                // 执行健康检查
                if let Err(e) = manager.perform_health_check(&plugin_id).await {
                    error!("插件健康检查失败: {} - {}", plugin_id, e);
                }
            }
        });
    }

    /// 执行健康检查
    async fn perform_health_check(&self, plugin_id: &str) -> Result<(), AiStudioError> {
        let health_result = {
            let plugins = self.plugins.read().await;
            let instance = plugins.get(plugin_id)
                .ok_or_else(|| AiStudioError::not_found("插件不存在"))?;

            instance.plugin.health_check().await
        };

        match health_result {
            Ok(health) => {
                if !health.healthy {
                    warn!("插件健康检查失败: {} - {}", plugin_id, health.message);
                    self.handle_plugin_error(plugin_id, PluginErrorType::ExecutionError, &health.message).await;
                }

                self.emit_event(plugin_id, PluginEventType::HealthCheck,
                               serde_json::to_value(health).unwrap_or_default()).await;

                Ok(())
            }
            Err(e) => {
                self.handle_plugin_error(plugin_id, PluginErrorType::CommunicationError, &e.to_string()).await;
                Err(e)
            }
        }
    }

    /// 获取插件实例信息
    pub async fn get_plugin_info(&self, plugin_id: &str) -> Result<PluginInstanceInfo, AiStudioError> {
        let plugins = self.plugins.read().await;
        let instance = plugins.get(plugin_id)
            .ok_or_else(|| AiStudioError::not_found("插件不存在"))?;

        Ok(PluginInstanceInfo {
            plugin_id: instance.plugin_id.clone(),
            status: instance.status.clone(),
            created_at: instance.created_at,
            last_status_change: instance.last_status_change,
            restart_count: instance.restart_count,
            error_count: instance.error_history.len(),
            event_count: instance.event_history.len(),
        })
    }

    /// 清理插件历史
    pub async fn cleanup_plugin_history(&self, plugin_id: &str, max_history_size: usize) -> Result<(), AiStudioError> {
        let mut plugins = self.plugins.write().await;
        let instance = plugins.get_mut(plugin_id)
            .ok_or_else(|| AiStudioError::not_found("插件不存在"))?;

        // 清理错误历史
        if instance.error_history.len() > max_history_size {
            instance.error_history.drain(0..instance.error_history.len() - max_history_size);
        }

        // 清理事件历史
        if instance.event_history.len() > max_history_size {
            instance.event_history.drain(0..instance.event_history.len() - max_history_size);
        }

        Ok(())
    }
}

impl Clone for PluginLifecycleManager {
    fn clone(&self) -> Self {
        Self {
            plugins: self.plugins.clone(),
            config: self.config.clone(),
        }
    }
}

/// 插件实例信息
#[derive(Debug, Clone, Serialize)]
pub struct PluginInstanceInfo {
    /// 插件 ID
    pub plugin_id: String,
    /// 当前状态
    pub status: PluginStatus,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 最后状态变更时间
    pub last_status_change: DateTime<Utc>,
    /// 重启次数
    pub restart_count: u32,
    /// 错误数量
    pub error_count: usize,
    /// 事件数量
    pub event_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lifecycle_config_default() {
        let config = LifecycleConfig::default();
        assert_eq!(config.initialization_timeout_seconds, 30);
        assert_eq!(config.enable_auto_restart, true);
        assert_eq!(config.max_restart_attempts, 3);
    }

    #[test]
    fn test_status_transition_serialization() {
        let transition = StatusTransition {
            plugin_id: "test-plugin".to_string(),
            from_status: PluginStatus::Initialized,
            to_status: PluginStatus::Running,
            timestamp: Utc::now(),
            reason: "启动成功".to_string(),
            success: true,
            error: None,
        };

        let json = serde_json::to_string(&transition).unwrap();
        let deserialized: StatusTransition = serde_json::from_str(&json).unwrap();

        assert_eq!(transition.plugin_id, deserialized.plugin_id);
        assert_eq!(transition.success, deserialized.success);
    }
}
