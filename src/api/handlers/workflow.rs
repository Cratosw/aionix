// 工作流管理 API 处理器

use std::sync::Arc;
use std::collections::HashMap;
use actix_web::{web, HttpResponse, Result as ActixResult};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use tracing::{info, error, debug};
use utoipa::ToSchema;

use crate::ai::{
    workflow_engine::{WorkflowEngine, WorkflowDefinition, WorkflowStatus, ValidationResult},
    workflow_executor::{WorkflowExecutor, ExecutionRequest},
    agent_runtime::ExecutionContext,
};
use crate::db::entities::workflow_execution::ExecutionOptions;
use crate::errors::AiStudioError;
use crate::api::middleware::tenant::TenantInfo;

/// 工作流创建请求
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateWorkflowRequest {
    /// 工作流名称
    pub name: String,
    /// 工作流描述
    pub description: String,
    /// 工作流版本
    pub version: String,
    /// 工作流定义（JSON 字符串）
    pub workflow_definition: String,
}

/// 工作流创建响应
#[derive(Debug, Serialize, ToSchema)]
pub struct CreateWorkflowResponse {
    /// 工作流 ID
    pub workflow_id: Uuid,
    /// 工作流名称
    pub name: String,
    /// 创建时间
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// 验证结果
    pub validation_result: ValidationSummary,
}

/// 验证摘要
#[derive(Debug, Serialize, ToSchema)]
pub struct ValidationSummary {
    /// 是否有效
    pub is_valid: bool,
    /// 错误数量
    pub error_count: usize,
    /// 警告数量
    pub warning_count: usize,
    /// 主要错误信息
    pub main_errors: Vec<String>,
}

/// 工作流执行请求
#[derive(Debug, Deserialize, ToSchema)]
pub struct ExecuteWorkflowRequest {
    /// 执行参数
    pub parameters: HashMap<String, serde_json::Value>,
    /// 是否异步执行
    #[serde(default = "default_async")]
    pub async_execution: bool,
    /// 超时时间（秒）
    pub timeout_seconds: Option<u64>,
    /// 是否启用详细日志
    #[serde(default = "default_detailed_logs")]
    pub enable_detailed_logs: bool,
}

fn default_async() -> bool { true }
fn default_detailed_logs() -> bool { true }

/// 工作流执行响应
#[derive(Debug, Serialize, ToSchema)]
pub struct ExecuteWorkflowResponse {
    /// 执行 ID
    pub execution_id: Uuid,
    /// 工作流 ID
    pub workflow_id: Uuid,
    /// 执行状态
    pub status: String,
    /// 开始时间
    pub started_at: chrono::DateTime<chrono::Utc>,
    /// 预计完成时间
    pub estimated_completion: Option<chrono::DateTime<chrono::Utc>>,
}

/// 工作流列表查询参数
#[derive(Debug, Deserialize, ToSchema)]
pub struct WorkflowListQuery {
    /// 状态过滤
    pub status: Option<WorkflowStatus>,
    /// 名称搜索
    pub name: Option<String>,
    /// 分页大小
    pub limit: Option<u32>,
    /// 分页偏移
    pub offset: Option<u32>,
}

/// 工作流列表响应
#[derive(Debug, Serialize, ToSchema)]
pub struct WorkflowListResponse {
    /// 工作流列表
    pub workflows: Vec<WorkflowSummary>,
    /// 总数
    pub total: usize,
    /// 分页信息
    pub pagination: PaginationInfo,
}

/// 工作流摘要
#[derive(Debug, Serialize, ToSchema)]
pub struct WorkflowSummary {
    /// 工作流 ID
    pub id: Uuid,
    /// 工作流名称
    pub name: String,
    /// 工作流描述
    pub description: String,
    /// 工作流版本
    pub version: String,
    /// 工作流状态
    pub status: WorkflowStatus,
    /// 步骤数量
    pub step_count: usize,
    /// 创建时间
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// 更新时间
    pub updated_at: chrono::DateTime<chrono::Utc>,
    /// 最近执行时间
    pub last_executed_at: Option<chrono::DateTime<chrono::Utc>>,
    /// 执行统计
    pub execution_stats: WorkflowExecutionStats,
}

/// 工作流执行统计
#[derive(Debug, Serialize, ToSchema)]
pub struct WorkflowExecutionStats {
    /// 总执行次数
    pub total_executions: u64,
    /// 成功执行次数
    pub successful_executions: u64,
    /// 失败执行次数
    pub failed_executions: u64,
    /// 平均执行时间（毫秒）
    pub avg_execution_time_ms: f32,
}

/// 分页信息
#[derive(Debug, Serialize, ToSchema)]
pub struct PaginationInfo {
    /// 当前页
    pub page: u32,
    /// 每页大小
    pub page_size: u32,
    /// 总页数
    pub total_pages: u32,
    /// 是否有下一页
    pub has_next: bool,
    /// 是否有上一页
    pub has_prev: bool,
}

/// 执行历史查询参数
#[derive(Debug, Deserialize, ToSchema)]
pub struct ExecutionHistoryQuery {
    /// 状态过滤
    pub status: Option<String>,
    /// 开始时间
    pub start_time: Option<chrono::DateTime<chrono::Utc>>,
    /// 结束时间
    pub end_time: Option<chrono::DateTime<chrono::Utc>>,
    /// 分页大小
    pub limit: Option<u32>,
    /// 分页偏移
    pub offset: Option<u32>,
}

/// 执行历史响应
#[derive(Debug, Serialize, ToSchema)]
pub struct ExecutionHistoryResponse {
    /// 执行历史列表
    pub executions: Vec<ExecutionSummary>,
    /// 总数
    pub total: usize,
    /// 分页信息
    pub pagination: PaginationInfo,
}

/// 执行摘要
#[derive(Debug, Serialize, ToSchema)]
pub struct ExecutionSummary {
    /// 执行 ID
    pub execution_id: Uuid,
    /// 工作流 ID
    pub workflow_id: Uuid,
    /// 工作流名称
    pub workflow_name: String,
    /// 执行状态
    pub status: String,
    /// 开始时间
    pub started_at: chrono::DateTime<chrono::Utc>,
    /// 完成时间
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    /// 执行时间（毫秒）
    pub execution_time_ms: u64,
    /// 步骤统计
    pub step_stats: StepStats,
    /// 错误信息
    pub error: Option<String>,
}

/// 步骤统计
#[derive(Debug, Serialize, ToSchema)]
pub struct StepStats {
    /// 总步骤数
    pub total: u32,
    /// 已完成步骤数
    pub completed: u32,
    /// 失败步骤数
    pub failed: u32,
    /// 跳过步骤数
    pub skipped: u32,
}

/// 创建工作流
#[utoipa::path(
    post,
    path = "/api/v1/workflows",
    request_body = CreateWorkflowRequest,
    responses(
        (status = 201, description = "工作流创建成功", body = CreateWorkflowResponse),
        (status = 400, description = "请求参数错误"),
        (status = 401, description = "未授权"),
        (status = 500, description = "服务器内部错误")
    ),
    tag = "workflows"
)]
pub async fn create_workflow(
    workflow_engine: web::Data<Arc<WorkflowEngine>>,
    tenant_info: web::ReqData<TenantInfo>,
    request: web::Json<CreateWorkflowRequest>,
) -> ActixResult<HttpResponse> {
    debug!("创建工作流: tenant_id={}, name={}", tenant_info.id, request.name);
    
    // 解析工作流定义
    let mut workflow = match workflow_engine.parse_workflow(&request.workflow_definition).await {
        Ok(workflow) => workflow,
        Err(e) => {
            error!("工作流定义解析失败: {}", e);
            return Ok(HttpResponse::BadRequest().json(serde_json::json!({
                "error": "工作流定义解析失败",
                "message": e.to_string()
            })));
        }
    };
    
    // 设置工作流基本信息
    workflow.id = Uuid::new_v4();
    workflow.name = request.name.clone();
    workflow.description = request.description.clone();
    workflow.version = request.version.clone();
    workflow.tenant_id = tenant_info.id;
    workflow.created_by = Uuid::new_v4(); // TODO: 从认证中间件获取用户ID
    workflow.created_at = chrono::Utc::now();
    workflow.updated_at = chrono::Utc::now();
    workflow.status = WorkflowStatus::Draft;
    
    // 验证工作流
    let validation_result = match workflow_engine.validate_workflow(&workflow).await {
        Ok(result) => result,
        Err(e) => {
            error!("工作流验证失败: {}", e);
            return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "工作流验证失败",
                "message": e.to_string()
            })));
        }
    };
    
    // 注册工作流
    if let Err(e) = workflow_engine.register_workflow(workflow.clone()).await {
        error!("工作流注册失败: {}", e);
        return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
            "error": "工作流注册失败",
            "message": e.to_string()
        })));
    }
    
    info!("工作流创建成功: workflow_id={}, name={}", workflow.id, workflow.name);
    
    let response = CreateWorkflowResponse {
        workflow_id: workflow.id,
        name: workflow.name,
        created_at: workflow.created_at,
        validation_result: ValidationSummary {
            is_valid: validation_result.is_valid,
            error_count: validation_result.errors.len(),
            warning_count: validation_result.warnings.len(),
            main_errors: validation_result.errors.into_iter()
                .take(3)
                .map(|e| e.message)
                .collect(),
        },
    };
    
    Ok(HttpResponse::Created().json(response))
}

/// 执行工作流
#[utoipa::path(
    post,
    path = "/api/v1/workflows/{workflow_id}/execute",
    request_body = ExecuteWorkflowRequest,
    responses(
        (status = 200, description = "工作流执行启动成功", body = ExecuteWorkflowResponse),
        (status = 400, description = "请求参数错误"),
        (status = 404, description = "工作流不存在"),
        (status = 500, description = "服务器内部错误")
    ),
    params(
        ("workflow_id" = Uuid, Path, description = "工作流 ID")
    ),
    tag = "workflows"
)]
pub async fn execute_workflow(
    workflow_engine: web::Data<Arc<WorkflowEngine>>,
    workflow_executor: web::Data<Arc<WorkflowExecutor>>,
    tenant_info: web::ReqData<TenantInfo>,
    path: web::Path<Uuid>,
    request: web::Json<ExecuteWorkflowRequest>,
) -> ActixResult<HttpResponse> {
    let workflow_id = path.into_inner();
    debug!("执行工作流: workflow_id={}, tenant_id={}", workflow_id, tenant_info.id);
    
    // 获取工作流定义
    let workflow = match workflow_engine.get_workflow(workflow_id).await {
        Ok(workflow) => workflow,
        Err(e) => {
            error!("获取工作流失败: workflow_id={}, error={}", workflow_id, e);
            return Ok(HttpResponse::NotFound().json(serde_json::json!({
                "error": "工作流不存在",
                "message": e.to_string()
            })));
        }
    };
    
    // 检查租户权限
    if workflow.tenant_id != tenant_info.id {
        return Ok(HttpResponse::Forbidden().json(serde_json::json!({
            "error": "无权限访问此工作流"
        })));
    }
    
    // 检查工作流状态
    if workflow.status != WorkflowStatus::Published {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "error": "只能执行已发布的工作流",
            "current_status": workflow.status
        })));
    }
    
    // 构建执行请求
    let execution_context = ExecutionContext {
        tenant_id: tenant_info.tenant_id,
        user_id: tenant_info.user_id,
        session_id: None,
        variables: HashMap::new(),
        environment: "production".to_string(),
    };
    
    let execution_options = ExecutionOptions {
        async_execution: request.async_execution,
        timeout_seconds: request.timeout_seconds,
        enable_detailed_logs: request.enable_detailed_logs,
        enable_rollback: false,
    };
    
    let execution_request = ExecutionRequest {
        workflow: workflow.clone(),
        parameters: request.parameters.clone(),
        context: execution_context,
        options: execution_options,
    };
    
    // 启动执行
    let execution_id = match workflow_executor.execute_workflow(execution_request).await {
        Ok(execution_id) => execution_id,
        Err(e) => {
            error!("启动工作流执行失败: workflow_id={}, error={}", workflow_id, e);
            return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "启动工作流执行失败",
                "message": e.to_string()
            })));
        }
    };
    
    info!("工作流执行启动成功: workflow_id={}, execution_id={}", workflow_id, execution_id);
    
    let response = ExecuteWorkflowResponse {
        execution_id,
        workflow_id,
        status: "running".to_string(),
        started_at: chrono::Utc::now(),
        estimated_completion: None, // TODO: 计算预计完成时间
    };
    
    Ok(HttpResponse::Ok().json(response))
}

/// 获取工作流列表
#[utoipa::path(
    get,
    path = "/api/v1/workflows",
    responses(
        (status = 200, description = "获取工作流列表成功", body = WorkflowListResponse),
        (status = 500, description = "服务器内部错误")
    ),
    params(
        ("status" = Option<WorkflowStatus>, Query, description = "状态过滤"),
        ("name" = Option<String>, Query, description = "名称搜索"),
        ("limit" = Option<u32>, Query, description = "分页大小"),
        ("offset" = Option<u32>, Query, description = "分页偏移")
    ),
    tag = "workflows"
)]
pub async fn list_workflows(
    workflow_engine: web::Data<Arc<WorkflowEngine>>,
    tenant_info: web::ReqData<TenantInfo>,
    query: web::Query<WorkflowListQuery>,
) -> ActixResult<HttpResponse> {
    debug!("获取工作流列表: tenant_id={}", tenant_info.id);
    
    // 获取租户的工作流列表
    let workflows = match workflow_engine.list_workflows(Some(tenant_info.id)).await {
        Ok(workflows) => workflows,
        Err(e) => {
            error!("获取工作流列表失败: {}", e);
            return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "获取工作流列表失败",
                "message": e.to_string()
            })));
        }
    };
    
    // 应用过滤条件
    let mut filtered_workflows = workflows;
    
    if let Some(ref status) = query.status {
        filtered_workflows.retain(|w| w.status == *status);
    }
    
    if let Some(ref name) = query.name {
        let name_lower = name.to_lowercase();
        filtered_workflows.retain(|w| w.name.to_lowercase().contains(&name_lower));
    }
    
    // 应用分页
    let total = filtered_workflows.len();
    let limit = query.limit.unwrap_or(20) as usize;
    let offset = query.offset.unwrap_or(0) as usize;
    
    let start = offset.min(total);
    let end = (offset + limit).min(total);
    let page_workflows = filtered_workflows.into_iter().skip(start).take(end - start).collect::<Vec<_>>();
    
    // 构建响应
    let workflow_summaries: Vec<WorkflowSummary> = page_workflows.into_iter().map(|w| {
        WorkflowSummary {
            id: w.id,
            name: w.name,
            description: w.description,
            version: w.version,
            status: w.status,
            step_count: w.steps.len(),
            created_at: w.created_at,
            updated_at: w.updated_at,
            last_executed_at: None, // TODO: 从执行历史获取
            execution_stats: WorkflowExecutionStats {
                total_executions: 0,
                successful_executions: 0,
                failed_executions: 0,
                avg_execution_time_ms: 0.0,
            },
        }
    }).collect();
    
    let total_pages = (total + limit - 1) / limit;
    let current_page = offset / limit + 1;
    
    let response = WorkflowListResponse {
        workflows: workflow_summaries,
        total,
        pagination: PaginationInfo {
            page: current_page as u32,
            page_size: limit as u32,
            total_pages: total_pages as u32,
            has_next: end < total,
            has_prev: offset > 0,
        },
    };
    
    Ok(HttpResponse::Ok().json(response))
}

/// 获取工作流详情
#[utoipa::path(
    get,
    path = "/api/v1/workflows/{workflow_id}",
    responses(
        (status = 200, description = "获取工作流详情成功", body = WorkflowDefinition),
        (status = 404, description = "工作流不存在"),
        (status = 500, description = "服务器内部错误")
    ),
    params(
        ("workflow_id" = Uuid, Path, description = "工作流 ID")
    ),
    tag = "workflows"
)]
pub async fn get_workflow(
    workflow_engine: web::Data<Arc<WorkflowEngine>>,
    tenant_info: web::ReqData<TenantInfo>,
    path: web::Path<Uuid>,
) -> ActixResult<HttpResponse> {
    let workflow_id = path.into_inner();
    debug!("获取工作流详情: workflow_id={}, tenant_id={}", workflow_id, tenant_info.id);
    
    match workflow_engine.get_workflow(workflow_id).await {
        Ok(workflow) => {
            // 检查租户权限
            if workflow.tenant_id != tenant_info.id {
                return Ok(HttpResponse::Forbidden().json(serde_json::json!({
                    "error": "无权限访问此工作流"
                })));
            }
            
            Ok(HttpResponse::Ok().json(workflow))
        }
        Err(e) => {
            error!("获取工作流详情失败: workflow_id={}, error={}", workflow_id, e);
            
            let error_response = match e {
                AiStudioError::NotFound { resource: _ } => HttpResponse::NotFound(),
                _ => HttpResponse::InternalServerError(),
            };
            
            Ok(error_response.json(serde_json::json!({
                "error": "获取工作流详情失败",
                "message": e.to_string()
            })))
        }
    }
}

/// 获取执行状态
#[utoipa::path(
    get,
    path = "/api/v1/workflows/executions/{execution_id}",
    responses(
        (status = 200, description = "获取执行状态成功", body = WorkflowExecution),
        (status = 404, description = "执行不存在"),
        (status = 500, description = "服务器内部错误")
    ),
    params(
        ("execution_id" = Uuid, Path, description = "执行 ID")
    ),
    tag = "workflows"
)]
pub async fn get_execution_status(
    workflow_executor: web::Data<Arc<WorkflowExecutor>>,
    tenant_info: web::ReqData<TenantInfo>,
    path: web::Path<Uuid>,
) -> ActixResult<HttpResponse> {
    let execution_id = path.into_inner();
    debug!("获取执行状态: execution_id={}, tenant_id={}", execution_id, tenant_info.tenant_id);
    
    match workflow_executor.get_execution_status(execution_id).await {
        Ok(execution) => {
            // 检查租户权限
            if execution.context.tenant_id != tenant_info.tenant_id {
                return Ok(HttpResponse::Forbidden().json(serde_json::json!({
                    "error": "无权限访问此执行"
                })));
            }
            
            Ok(HttpResponse::Ok().json(execution))
        }
        Err(e) => {
            error!("获取执行状态失败: execution_id={}, error={}", execution_id, e);
            
            let error_response = match e {
                AiStudioError::NotFound { resource: _ } => HttpResponse::NotFound(),
                _ => HttpResponse::InternalServerError(),
            };
            
            Ok(error_response.json(serde_json::json!({
                "error": "获取执行状态失败",
                "message": e.to_string()
            })))
        }
    }
}

/// 取消执行
#[utoipa::path(
    post,
    path = "/api/v1/workflows/executions/{execution_id}/cancel",
    responses(
        (status = 200, description = "取消执行成功"),
        (status = 404, description = "执行不存在"),
        (status = 500, description = "服务器内部错误")
    ),
    params(
        ("execution_id" = Uuid, Path, description = "执行 ID")
    ),
    tag = "workflows"
)]
pub async fn cancel_execution(
    workflow_executor: web::Data<Arc<WorkflowExecutor>>,
    tenant_info: web::ReqData<TenantInfo>,
    path: web::Path<Uuid>,
) -> ActixResult<HttpResponse> {
    let execution_id = path.into_inner();
    debug!("取消执行: execution_id={}, tenant_id={}", execution_id, tenant_info.tenant_id);
    
    // 检查执行是否存在和权限
    match workflow_executor.get_execution_status(execution_id).await {
        Ok(execution) => {
            if execution.context.tenant_id != tenant_info.tenant_id {
                return Ok(HttpResponse::Forbidden().json(serde_json::json!({
                    "error": "无权限访问此执行"
                })));
            }
        }
        Err(e) => {
            error!("获取执行状态失败: execution_id={}, error={}", execution_id, e);
            return Ok(HttpResponse::NotFound().json(serde_json::json!({
                "error": "执行不存在",
                "message": e.to_string()
            })));
        }
    }
    
    // 取消执行
    match workflow_executor.cancel_execution(execution_id).await {
        Ok(_) => {
            info!("执行取消成功: execution_id={}", execution_id);
            Ok(HttpResponse::Ok().json(serde_json::json!({
                "message": "执行取消成功",
                "execution_id": execution_id,
                "cancelled_at": chrono::Utc::now()
            })))
        }
        Err(e) => {
            error!("取消执行失败: execution_id={}, error={}", execution_id, e);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "取消执行失败",
                "message": e.to_string()
            })))
        }
    }
}

/// 获取执行历史
#[utoipa::path(
    get,
    path = "/api/v1/workflows/{workflow_id}/executions",
    responses(
        (status = 200, description = "获取执行历史成功", body = ExecutionHistoryResponse),
        (status = 404, description = "工作流不存在"),
        (status = 500, description = "服务器内部错误")
    ),
    params(
        ("workflow_id" = Uuid, Path, description = "工作流 ID"),
        ("status" = Option<String>, Query, description = "状态过滤"),
        ("start_time" = Option<chrono::DateTime<chrono::Utc>>, Query, description = "开始时间"),
        ("end_time" = Option<chrono::DateTime<chrono::Utc>>, Query, description = "结束时间"),
        ("limit" = Option<u32>, Query, description = "分页大小"),
        ("offset" = Option<u32>, Query, description = "分页偏移")
    ),
    tag = "workflows"
)]
pub async fn get_execution_history(
    workflow_engine: web::Data<Arc<WorkflowEngine>>,
    tenant_info: web::ReqData<TenantInfo>,
    path: web::Path<Uuid>,
    query: web::Query<ExecutionHistoryQuery>,
) -> ActixResult<HttpResponse> {
    let workflow_id = path.into_inner();
    debug!("获取执行历史: workflow_id={}, tenant_id={}", workflow_id, tenant_info.tenant_id);
    
    // 检查工作流是否存在和权限
    match workflow_engine.get_workflow(workflow_id).await {
        Ok(workflow) => {
            if workflow.tenant_id != tenant_info.tenant_id {
                return Ok(HttpResponse::Forbidden().json(serde_json::json!({
                    "error": "无权限访问此工作流"
                })));
            }
        }
        Err(e) => {
            error!("获取工作流失败: workflow_id={}, error={}", workflow_id, e);
            return Ok(HttpResponse::NotFound().json(serde_json::json!({
                "error": "工作流不存在",
                "message": e.to_string()
            })));
        }
    }
    
    // TODO: 实现从数据库查询执行历史
    // 目前返回模拟数据
    let executions = vec![];
    let total = 0;
    
    let limit = query.limit.unwrap_or(20) as usize;
    let offset = query.offset.unwrap_or(0) as usize;
    let total_pages = (total + limit - 1) / limit;
    let current_page = offset / limit + 1;
    
    let response = ExecutionHistoryResponse {
        executions,
        total,
        pagination: PaginationInfo {
            page: current_page as u32,
            page_size: limit as u32,
            total_pages: total_pages as u32,
            has_next: offset + limit < total,
            has_prev: offset > 0,
        },
    };
    
    Ok(HttpResponse::Ok().json(response))
}

/// 发布工作流
#[utoipa::path(
    post,
    path = "/api/v1/workflows/{workflow_id}/publish",
    responses(
        (status = 200, description = "工作流发布成功"),
        (status = 400, description = "工作流验证失败"),
        (status = 404, description = "工作流不存在"),
        (status = 500, description = "服务器内部错误")
    ),
    params(
        ("workflow_id" = Uuid, Path, description = "工作流 ID")
    ),
    tag = "workflows"
)]
pub async fn publish_workflow(
    workflow_engine: web::Data<Arc<WorkflowEngine>>,
    tenant_info: web::ReqData<TenantInfo>,
    path: web::Path<Uuid>,
) -> ActixResult<HttpResponse> {
    let workflow_id = path.into_inner();
    debug!("发布工作流: workflow_id={}, tenant_id={}", workflow_id, tenant_info.tenant_id);
    
    // 获取工作流
    let mut workflow = match workflow_engine.get_workflow(workflow_id).await {
        Ok(workflow) => workflow,
        Err(e) => {
            error!("获取工作流失败: workflow_id={}, error={}", workflow_id, e);
            return Ok(HttpResponse::NotFound().json(serde_json::json!({
                "error": "工作流不存在",
                "message": e.to_string()
            })));
        }
    };
    
    // 检查租户权限
    if workflow.tenant_id != tenant_info.tenant_id {
        return Ok(HttpResponse::Forbidden().json(serde_json::json!({
            "error": "无权限访问此工作流"
        })));
    }
    
    // 验证工作流
    let validation_result = match workflow_engine.validate_workflow(&workflow).await {
        Ok(result) => result,
        Err(e) => {
            error!("工作流验证失败: {}", e);
            return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "工作流验证失败",
                "message": e.to_string()
            })));
        }
    };
    
    if !validation_result.is_valid {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "error": "工作流验证失败，无法发布",
            "validation_errors": validation_result.errors.into_iter()
                .map(|e| e.message)
                .collect::<Vec<_>>()
        })));
    }
    
    // 更新工作流状态
    workflow.status = WorkflowStatus::Published;
    workflow.updated_at = chrono::Utc::now();
    
    // 重新注册工作流
    if let Err(e) = workflow_engine.register_workflow(workflow).await {
        error!("工作流注册失败: {}", e);
        return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
            "error": "工作流发布失败",
            "message": e.to_string()
        })));
    }
    
    info!("工作流发布成功: workflow_id={}", workflow_id);
    
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "message": "工作流发布成功",
        "workflow_id": workflow_id,
        "published_at": chrono::Utc::now()
    })))
}

/// 配置工作流 API 路由
pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/workflows")
            .route("", web::post().to(create_workflow))
            .route("", web::get().to(list_workflows))
            .route("/{workflow_id}", web::get().to(get_workflow))
            .route("/{workflow_id}/execute", web::post().to(execute_workflow))
            .route("/{workflow_id}/publish", web::post().to(publish_workflow))
            .route("/{workflow_id}/executions", web::get().to(get_execution_history))
            .route("/executions/{execution_id}", web::get().to(get_execution_status))
            .route("/executions/{execution_id}/cancel", web::post().to(cancel_execution))
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_create_workflow_request_validation() {
        let request = CreateWorkflowRequest {
            name: "测试工作流".to_string(),
            description: "用于测试的工作流".to_string(),
            version: "1.0.0".to_string(),
            workflow_definition: r#"{"steps": []}"#.to_string(),
        };
        
        let json = serde_json::to_string(&request).unwrap();
        let deserialized: CreateWorkflowRequest = serde_json::from_str(&json).unwrap();
        
        assert_eq!(request.name, deserialized.name);
        assert_eq!(request.version, deserialized.version);
    }
    
    #[test]
    fn test_execution_request_defaults() {
        let request = ExecuteWorkflowRequest {
            parameters: HashMap::new(),
            async_execution: default_async(),
            timeout_seconds: None,
            enable_detailed_logs: default_detailed_logs(),
        };
        
        assert!(request.async_execution);
        assert!(request.enable_detailed_logs);
    }
}