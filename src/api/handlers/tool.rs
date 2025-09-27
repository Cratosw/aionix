// 工具管理 API 处理器

use std::sync::Arc;
use std::collections::HashMap;
use actix_web::{web, HttpResponse, Result as ActixResult};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use tracing::{info, error, debug};
use utoipa::ToSchema;

use crate::ai::{
    tool_manager::{ToolManager, ToolPermissions, ToolUsageStats, PermissionLevel},
    tool_loader::{ToolLoader, ToolLoadResult},
    agent_runtime::ExecutionContext,
};
use crate::errors::AiStudioError;
use crate::middleware::auth::TenantInfo;

/// 工具调用请求
#[derive(Debug, Deserialize, ToSchema)]
pub struct ToolCallRequest {
    /// 工具名称
    pub tool_name: String,
    /// 调用参数
    pub parameters: HashMap<String, serde_json::Value>,
    /// 超时时间（秒）
    pub timeout_seconds: Option<u64>,
}

/// 工具调用响应
#[derive(Debug, Serialize, ToSchema)]
pub struct ToolCallResponse {
    /// 调用 ID
    pub call_id: Uuid,
    /// 工具名称
    pub tool_name: String,
    /// 执行结果
    pub result: crate::ai::agent_runtime::ToolResult,
    /// 执行时间（毫秒）
    pub execution_time_ms: u64,
    /// 调用时间
    pub called_at: chrono::DateTime<chrono::Utc>,
}

/// 工具权限更新请求
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateToolPermissionsRequest {
    /// 是否启用
    pub enabled: Option<bool>,
    /// 允许的租户 ID 列表
    pub allowed_tenants: Option<Vec<Uuid>>,
    /// 禁止的租户 ID 列表
    pub blocked_tenants: Option<Vec<Uuid>>,
    /// 允许的用户 ID 列表
    pub allowed_users: Option<Vec<Uuid>>,
    /// 禁止的用户 ID 列表
    pub blocked_users: Option<Vec<Uuid>>,
    /// 每小时调用限制
    pub hourly_limit: Option<u32>,
    /// 每天调用限制
    pub daily_limit: Option<u32>,
    /// 需要的权限级别
    pub required_permission_level: Option<PermissionLevel>,
}

/// 工具列表查询参数
#[derive(Debug, Deserialize, ToSchema)]
pub struct ToolListQuery {
    /// 工具类别过滤
    pub category: Option<String>,
    /// 是否只显示启用的工具
    pub enabled_only: Option<bool>,
    /// 分页大小
    pub limit: Option<u32>,
    /// 分页偏移
    pub offset: Option<u32>,
}

/// 工具重新加载请求
#[derive(Debug, Deserialize, ToSchema)]
pub struct ReloadToolRequest {
    /// 工具名称
    pub tool_name: String,
}

/// 工具重新加载响应
#[derive(Debug, Serialize, ToSchema)]
pub struct ReloadToolResponse {
    /// 工具名称
    pub tool_name: String,
    /// 重新加载是否成功
    pub success: bool,
    /// 错误信息
    pub error: Option<String>,
    /// 重新加载时间
    pub reloaded_at: chrono::DateTime<chrono::Utc>,
}

/// 调用工具
#[utoipa::path(
    post,
    path = "/api/v1/tools/call",
    request_body = ToolCallRequest,
    responses(
        (status = 200, description = "工具调用成功", body = ToolCallResponse),
        (status = 400, description = "请求参数错误"),
        (status = 403, description = "权限不足"),
        (status = 404, description = "工具不存在"),
        (status = 500, description = "服务器内部错误")
    ),
    tag = "tools"
)]
pub async fn call_tool(
    tool_manager: web::Data<Arc<ToolManager>>,
    tenant_info: web::ReqData<TenantInfo>,
    request: web::Json<ToolCallRequest>,
) -> ActixResult<HttpResponse> {
    debug!("调用工具: {} (tenant_id={})", request.tool_name, tenant_info.tenant_id);
    
    let call_id = Uuid::new_v4();
    
    // 构建执行上下文
    let mut context_variables = HashMap::new();
    context_variables.insert("tenant_id".to_string(), serde_json::Value::String(tenant_info.tenant_id.to_string()));
    
    let execution_context = ExecutionContext {
        current_task: None,
        execution_history: Vec::new(),
        context_variables,
        session_id: None,
        user_id: tenant_info.user_id,
    };
    
    // 构建工具调用请求
    let tool_call_request = crate::ai::tool_manager::ToolCallRequest {
        tool_name: request.tool_name.clone(),
        parameters: request.parameters.clone(),
        context: execution_context,
        call_id,
        timeout_seconds: request.timeout_seconds,
    };
    
    match tool_manager.call_tool(tool_call_request).await {
        Ok(response) => {
            info!("工具调用成功: {} (call_id={}, 执行时间={}ms)", 
                  request.tool_name, call_id, response.execution_time_ms);
            
            let api_response = ToolCallResponse {
                call_id: response.call_id,
                tool_name: response.tool_name,
                result: response.result,
                execution_time_ms: response.execution_time_ms,
                called_at: response.started_at,
            };
            
            Ok(HttpResponse::Ok().json(api_response))
        }
        Err(e) => {
            error!("工具调用失败: {} - {}", request.tool_name, e);
            
            let error_response = match e {
                AiStudioError::NotFound(_) => HttpResponse::NotFound(),
                AiStudioError::PermissionDenied(_) => HttpResponse::Forbidden(),
                AiStudioError::Validation(_) => HttpResponse::BadRequest(),
                _ => HttpResponse::InternalServerError(),
            };
            
            Ok(error_response.json(serde_json::json!({
                "error": "工具调用失败",
                "message": e.to_string(),
                "tool_name": request.tool_name,
                "call_id": call_id
            })))
        }
    }
}

/// 获取工具列表
#[utoipa::path(
    get,
    path = "/api/v1/tools",
    responses(
        (status = 200, description = "获取工具列表成功", body = crate::ai::tool_manager::ToolListResponse),
        (status = 500, description = "服务器内部错误")
    ),
    params(
        ("category" = Option<String>, Query, description = "工具类别过滤"),
        ("enabled_only" = Option<bool>, Query, description = "是否只显示启用的工具"),
        ("limit" = Option<u32>, Query, description = "分页大小"),
        ("offset" = Option<u32>, Query, description = "分页偏移")
    ),
    tag = "tools"
)]
pub async fn list_tools(
    tool_manager: web::Data<Arc<ToolManager>>,
    tenant_info: web::ReqData<TenantInfo>,
    query: web::Query<ToolListQuery>,
) -> ActixResult<HttpResponse> {
    debug!("获取工具列表: tenant_id={}", tenant_info.tenant_id);
    
    match tool_manager.list_tools().await {
        Ok(mut response) => {
            // 应用过滤条件
            if let Some(ref category) = query.category {
                response.tools.retain(|tool| tool.metadata.category == *category);
            }
            
            if query.enabled_only.unwrap_or(false) {
                response.tools.retain(|tool| tool.permissions.enabled);
            }
            
            // 应用分页
            let offset = query.offset.unwrap_or(0) as usize;
            let limit = query.limit.unwrap_or(50) as usize;
            
            let total = response.tools.len();
            let start = offset.min(total);
            let end = (offset + limit).min(total);
            
            response.tools = response.tools.into_iter().skip(start).take(end - start).collect();
            response.total = total;
            
            Ok(HttpResponse::Ok().json(response))
        }
        Err(e) => {
            error!("获取工具列表失败: {}", e);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "获取工具列表失败",
                "message": e.to_string()
            })))
        }
    }
}

/// 获取工具元数据
#[utoipa::path(
    get,
    path = "/api/v1/tools/{tool_name}/metadata",
    responses(
        (status = 200, description = "获取工具元数据成功", body = crate::ai::agent_runtime::ToolMetadata),
        (status = 404, description = "工具不存在"),
        (status = 500, description = "服务器内部错误")
    ),
    params(
        ("tool_name" = String, Path, description = "工具名称")
    ),
    tag = "tools"
)]
pub async fn get_tool_metadata(
    tool_manager: web::Data<Arc<ToolManager>>,
    tenant_info: web::ReqData<TenantInfo>,
    path: web::Path<String>,
) -> ActixResult<HttpResponse> {
    let tool_name = path.into_inner();
    debug!("获取工具元数据: {} (tenant_id={})", tool_name, tenant_info.tenant_id);
    
    match tool_manager.get_tool_metadata(&tool_name).await {
        Ok(metadata) => {
            Ok(HttpResponse::Ok().json(metadata))
        }
        Err(e) => {
            error!("获取工具元数据失败: {} - {}", tool_name, e);
            
            let error_response = match e {
                AiStudioError::NotFound(_) => HttpResponse::NotFound(),
                _ => HttpResponse::InternalServerError(),
            };
            
            Ok(error_response.json(serde_json::json!({
                "error": "获取工具元数据失败",
                "message": e.to_string(),
                "tool_name": tool_name
            })))
        }
    }
}

/// 更新工具权限
#[utoipa::path(
    put,
    path = "/api/v1/tools/{tool_name}/permissions",
    request_body = UpdateToolPermissionsRequest,
    responses(
        (status = 200, description = "更新工具权限成功"),
        (status = 400, description = "请求参数错误"),
        (status = 404, description = "工具不存在"),
        (status = 500, description = "服务器内部错误")
    ),
    params(
        ("tool_name" = String, Path, description = "工具名称")
    ),
    tag = "tools"
)]
pub async fn update_tool_permissions(
    tool_manager: web::Data<Arc<ToolManager>>,
    tenant_info: web::ReqData<TenantInfo>,
    path: web::Path<String>,
    request: web::Json<UpdateToolPermissionsRequest>,
) -> ActixResult<HttpResponse> {
    let tool_name = path.into_inner();
    debug!("更新工具权限: {} (tenant_id={})", tool_name, tenant_info.tenant_id);
    
    // 获取当前权限配置
    let current_metadata = match tool_manager.get_tool_metadata(&tool_name).await {
        Ok(metadata) => metadata,
        Err(e) => {
            error!("获取工具元数据失败: {} - {}", tool_name, e);
            return Ok(HttpResponse::NotFound().json(serde_json::json!({
                "error": "工具不存在",
                "message": e.to_string(),
                "tool_name": tool_name
            })));
        }
    };
    
    // 构建新的权限配置
    let mut new_permissions = ToolPermissions {
        tool_name: tool_name.clone(),
        enabled: request.enabled.unwrap_or(true),
        allowed_tenants: request.allowed_tenants.clone().unwrap_or_default(),
        blocked_tenants: request.blocked_tenants.clone().unwrap_or_default(),
        allowed_users: request.allowed_users.clone().unwrap_or_default(),
        blocked_users: request.blocked_users.clone().unwrap_or_default(),
        hourly_limit: request.hourly_limit,
        daily_limit: request.daily_limit,
        required_permission_level: request.required_permission_level.clone().unwrap_or(PermissionLevel::Basic),
    };
    
    match tool_manager.update_tool_permissions(&tool_name, new_permissions).await {
        Ok(_) => {
            info!("工具权限更新成功: {}", tool_name);
            Ok(HttpResponse::Ok().json(serde_json::json!({
                "message": "工具权限更新成功",
                "tool_name": tool_name,
                "updated_at": chrono::Utc::now()
            })))
        }
        Err(e) => {
            error!("更新工具权限失败: {} - {}", tool_name, e);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "更新工具权限失败",
                "message": e.to_string(),
                "tool_name": tool_name
            })))
        }
    }
}

/// 获取工具使用统计
#[utoipa::path(
    get,
    path = "/api/v1/tools/{tool_name}/stats",
    responses(
        (status = 200, description = "获取工具使用统计成功", body = ToolUsageStats),
        (status = 404, description = "工具不存在"),
        (status = 500, description = "服务器内部错误")
    ),
    params(
        ("tool_name" = String, Path, description = "工具名称")
    ),
    tag = "tools"
)]
pub async fn get_tool_usage_stats(
    tool_manager: web::Data<Arc<ToolManager>>,
    tenant_info: web::ReqData<TenantInfo>,
    path: web::Path<String>,
) -> ActixResult<HttpResponse> {
    let tool_name = path.into_inner();
    debug!("获取工具使用统计: {} (tenant_id={})", tool_name, tenant_info.tenant_id);
    
    match tool_manager.get_tool_usage_stats(&tool_name).await {
        Ok(stats) => {
            Ok(HttpResponse::Ok().json(stats))
        }
        Err(e) => {
            error!("获取工具使用统计失败: {} - {}", tool_name, e);
            
            let error_response = match e {
                AiStudioError::NotFound(_) => HttpResponse::NotFound(),
                _ => HttpResponse::InternalServerError(),
            };
            
            Ok(error_response.json(serde_json::json!({
                "error": "获取工具使用统计失败",
                "message": e.to_string(),
                "tool_name": tool_name
            })))
        }
    }
}

/// 获取所有工具使用统计
#[utoipa::path(
    get,
    path = "/api/v1/tools/stats",
    responses(
        (status = 200, description = "获取所有工具使用统计成功", body = Vec<ToolUsageStats>),
        (status = 500, description = "服务器内部错误")
    ),
    tag = "tools"
)]
pub async fn get_all_tool_usage_stats(
    tool_manager: web::Data<Arc<ToolManager>>,
    tenant_info: web::ReqData<TenantInfo>,
) -> ActixResult<HttpResponse> {
    debug!("获取所有工具使用统计: tenant_id={}", tenant_info.tenant_id);
    
    match tool_manager.get_all_usage_stats().await {
        Ok(stats) => {
            Ok(HttpResponse::Ok().json(stats))
        }
        Err(e) => {
            error!("获取所有工具使用统计失败: {}", e);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "获取工具使用统计失败",
                "message": e.to_string()
            })))
        }
    }
}

/// 重新加载工具
#[utoipa::path(
    post,
    path = "/api/v1/tools/reload",
    request_body = ReloadToolRequest,
    responses(
        (status = 200, description = "工具重新加载成功", body = ReloadToolResponse),
        (status = 400, description = "请求参数错误"),
        (status = 404, description = "工具不存在"),
        (status = 500, description = "服务器内部错误")
    ),
    tag = "tools"
)]
pub async fn reload_tool(
    tool_loader: web::Data<Arc<ToolLoader>>,
    tenant_info: web::ReqData<TenantInfo>,
    request: web::Json<ReloadToolRequest>,
) -> ActixResult<HttpResponse> {
    debug!("重新加载工具: {} (tenant_id={})", request.tool_name, tenant_info.tenant_id);
    
    let reload_start = chrono::Utc::now();
    
    match tool_loader.reload_tool(&request.tool_name).await {
        Ok(_) => {
            info!("工具重新加载成功: {}", request.tool_name);
            
            let response = ReloadToolResponse {
                tool_name: request.tool_name.clone(),
                success: true,
                error: None,
                reloaded_at: reload_start,
            };
            
            Ok(HttpResponse::Ok().json(response))
        }
        Err(e) => {
            error!("工具重新加载失败: {} - {}", request.tool_name, e);
            
            let response = ReloadToolResponse {
                tool_name: request.tool_name.clone(),
                success: false,
                error: Some(e.to_string()),
                reloaded_at: reload_start,
            };
            
            Ok(HttpResponse::InternalServerError().json(response))
        }
    }
}

/// 重新加载所有工具
#[utoipa::path(
    post,
    path = "/api/v1/tools/reload-all",
    responses(
        (status = 200, description = "所有工具重新加载完成", body = ToolLoadResult),
        (status = 500, description = "服务器内部错误")
    ),
    tag = "tools"
)]
pub async fn reload_all_tools(
    tool_loader: web::Data<Arc<ToolLoader>>,
    tenant_info: web::ReqData<TenantInfo>,
) -> ActixResult<HttpResponse> {
    debug!("重新加载所有工具: tenant_id={}", tenant_info.tenant_id);
    
    match tool_loader.load_all_tools().await {
        Ok(result) => {
            info!("所有工具重新加载完成: 成功={}, 失败={}, 跳过={}", 
                  result.loaded_count, result.failed_count, result.skipped_count);
            
            Ok(HttpResponse::Ok().json(result))
        }
        Err(e) => {
            error!("重新加载所有工具失败: {}", e);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "重新加载所有工具失败",
                "message": e.to_string()
            })))
        }
    }
}

/// 配置工具 API 路由
pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/tools")
            .route("/call", web::post().to(call_tool))
            .route("", web::get().to(list_tools))
            .route("/stats", web::get().to(get_all_tool_usage_stats))
            .route("/reload", web::post().to(reload_tool))
            .route("/reload-all", web::post().to(reload_all_tools))
            .route("/{tool_name}/metadata", web::get().to(get_tool_metadata))
            .route("/{tool_name}/permissions", web::put().to(update_tool_permissions))
            .route("/{tool_name}/stats", web::get().to(get_tool_usage_stats))
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_tool_call_request_validation() {
        let request = ToolCallRequest {
            tool_name: "calculator".to_string(),
            parameters: {
                let mut params = HashMap::new();
                params.insert("operation".to_string(), serde_json::Value::String("add".to_string()));
                params.insert("a".to_string(), serde_json::Value::Number(serde_json::Number::from(5)));
                params.insert("b".to_string(), serde_json::Value::Number(serde_json::Number::from(3)));
                params
            },
            timeout_seconds: Some(30),
        };
        
        let json = serde_json::to_string(&request).unwrap();
        let deserialized: ToolCallRequest = serde_json::from_str(&json).unwrap();
        
        assert_eq!(request.tool_name, deserialized.tool_name);
        assert_eq!(request.timeout_seconds, deserialized.timeout_seconds);
    }
}