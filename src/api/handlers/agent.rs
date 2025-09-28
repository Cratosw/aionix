// Agent 管理 API 处理器

use std::sync::Arc;
use actix_web::{web, HttpResponse, Result as ActixResult};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use tracing::{info, error, debug};
use utoipa::ToSchema;

use crate::ai::agent_runtime::{
    AgentRuntime, AgentConfig, AgentTask, TaskPriority, TaskStatus, AgentState, ReasoningStrategy
};
use crate::api::middleware::tenant::TenantInfo;
use crate::errors::AiStudioError;

/// Agent 创建请求
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateAgentRequest {
    /// Agent 名称
    pub name: String,
    /// Agent 描述
    pub description: String,
    /// 系统提示词
    pub system_prompt: String,
    /// 可用工具列表
    pub available_tools: Vec<String>,
    /// 推理策略
    pub reasoning_strategy: ReasoningStrategy,
    /// 温度参数
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    /// 最大令牌数
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
}

fn default_temperature() -> f32 { 0.7 }
fn default_max_tokens() -> u32 { 2000 }

/// Agent 创建响应
#[derive(Debug, Serialize, ToSchema)]
pub struct CreateAgentResponse {
    /// Agent ID
    pub agent_id: Uuid,
    /// 创建时间
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// 状态
    pub status: String,
}

/// Agent 任务执行请求
#[derive(Debug, Deserialize, ToSchema)]
pub struct ExecuteTaskRequest {
    /// 任务描述
    pub description: String,
    /// 任务目标
    pub objective: String,
    /// 任务参数
    #[serde(default)]
    pub parameters: std::collections::HashMap<String, serde_json::Value>,
    /// 任务优先级
    #[serde(default)]
    pub priority: TaskPriority,
    /// 截止时间
    pub deadline: Option<chrono::DateTime<chrono::Utc>>,
}

/// Agent 任务执行响应
#[derive(Debug, Serialize, ToSchema)]
pub struct ExecuteTaskResponse {
    /// 任务 ID
    pub task_id: Uuid,
    /// 执行结果
    pub result: serde_json::Value,
    /// 执行状态
    pub status: TaskStatus,
    /// 执行时间（毫秒）
    pub execution_time_ms: u64,
}

/// Agent 状态响应
#[derive(Debug, Serialize, ToSchema)]
pub struct AgentStatusResponse {
    /// Agent ID
    pub agent_id: Uuid,
    /// Agent 状态
    pub state: AgentState,
    /// 当前任务
    pub current_task: Option<AgentTaskInfo>,
    /// 最后活跃时间
    pub last_active_at: chrono::DateTime<chrono::Utc>,
    /// 执行统计
    pub execution_stats: ExecutionStats,
}

/// Agent 任务信息
#[derive(Debug, Serialize, ToSchema)]
pub struct AgentTaskInfo {
    /// 任务 ID
    pub task_id: Uuid,
    /// 任务描述
    pub description: String,
    /// 任务状态
    pub status: TaskStatus,
    /// 创建时间
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// 执行统计
#[derive(Debug, Serialize, ToSchema)]
pub struct ExecutionStats {
    /// 总任务数
    pub total_tasks: u32,
    /// 成功任务数
    pub successful_tasks: u32,
    /// 失败任务数
    pub failed_tasks: u32,
    /// 平均执行时间（毫秒）
    pub avg_execution_time_ms: f32,
}

/// Agent 列表响应
#[derive(Debug, Serialize, ToSchema)]
pub struct ListAgentsResponse {
    /// Agent 列表
    pub agents: Vec<AgentInfo>,
    /// 总数
    pub total: u32,
    /// 活跃数
    pub active: u32,
}

/// Agent 信息
#[derive(Debug, Serialize, ToSchema)]
pub struct AgentInfo {
    /// Agent ID
    pub agent_id: Uuid,
    /// Agent 名称
    pub name: String,
    /// Agent 描述
    pub description: String,
    /// Agent 状态
    pub state: AgentState,
    /// 创建时间
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// 最后活跃时间
    pub last_active_at: chrono::DateTime<chrono::Utc>,
}

/// 创建 Agent
#[utoipa::path(
    post,
    path = "/api/v1/agents",
    request_body = CreateAgentRequest,
    responses(
        (status = 201, description = "Agent 创建成功", body = CreateAgentResponse),
        (status = 400, description = "请求参数错误"),
        (status = 401, description = "未授权"),
        (status = 500, description = "服务器内部错误")
    ),
    tag = "agents"
)]
pub async fn create_agent(
    agent_runtime: web::Data<Arc<AgentRuntime>>,
    tenant_info: web::ReqData<TenantInfo>,
    request: web::Json<CreateAgentRequest>,
) -> ActixResult<HttpResponse> {
    debug!("创建 Agent: tenant_id={}", tenant_info.tenant_id);
    
    let config = AgentConfig {
        name: request.name.clone(),
        description: request.description.clone(),
        system_prompt: request.system_prompt.clone(),
        available_tools: request.available_tools.clone(),
        reasoning_strategy: request.reasoning_strategy.clone(),
        temperature: request.temperature,
        max_tokens: request.max_tokens,
        tenant_id: tenant_info.tenant_id,
        created_by: tenant_info.user_id.unwrap_or_else(Uuid::new_v4),
    };
    
    match agent_runtime.create_agent(config).await {
        Ok(agent_id) => {
            info!("Agent 创建成功: agent_id={}, tenant_id={}", agent_id, tenant_info.tenant_id);
            
            let response = CreateAgentResponse {
                agent_id,
                created_at: chrono::Utc::now(),
                status: "created".to_string(),
            };
            
            Ok(HttpResponse::Created().json(response))
        }
        Err(e) => {
            error!("创建 Agent 失败: {}", e);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "创建 Agent 失败",
                "message": e.to_string()
            })))
        }
    }
}

/// 执行 Agent 任务
#[utoipa::path(
    post,
    path = "/api/v1/agents/{agent_id}/execute",
    request_body = ExecuteTaskRequest,
    responses(
        (status = 200, description = "任务执行成功", body = ExecuteTaskResponse),
        (status = 400, description = "请求参数错误"),
        (status = 404, description = "Agent 不存在"),
        (status = 500, description = "服务器内部错误")
    ),
    params(
        ("agent_id" = Uuid, Path, description = "Agent ID")
    ),
    tag = "agents"
)]
pub async fn execute_task(
    agent_runtime: web::Data<Arc<AgentRuntime>>,
    tenant_info: web::ReqData<TenantInfo>,
    path: web::Path<Uuid>,
    request: web::Json<ExecuteTaskRequest>,
) -> ActixResult<HttpResponse> {
    let agent_id = path.into_inner();
    debug!("执行 Agent 任务: agent_id={}, tenant_id={}", agent_id, tenant_info.tenant_id);
    
    let task = AgentTask {
        task_id: Uuid::new_v4(),
        description: request.description.clone(),
        objective: request.objective.clone(),
        parameters: request.parameters.clone(),
        priority: request.priority.clone(),
        status: TaskStatus::Pending,
        created_at: chrono::Utc::now(),
        deadline: request.deadline,
    };
    
    let start_time = std::time::Instant::now();
    
    match agent_runtime.execute_task(agent_id, task.clone()).await {
        Ok(result) => {
            let execution_time = start_time.elapsed().as_millis() as u64;
            
            info!("Agent 任务执行成功: agent_id={}, task_id={}, 执行时间={}ms", 
                  agent_id, task.task_id, execution_time);
            
            let response = ExecuteTaskResponse {
                task_id: task.task_id,
                result,
                status: TaskStatus::Completed,
                execution_time_ms: execution_time,
            };
            
            Ok(HttpResponse::Ok().json(response))
        }
        Err(e) => {
            error!("Agent 任务执行失败: agent_id={}, error={}", agent_id, e);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "任务执行失败",
                "message": e.to_string()
            })))
        }
    }
}

/// 获取 Agent 状态
#[utoipa::path(
    get,
    path = "/api/v1/agents/{agent_id}/status",
    responses(
        (status = 200, description = "获取状态成功", body = AgentStatusResponse),
        (status = 404, description = "Agent 不存在"),
        (status = 500, description = "服务器内部错误")
    ),
    params(
        ("agent_id" = Uuid, Path, description = "Agent ID")
    ),
    tag = "agents"
)]
pub async fn get_agent_status(
    agent_runtime: web::Data<Arc<AgentRuntime>>,
    tenant_info: web::ReqData<TenantInfo>,
    path: web::Path<Uuid>,
) -> ActixResult<HttpResponse> {
    let agent_id = path.into_inner();
    debug!("获取 Agent 状态: agent_id={}, tenant_id={}", agent_id, tenant_info.tenant_id);
    
    match agent_runtime.get_agent_state(agent_id).await {
        Ok(state) => {
            // 这里应该从数据库获取更详细的信息
            // 目前返回基本状态信息
            let response = AgentStatusResponse {
                agent_id,
                state,
                current_task: None, // TODO: 从实际状态获取
                last_active_at: chrono::Utc::now(),
                execution_stats: ExecutionStats {
                    total_tasks: 0,
                    successful_tasks: 0,
                    failed_tasks: 0,
                    avg_execution_time_ms: 0.0,
                },
            };
            
            Ok(HttpResponse::Ok().json(response))
        }
        Err(e) => {
            error!("获取 Agent 状态失败: agent_id={}, error={}", agent_id, e);
            Ok(HttpResponse::NotFound().json(serde_json::json!({
                "error": "Agent 不存在",
                "message": e.to_string()
            })))
        }
    }
}

/// 停止 Agent
#[utoipa::path(
    post,
    path = "/api/v1/agents/{agent_id}/stop",
    responses(
        (status = 200, description = "Agent 停止成功"),
        (status = 404, description = "Agent 不存在"),
        (status = 500, description = "服务器内部错误")
    ),
    params(
        ("agent_id" = Uuid, Path, description = "Agent ID")
    ),
    tag = "agents"
)]
pub async fn stop_agent(
    agent_runtime: web::Data<Arc<AgentRuntime>>,
    tenant_info: web::ReqData<TenantInfo>,
    path: web::Path<Uuid>,
) -> ActixResult<HttpResponse> {
    let agent_id = path.into_inner();
    debug!("停止 Agent: agent_id={}, tenant_id={}", agent_id, tenant_info.tenant_id);
    
    match agent_runtime.stop_agent(agent_id).await {
        Ok(_) => {
            info!("Agent 停止成功: agent_id={}", agent_id);
            Ok(HttpResponse::Ok().json(serde_json::json!({
                "message": "Agent 停止成功",
                "agent_id": agent_id
            })))
        }
        Err(e) => {
            error!("停止 Agent 失败: agent_id={}, error={}", agent_id, e);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "停止 Agent 失败",
                "message": e.to_string()
            })))
        }
    }
}

/// 列出 Agent
#[utoipa::path(
    get,
    path = "/api/v1/agents",
    responses(
        (status = 200, description = "获取 Agent 列表成功", body = ListAgentsResponse),
        (status = 500, description = "服务器内部错误")
    ),
    params(
        ("limit" = Option<u32>, Query, description = "返回数量限制"),
        ("offset" = Option<u32>, Query, description = "偏移量")
    ),
    tag = "agents"
)]
pub async fn list_agents(
    agent_runtime: web::Data<Arc<AgentRuntime>>,
    tenant_info: web::ReqData<TenantInfo>,
    query: web::Query<ListQuery>,
) -> ActixResult<HttpResponse> {
    debug!("列出 Agent: tenant_id={}", tenant_info.tenant_id);
    
    // TODO: 实现实际的 Agent 列表查询
    // 目前返回空列表
    let response = ListAgentsResponse {
        agents: Vec::new(),
        total: 0,
        active: 0,
    };
    
    Ok(HttpResponse::Ok().json(response))
}

/// 清理非活跃 Agent
#[utoipa::path(
    post,
    path = "/api/v1/agents/cleanup",
    responses(
        (status = 200, description = "清理完成"),
        (status = 500, description = "服务器内部错误")
    ),
    tag = "agents"
)]
pub async fn cleanup_agents(
    agent_runtime: web::Data<Arc<AgentRuntime>>,
    tenant_info: web::ReqData<TenantInfo>,
) -> ActixResult<HttpResponse> {
    debug!("清理非活跃 Agent: tenant_id={}", tenant_info.tenant_id);
    
    match agent_runtime.cleanup_inactive_agents().await {
        Ok(cleaned_count) => {
            info!("清理了 {} 个非活跃 Agent", cleaned_count);
            Ok(HttpResponse::Ok().json(serde_json::json!({
                "message": "清理完成",
                "cleaned_count": cleaned_count
            })))
        }
        Err(e) => {
            error!("清理 Agent 失败: {}", e);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "清理失败",
                "message": e.to_string()
            })))
        }
    }
}

/// 查询参数
#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

/// 配置 Agent API 路由
pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/agents")
            .route("", web::post().to(create_agent))
            .route("", web::get().to(list_agents))
            .route("/cleanup", web::post().to(cleanup_agents))
            .route("/{agent_id}/execute", web::post().to(execute_task))
            .route("/{agent_id}/status", web::get().to(get_agent_status))
            .route("/{agent_id}/stop", web::post().to(stop_agent))
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, App};
    
    #[actix_web::test]
    async fn test_create_agent_request_validation() {
        let request = CreateAgentRequest {
            name: "测试 Agent".to_string(),
            description: "用于测试的 Agent".to_string(),
            system_prompt: "你是一个有用的助手".to_string(),
            available_tools: vec!["search".to_string()],
            reasoning_strategy: ReasoningStrategy::React,
            temperature: 0.7,
            max_tokens: 2000,
        };
        
        let json = serde_json::to_string(&request).unwrap();
        let deserialized: CreateAgentRequest = serde_json::from_str(&json).unwrap();
        
        assert_eq!(request.name, deserialized.name);
        assert_eq!(request.reasoning_strategy, deserialized.reasoning_strategy);
    }
}