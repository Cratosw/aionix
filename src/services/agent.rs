// Agent 服务
// 管理 Agent 运行时和相关服务

use std::sync::Arc;
use tracing::{info, error, debug};
use sea_orm::DatabaseConnection;

use crate::ai::{
    agent_runtime::{AgentRuntime, AgentRuntimeConfig, AgentRuntimeFactory},
    rig_client::RigClient,
    tool_manager::{ToolManager, ToolManagerFactory},
    tool_loader::{ToolLoader, ToolLoaderFactory},
    tools::{ToolFactory, Tool},
};
use crate::errors::AiStudioError;
use crate::config::AppConfig;

/// Agent 服务管理器
pub struct AgentService {
    /// Agent 运行时
    runtime: Arc<AgentRuntime>,
    /// 工具管理器
    tool_manager: Arc<ToolManager>,
    /// 工具加载器
    tool_loader: Arc<ToolLoader>,
}

impl AgentService {
    /// 创建新的 Agent 服务
    pub async fn new(
        db: Arc<DatabaseConnection>,
        rig_client: Arc<RigClient>,
        config: &AppConfig,
    ) -> Result<Self, AiStudioError> {
        info!("初始化 Agent 服务");
        
        // 创建工具管理器
        let tool_manager = ToolManagerFactory::create(None);
        
        // 创建工具加载器
        let tools_directory = std::path::PathBuf::from("tools");
        let tool_loader = ToolLoaderFactory::create(
            tool_manager.clone(),
            tools_directory,
            None,
        );
        
        // 加载所有工具
        match tool_loader.load_all_tools().await {
            Ok(result) => {
                info!("工具加载完成: 成功={}, 失败={}, 跳过={}", 
                      result.loaded_count, result.failed_count, result.skipped_count);
            }
            Err(e) => {
                error!("工具加载失败: {}", e);
            }
        }
        
        // 创建 Agent 运行时配置
        let runtime_config = AgentRuntimeConfig {
            max_reasoning_steps: 50,
            reasoning_timeout_seconds: 300,
            max_concurrent_agents: 100,
            memory_config: crate::ai::agent_runtime::MemoryConfig::default(),
            tool_call_timeout_seconds: 30,
        };
        
        // 创建 Agent 运行时
        let runtime = AgentRuntimeFactory::create(
            db.clone(),
            rig_client.clone(),
            Some(runtime_config),
        );
        
        info!("Agent 服务初始化完成");
        
        Ok(Self { 
            runtime,
            tool_manager,
            tool_loader,
        })
    }
    
    /// 获取 Agent 运行时
    pub fn get_runtime(&self) -> Arc<AgentRuntime> {
        self.runtime.clone()
    }
    
    /// 获取工具管理器
    pub fn get_tool_manager(&self) -> Arc<ToolManager> {
        self.tool_manager.clone()
    }
    
    /// 获取工具加载器
    pub fn get_tool_loader(&self) -> Arc<ToolLoader> {
        self.tool_loader.clone()
    }
    
    /// 注册自定义工具
    pub async fn register_tool(&self, tool: Arc<dyn Tool + Send + Sync>) -> Result<(), AiStudioError> {
        self.tool_manager.register_tool(tool, None).await
    }
    
    /// 清理非活跃 Agent
    pub async fn cleanup_inactive_agents(&self) -> Result<u32, AiStudioError> {
        debug!("清理非活跃 Agent");
        self.runtime.cleanup_inactive_agents().await
    }
    
    /// 获取服务统计信息
    pub async fn get_service_stats(&self) -> Result<AgentServiceStats, AiStudioError> {
        // TODO: 实现实际的统计信息收集
        Ok(AgentServiceStats {
            total_agents: 0,
            active_agents: 0,
            total_tasks_executed: 0,
            successful_tasks: 0,
            failed_tasks: 0,
            avg_task_duration_ms: 0.0,
            registered_tools: 4, // 基础工具数量
        })
    }
}

/// Agent 服务统计信息
#[derive(Debug, Clone, serde::Serialize)]
pub struct AgentServiceStats {
    /// 总 Agent 数
    pub total_agents: u32,
    /// 活跃 Agent 数
    pub active_agents: u32,
    /// 总执行任务数
    pub total_tasks_executed: u64,
    /// 成功任务数
    pub successful_tasks: u64,
    /// 失败任务数
    pub failed_tasks: u64,
    /// 平均任务执行时间（毫秒）
    pub avg_task_duration_ms: f32,
    /// 注册的工具数
    pub registered_tools: u32,
}

/// Agent 服务工厂
pub struct AgentServiceFactory;

impl AgentServiceFactory {
    /// 创建 Agent 服务实例
    pub async fn create(
        db: Arc<DatabaseConnection>,
        rig_client: Arc<RigClient>,
        config: &AppConfig,
    ) -> Result<Arc<AgentService>, AiStudioError> {
        let service = AgentService::new(db, rig_client, config).await?;
        Ok(Arc::new(service))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_agent_service_stats_serialization() {
        let stats = AgentServiceStats {
            total_agents: 10,
            active_agents: 5,
            total_tasks_executed: 100,
            successful_tasks: 95,
            failed_tasks: 5,
            avg_task_duration_ms: 1500.0,
            registered_tools: 4,
        };
        
        let json = serde_json::to_string(&stats).unwrap();
        let deserialized: AgentServiceStats = serde_json::from_str(&json).unwrap();
        
        assert_eq!(stats.total_agents, deserialized.total_agents);
        assert_eq!(stats.active_agents, deserialized.active_agents);
    }
}