// 插件注册表
// 实现插件的注册、版本管理和元数据存储

use std::sync::Arc;
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{info, warn, error, debug};
use tokio::sync::RwLock;

use crate::plugins::plugin_interface::{PluginMetadata, PluginType};
use crate::errors::AiStudioError;

/// 插件注册表
pub struct PluginRegistry {
    /// 注册的插件
    plugins: Arc<RwLock<HashMap<String, RegisteredPlugin>>>,
    /// 注册表配置
    config: RegistryConfig,
}

/// 注册表配置
#[derive(Debug, Clone)]
pub struct RegistryConfig {
    /// 是否启用版本控制
    pub enable_versioning: bool,
    /// 最大版本历史数
    pub max_version_history: usize,
    /// 是否启用插件索引
    pub enable_indexing: bool,
}

impl Default for RegistryConfig {
    fn default() -> Self {
        Self {
            enable_versioning: true,
            max_version_history: 10,
            enable_indexing: true,
        }
    }
}

/// 注册的插件
#[derive(Debug, Clone, Serialize)]
pub struct RegisteredPlugin {
    /// 插件元数据
    pub metadata: PluginMetadata,
    /// 注册时间
    pub registered_at: DateTime<Utc>,
    /// 最后更新时间
    pub updated_at: DateTime<Utc>,
    /// 版本历史
    pub version_history: Vec<PluginVersion>,
    /// 插件状态
    pub registry_status: RegistryStatus,
}

/// 插件版本
#[derive(Debug, Clone, Serialize)]
pub struct PluginVersion {
    /// 版本号
    pub version: String,
    /// 版本元数据
    pub metadata: PluginMetadata,
    /// 发布时间
    pub released_at: DateTime<Utc>,
    /// 版本状态
    pub status: VersionStatus,
}

/// 注册表状态
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RegistryStatus {
    /// 已注册
    Registered,
    /// 已弃用
    Deprecated,
    /// 已禁用
    Disabled,
    /// 已删除
    Deleted,
}

/// 版本状态
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum VersionStatus {
    /// 稳定版
    Stable,
    /// 测试版
    Beta,
    /// 开发版
    Alpha,
    /// 已弃用
    Deprecated,
}

/// 插件搜索查询
#[derive(Debug, Clone, Deserialize)]
pub struct PluginSearchQuery {
    /// 搜索关键词
    pub query: Option<String>,
    /// 插件类型过滤
    pub plugin_type: Option<PluginType>,
    /// 标签过滤
    pub tags: Option<Vec<String>>,
    /// 作者过滤
    pub author: Option<String>,
    /// 版本要求
    pub version_requirement: Option<String>,
    /// 分页大小
    pub limit: Option<usize>,
    /// 分页偏移
    pub offset: Option<usize>,
}

/// 插件搜索结果
#[derive(Debug, Clone, Serialize)]
pub struct PluginSearchResult {
    /// 匹配的插件
    pub plugins: Vec<RegisteredPlugin>,
    /// 总匹配数
    pub total: usize,
    /// 搜索时间（毫秒）
    pub search_time_ms: u64,
}

impl PluginRegistry {
    /// 创建新的插件注册表
    pub fn new(config: Option<RegistryConfig>) -> Self {
        Self {
            plugins: Arc::new(RwLock::new(HashMap::new())),
            config: config.unwrap_or_default(),
        }
    }
    
    /// 注册插件
    pub async fn register_plugin(&self, metadata: PluginMetadata) -> Result<(), AiStudioError> {
        let plugin_id = metadata.id.clone();
        info!("注册插件到注册表: {}", plugin_id);
        
        let mut plugins = self.plugins.write().await;
        
        let now = Utc::now();
        
        if let Some(existing) = plugins.get_mut(&plugin_id) {
            // 更新现有插件
            if self.config.enable_versioning {
                // 添加到版本历史
                let version = PluginVersion {
                    version: metadata.version.clone(),
                    metadata: existing.metadata.clone(),
                    released_at: existing.updated_at,
                    status: VersionStatus::Stable,
                };
                
                existing.version_history.push(version);
                
                // 限制版本历史长度
                if existing.version_history.len() > self.config.max_version_history {
                    existing.version_history.remove(0);
                }
            }
            
            existing.metadata = metadata;
            existing.updated_at = now;
            
            info!("插件更新成功: {}", plugin_id);
        } else {
            // 注册新插件
            let registered_plugin = RegisteredPlugin {
                metadata,
                registered_at: now,
                updated_at: now,
                version_history: Vec::new(),
                registry_status: RegistryStatus::Registered,
            };
            
            plugins.insert(plugin_id.clone(), registered_plugin);
            
            info!("插件注册成功: {}", plugin_id);
        }
        
        Ok(())
    }
    
    /// 注销插件
    pub async fn unregister_plugin(&self, plugin_id: &str) -> Result<(), AiStudioError> {
        info!("从注册表注销插件: {}", plugin_id);
        
        let mut plugins = self.plugins.write().await;
        
        if let Some(plugin) = plugins.get_mut(plugin_id) {
            plugin.registry_status = RegistryStatus::Deleted;
            plugin.updated_at = Utc::now();
            
            // 可选择完全移除或标记为删除
            // plugins.remove(plugin_id);
            
            info!("插件注销成功: {}", plugin_id);
            Ok(())
        } else {
            Err(AiStudioError::not_found("插件不存在"))
        }
    }
    
    /// 获取插件元数据
    pub async fn get_plugin_metadata(&self, plugin_id: &str) -> Result<PluginMetadata, AiStudioError> {
        let plugins = self.plugins.read().await;
        
        plugins.get(plugin_id)
            .filter(|p| p.registry_status != RegistryStatus::Deleted)
            .map(|p| p.metadata.clone())
            .ok_or_else(|| AiStudioError::not_found("插件不存在"))
    }
    
    /// 获取注册的插件
    pub async fn get_registered_plugin(&self, plugin_id: &str) -> Result<RegisteredPlugin, AiStudioError> {
        let plugins = self.plugins.read().await;
        
        plugins.get(plugin_id)
            .filter(|p| p.registry_status != RegistryStatus::Deleted)
            .cloned()
            .ok_or_else(|| AiStudioError::not_found("插件不存在"))
    }
    
    /// 列出所有插件
    pub async fn list_plugins(&self) -> Result<Vec<PluginMetadata>, AiStudioError> {
        let plugins = self.plugins.read().await;
        
        let result = plugins.values()
            .filter(|p| p.registry_status == RegistryStatus::Registered)
            .map(|p| p.metadata.clone())
            .collect();
        
        Ok(result)
    }
    
    /// 搜索插件
    pub async fn search_plugins(&self, query: PluginSearchQuery) -> Result<PluginSearchResult, AiStudioError> {
        let start_time = std::time::Instant::now();
        
        debug!("搜索插件: query={:?}", query.query);
        
        let plugins = self.plugins.read().await;
        
        let mut matched_plugins: Vec<RegisteredPlugin> = plugins.values()
            .filter(|p| p.registry_status == RegistryStatus::Registered)
            .filter(|p| self.matches_query(p, &query))
            .cloned()
            .collect();
        
        // 排序（按相关性或其他标准）
        matched_plugins.sort_by(|a, b| {
            // 简单的排序：按注册时间倒序
            b.registered_at.cmp(&a.registered_at)
        });
        
        let total = matched_plugins.len();
        
        // 应用分页
        let offset = query.offset.unwrap_or(0);
        let limit = query.limit.unwrap_or(50);
        
        let start = offset.min(total);
        let end = (offset + limit).min(total);
        
        matched_plugins = matched_plugins.into_iter().skip(start).take(end - start).collect();
        
        let search_time_ms = start_time.elapsed().as_millis() as u64;
        
        Ok(PluginSearchResult {
            plugins: matched_plugins,
            total,
            search_time_ms,
        })
    }
    
    /// 检查查询匹配
    fn matches_query(&self, plugin: &RegisteredPlugin, query: &PluginSearchQuery) -> bool {
        // 检查关键词匹配
        if let Some(ref search_query) = query.query {
            let search_lower = search_query.to_lowercase();
            let matches_name = plugin.metadata.name.to_lowercase().contains(&search_lower);
            let matches_description = plugin.metadata.description.to_lowercase().contains(&search_lower);
            let matches_tags = plugin.metadata.tags.iter()
                .any(|tag| tag.to_lowercase().contains(&search_lower));
            
            if !matches_name && !matches_description && !matches_tags {
                return false;
            }
        }
        
        // 检查插件类型
        if let Some(ref plugin_type) = query.plugin_type {
            if plugin.metadata.plugin_type != *plugin_type {
                return false;
            }
        }
        
        // 检查标签
        if let Some(ref tags) = query.tags {
            if !tags.iter().any(|tag| plugin.metadata.tags.contains(tag)) {
                return false;
            }
        }
        
        // 检查作者
        if let Some(ref author) = query.author {
            if plugin.metadata.author != *author {
                return false;
            }
        }
        
        // TODO: 检查版本要求
        if let Some(ref _version_req) = query.version_requirement {
            // 实现版本匹配逻辑
        }
        
        true
    }
    
    /// 获取插件数量
    pub async fn get_plugin_count(&self) -> usize {
        let plugins = self.plugins.read().await;
        plugins.values()
            .filter(|p| p.registry_status == RegistryStatus::Registered)
            .count()
    }
    
    /// 获取插件统计
    pub async fn get_plugin_statistics(&self) -> PluginStatistics {
        let plugins = self.plugins.read().await;
        
        let mut stats = PluginStatistics {
            total_plugins: 0,
            registered_plugins: 0,
            deprecated_plugins: 0,
            disabled_plugins: 0,
            plugins_by_type: HashMap::new(),
            plugins_by_author: HashMap::new(),
        };
        
        for plugin in plugins.values() {
            stats.total_plugins += 1;
            
            match plugin.registry_status {
                RegistryStatus::Registered => stats.registered_plugins += 1,
                RegistryStatus::Deprecated => stats.deprecated_plugins += 1,
                RegistryStatus::Disabled => stats.disabled_plugins += 1,
                _ => {}
            }
            
            // 按类型统计
            *stats.plugins_by_type.entry(plugin.metadata.plugin_type.clone()).or_insert(0) += 1;
            
            // 按作者统计
            *stats.plugins_by_author.entry(plugin.metadata.author.clone()).or_insert(0) += 1;
        }
        
        stats
    }
    
    /// 更新插件状态
    pub async fn update_plugin_status(
        &self,
        plugin_id: &str,
        status: RegistryStatus,
    ) -> Result<(), AiStudioError> {
        info!("更新插件注册表状态: {} -> {:?}", plugin_id, status);
        
        let mut plugins = self.plugins.write().await;
        
        if let Some(plugin) = plugins.get_mut(plugin_id) {
            plugin.registry_status = status;
            plugin.updated_at = Utc::now();
            Ok(())
        } else {
            Err(AiStudioError::not_found("插件不存在"))
        }
    }
    
    /// 获取插件版本历史
    pub async fn get_plugin_versions(&self, plugin_id: &str) -> Result<Vec<PluginVersion>, AiStudioError> {
        let plugins = self.plugins.read().await;
        
        plugins.get(plugin_id)
            .filter(|p| p.registry_status != RegistryStatus::Deleted)
            .map(|p| p.version_history.clone())
            .ok_or_else(|| AiStudioError::not_found("插件不存在"))
    }
    
    /// 检查插件依赖
    pub async fn check_dependencies(&self, plugin_id: &str) -> Result<DependencyCheckResult, AiStudioError> {
        let plugin = self.get_registered_plugin(plugin_id).await?;
        
        let mut result = DependencyCheckResult {
            plugin_id: plugin_id.to_string(),
            dependencies_satisfied: true,
            missing_dependencies: Vec::new(),
            dependency_conflicts: Vec::new(),
        };
        
        for dependency in &plugin.metadata.dependencies {
            match self.get_plugin_metadata(&dependency.plugin_id).await {
                Ok(dep_metadata) => {
                    // TODO: 检查版本兼容性
                    if !self.is_version_compatible(&dep_metadata.version, &dependency.version_requirement) {
                        result.dependencies_satisfied = false;
                        result.dependency_conflicts.push(DependencyConflict {
                            plugin_id: dependency.plugin_id.clone(),
                            required_version: dependency.version_requirement.clone(),
                            actual_version: dep_metadata.version,
                        });
                    }
                }
                Err(_) => {
                    if !dependency.optional {
                        result.dependencies_satisfied = false;
                        result.missing_dependencies.push(dependency.clone());
                    }
                }
            }
        }
        
        Ok(result)
    }
    
    /// 检查版本兼容性
    fn is_version_compatible(&self, actual_version: &str, requirement: &str) -> bool {
        // TODO: 实现语义版本比较
        // 目前简单比较
        actual_version == requirement
    }
    
    /// 清理注册表
    pub async fn cleanup_registry(&self) -> Result<usize, AiStudioError> {
        info!("清理插件注册表");
        
        let mut plugins = self.plugins.write().await;
        let initial_count = plugins.len();
        
        // 移除已删除的插件
        plugins.retain(|_, plugin| plugin.registry_status != RegistryStatus::Deleted);
        
        let removed_count = initial_count - plugins.len();
        
        if removed_count > 0 {
            info!("清理了 {} 个已删除的插件", removed_count);
        }
        
        Ok(removed_count)
    }
}

/// 插件统计
#[derive(Debug, Clone, Serialize)]
pub struct PluginStatistics {
    /// 总插件数
    pub total_plugins: usize,
    /// 已注册插件数
    pub registered_plugins: usize,
    /// 已弃用插件数
    pub deprecated_plugins: usize,
    /// 已禁用插件数
    pub disabled_plugins: usize,
    /// 按类型分组的插件数
    pub plugins_by_type: HashMap<PluginType, usize>,
    /// 按作者分组的插件数
    pub plugins_by_author: HashMap<String, usize>,
}

/// 依赖检查结果
#[derive(Debug, Clone, Serialize)]
pub struct DependencyCheckResult {
    /// 插件 ID
    pub plugin_id: String,
    /// 依赖是否满足
    pub dependencies_satisfied: bool,
    /// 缺失的依赖
    pub missing_dependencies: Vec<crate::plugins::plugin_interface::PluginDependency>,
    /// 依赖冲突
    pub dependency_conflicts: Vec<DependencyConflict>,
}

/// 依赖冲突
#[derive(Debug, Clone, Serialize)]
pub struct DependencyConflict {
    /// 插件 ID
    pub plugin_id: String,
    /// 要求的版本
    pub required_version: String,
    /// 实际版本
    pub actual_version: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::plugin_interface::PluginType;
    
    #[tokio::test]
    async fn test_plugin_registration() {
        let registry = PluginRegistry::new(None);
        
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
            permissions: Vec::new(),
            tags: vec!["test".to_string()],
            icon: None,
            created_at: Utc::now(),
        };
        
        let result = registry.register_plugin(metadata.clone()).await;
        assert!(result.is_ok());
        
        let retrieved = registry.get_plugin_metadata("test-plugin").await;
        assert!(retrieved.is_ok());
        assert_eq!(retrieved.unwrap().id, "test-plugin");
    }
    
    #[tokio::test]
    async fn test_plugin_search() {
        let registry = PluginRegistry::new(None);
        
        // 注册测试插件
        let metadata = PluginMetadata {
            id: "search-test".to_string(),
            name: "Search Test Plugin".to_string(),
            version: "1.0.0".to_string(),
            description: "A plugin for testing search functionality".to_string(),
            author: "Test Author".to_string(),
            license: "MIT".to_string(),
            homepage: None,
            repository: None,
            plugin_type: PluginType::Tool,
            api_version: "1.0".to_string(),
            min_system_version: "1.0.0".to_string(),
            dependencies: Vec::new(),
            permissions: Vec::new(),
            tags: vec!["search".to_string(), "test".to_string()],
            icon: None,
            created_at: Utc::now(),
        };
        
        registry.register_plugin(metadata).await.unwrap();
        
        // 测试搜索
        let query = PluginSearchQuery {
            query: Some("search".to_string()),
            plugin_type: None,
            tags: None,
            author: None,
            version_requirement: None,
            limit: None,
            offset: None,
        };
        
        let result = registry.search_plugins(query).await.unwrap();
        assert_eq!(result.total, 1);
        assert_eq!(result.plugins[0].metadata.id, "search-test");
    }
}