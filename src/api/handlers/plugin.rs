// 插件管理 API 处理器

use std::sync::Arc;
use std::collections::HashMap;
use actix_web::{web, HttpResponse, Result as ActixResult};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use tracing::{info, error, debug};
use utoipa::ToSchema;

use crate::plugins::{
    plugin_manager::{PluginManager, InstallPluginRequest, InstallPluginResponse, PluginListResponse, PluginInfo},
    plugin_interface::{PluginConfig, PluginContext, PluginPermission, PluginStatus},
    plugin_registry::{PluginSearchQuery, PluginSearchResult, PluginStatistics},
};
use crate::errors::AiStudioError;
use crate::api::middleware::tenant::TenantInfo;

/// 插件调用请求
#[derive(Debug, Deserialize, ToSchema)]
pub struct PluginCallRequest {
    /// 插件 ID
    pub plugin_id: String,
    /// 调用方法
    pub method: String,
    /// 调用参数
    pub parameters: HashMap<String, serde_json::Value>,
}

/// 插件调用响应
#[derive(Debug, Serialize, ToSchema)]
pub struct PluginCallResponse {
    /// 插件 ID
    pub plugin_id: String,
    /// 调用方法
    pub method: String,
    /// 调用结果
    pub result: serde_json::Value,
    /// 执行时间（毫秒）
    pub execution_time_ms: u64,
    /// 调用时间
    pub called_at: chrono::DateTime<chrono::Utc>,
}

/// 插件配置更新请求
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdatePluginConfigRequest {
    /// 配置参数
    pub parameters: HashMap<String, serde_json::Value>,
    /// 环境变量
    pub environment: Option<HashMap<String, String>>,
    /// 是否重启插件
    pub restart_plugin: Option<bool>,
}

/// 插件状态控制请求
#[derive(Debug, Deserialize, ToSchema)]
pub struct PluginControlRequest {
    /// 操作类型
    pub action: PluginAction,
}

/// 插件操作类型
#[derive(Debug, Deserialize, ToSchema, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginAction {
    Start,
    Stop,
    Restart,
    Reload,
}

/// 安装插件
#[utoipa::path(
    post,
    path = "/api/v1/plugins/install",
    request_body = InstallPluginRequest,
    responses(
        (status = 200, description = "插件安装成功", body = InstallPluginResponse),
        (status = 400, description = "请求参数错误"),
        (status = 403, description = "权限不足"),
        (status = 500, description = "服务器内部错误")
    ),
    tag = "plugins"
)]
pub async fn install_plugin(
    plugin_manager: web::Data<Arc<PluginManager>>,
    tenant_info: web::ReqData<TenantInfo>,
    request: web::Json<InstallPluginRequest>,
) -> ActixResult<HttpResponse> {
    debug!("安装插件: {} (tenant_id={})", request.source, tenant_info.tenant_id);
    
    match plugin_manager.install_plugin(request.into_inner()).await {
        Ok(response) => {
            info!("插件安装完成: plugin_id={}, status={:?}", 
                  response.plugin_id, response.status);
            Ok(HttpResponse::Ok().json(response))
        }
        Err(e) => {
            error!("插件安装失败: {}", e);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "插件安装失败",
                "message": e.to_string()
            })))
        }
    }
}

/// 卸载插件
#[utoipa::path(
    delete,
    path = "/api/v1/plugins/{plugin_id}",
    responses(
        (status = 200, description = "插件卸载成功"),
        (status = 404, description = "插件不存在"),
        (status = 500, description = "服务器内部错误")
    ),
    params(
        ("plugin_id" = String, Path, description = "插件 ID")
    ),
    tag = "plugins"
)]
pub async fn uninstall_plugin(
    plugin_manager: web::Data<Arc<PluginManager>>,
    tenant_info: web::ReqData<TenantInfo>,
    path: web::Path<String>,
) -> ActixResult<HttpResponse> {
    let plugin_id = path.into_inner();
    debug!("卸载插件: {} (tenant_id={})", plugin_id, tenant_info.tenant_id);
    
    match plugin_manager.uninstall_plugin(&plugin_id).await {
        Ok(_) => {
            info!("插件卸载成功: {}", plugin_id);
            Ok(HttpResponse::Ok().json(serde_json::json!({
                "message": "插件卸载成功",
                "plugin_id": plugin_id
            })))
        }
        Err(e) => {
            error!("插件卸载失败: {} - {}", plugin_id, e);
            
            let error_response = match e {
                AiStudioError::NotFound { resource: _ } => HttpResponse::NotFound(),
                _ => HttpResponse::InternalServerError(),
            };
            
            Ok(error_response.json(serde_json::json!({
                "error": "插件卸载失败",
                "message": e.to_string(),
                "plugin_id": plugin_id
            })))
        }
    }
}

/// 获取插件列表
#[utoipa::path(
    get,
    path = "/api/v1/plugins",
    responses(
        (status = 200, description = "获取插件列表成功", body = PluginListResponse),
        (status = 500, description = "服务器内部错误")
    ),
    tag = "plugins"
)]
pub async fn list_plugins(
    plugin_manager: web::Data<Arc<PluginManager>>,
    tenant_info: web::ReqData<TenantInfo>,
) -> ActixResult<HttpResponse> {
    debug!("获取插件列表: tenant_id={}", tenant_info.tenant_id);
    
    match plugin_manager.list_plugins().await {
        Ok(response) => {
            Ok(HttpResponse::Ok().json(response))
        }
        Err(e) => {
            error!("获取插件列表失败: {}", e);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "获取插件列表失败",
                "message": e.to_string()
            })))
        }
    }
}

/// 获取插件信息
#[utoipa::path(
    get,
    path = "/api/v1/plugins/{plugin_id}",
    responses(
        (status = 200, description = "获取插件信息成功", body = PluginInfo),
        (status = 404, description = "插件不存在"),
        (status = 500, description = "服务器内部错误")
    ),
    params(
        ("plugin_id" = String, Path, description = "插件 ID")
    ),
    tag = "plugins"
)]
pub async fn get_plugin_info(
    plugin_manager: web::Data<Arc<PluginManager>>,
    tenant_info: web::ReqData<TenantInfo>,
    path: web::Path<String>,
) -> ActixResult<HttpResponse> {
    let plugin_id = path.into_inner();
    debug!("获取插件信息: {} (tenant_id={})", plugin_id, tenant_info.tenant_id);
    
    match plugin_manager.get_plugin_info(&plugin_id).await {
        Ok(info) => {
            Ok(HttpResponse::Ok().json(info))
        }
        Err(e) => {
            error!("获取插件信息失败: {} - {}", plugin_id, e);
            
            let error_response = match e {
                AiStudioError::NotFound { resource: _ } => HttpResponse::NotFound(),
                _ => HttpResponse::InternalServerError(),
            };
            
            Ok(error_response.json(serde_json::json!({
                "error": "获取插件信息失败",
                "message": e.to_string(),
                "plugin_id": plugin_id
            })))
        }
    }
}

/// 调用插件
#[utoipa::path(
    post,
    path = "/api/v1/plugins/call",
    request_body = PluginCallRequest,
    responses(
        (status = 200, description = "插件调用成功", body = PluginCallResponse),
        (status = 400, description = "请求参数错误"),
        (status = 404, description = "插件不存在"),
        (status = 500, description = "服务器内部错误")
    ),
    tag = "plugins"
)]
pub async fn call_plugin(
    plugin_manager: web::Data<Arc<PluginManager>>,
    tenant_info: web::ReqData<TenantInfo>,
    request: web::Json<PluginCallRequest>,
) -> ActixResult<HttpResponse> {
    debug!("调用插件: {} - {} (tenant_id={})", 
           request.plugin_id, request.method, tenant_info.tenant_id);
    
    let start_time = std::time::Instant::now();
    let call_time = chrono::Utc::now();
    
    // 构建插件上下文
    let context = PluginContext {
        tenant_id: tenant_info.id,
        user_id: tenant_info.user_id,
        session_id: None,
        request_id: Uuid::new_v4(),
        variables: HashMap::new(),
        timestamp: call_time,
    };
    
    match plugin_manager.call_plugin(
        &request.plugin_id,
        &request.method,
        request.parameters.clone(),
        context,
    ).await {
        Ok(result) => {
            let execution_time = start_time.elapsed().as_millis() as u64;
            
            info!("插件调用成功: {} - {} ({}ms)", 
                  request.plugin_id, request.method, execution_time);
            
            let response = PluginCallResponse {
                plugin_id: request.plugin_id.clone(),
                method: request.method.clone(),
                result,
                execution_time_ms: execution_time,
                called_at: call_time,
            };
            
            Ok(HttpResponse::Ok().json(response))
        }
        Err(e) => {
            error!("插件调用失败: {} - {} - {}", 
                   request.plugin_id, request.method, e);
            
            let error_response = match e {
                AiStudioError::NotFound { resource: _ } => HttpResponse::NotFound(),
                AiStudioError::Validation { field: _, message: _ } => HttpResponse::BadRequest(),
                _ => HttpResponse::InternalServerError(),
            };
            
            Ok(error_response.json(serde_json::json!({
                "error": "插件调用失败",
                "message": e.to_string(),
                "plugin_id": request.plugin_id,
                "method": request.method
            })))
        }
    }
}

/// 控制插件状态
#[utoipa::path(
    post,
    path = "/api/v1/plugins/{plugin_id}/control",
    request_body = PluginControlRequest,
    responses(
        (status = 200, description = "插件控制成功"),
        (status = 400, description = "请求参数错误"),
        (status = 404, description = "插件不存在"),
        (status = 500, description = "服务器内部错误")
    ),
    params(
        ("plugin_id" = String, Path, description = "插件 ID")
    ),
    tag = "plugins"
)]
pub async fn control_plugin(
    plugin_manager: web::Data<Arc<PluginManager>>,
    tenant_info: web::ReqData<TenantInfo>,
    path: web::Path<String>,
    request: web::Json<PluginControlRequest>,
) -> ActixResult<HttpResponse> {
    let plugin_id = path.into_inner();
    debug!("控制插件: {} - {:?} (tenant_id={})", 
           plugin_id, request.action, tenant_info.tenant_id);
    
    let result = match request.action {
        PluginAction::Start => plugin_manager.start_plugin(&plugin_id).await,
        PluginAction::Stop => plugin_manager.stop_plugin(&plugin_id).await,
        PluginAction::Restart => plugin_manager.restart_plugin(&plugin_id).await,
        PluginAction::Reload => {
            // TODO: 实现插件重新加载
            Err(AiStudioError::internal("插件重新加载暂未实现"))
        }
    };
    
    match result {
        Ok(_) => {
            info!("插件控制成功: {} - {:?}", plugin_id, request.action);
            Ok(HttpResponse::Ok().json(serde_json::json!({
                "message": format!("插件{}成功", match request.action {
                    PluginAction::Start => "启动",
                    PluginAction::Stop => "停止",
                    PluginAction::Restart => "重启",
                    PluginAction::Reload => "重新加载",
                }),
                "plugin_id": plugin_id,
                "action": request.action
            })))
        }
        Err(e) => {
            error!("插件控制失败: {} - {:?} - {}", plugin_id, request.action, e);
            
            let error_response = match e {
                AiStudioError::NotFound { resource: _ } => HttpResponse::NotFound(),
                AiStudioError::Validation { field: _, message: _ } => HttpResponse::BadRequest(),
                _ => HttpResponse::InternalServerError(),
            };
            
            Ok(error_response.json(serde_json::json!({
                "error": "插件控制失败",
                "message": e.to_string(),
                "plugin_id": plugin_id,
                "action": request.action
            })))
        }
    }
}

/// 更新插件配置
#[utoipa::path(
    put,
    path = "/api/v1/plugins/{plugin_id}/config",
    request_body = UpdatePluginConfigRequest,
    responses(
        (status = 200, description = "插件配置更新成功"),
        (status = 400, description = "请求参数错误"),
        (status = 404, description = "插件不存在"),
        (status = 500, description = "服务器内部错误")
    ),
    params(
        ("plugin_id" = String, Path, description = "插件 ID")
    ),
    tag = "plugins"
)]
pub async fn update_plugin_config(
    plugin_manager: web::Data<Arc<PluginManager>>,
    tenant_info: web::ReqData<TenantInfo>,
    path: web::Path<String>,
    request: web::Json<UpdatePluginConfigRequest>,
) -> ActixResult<HttpResponse> {
    let plugin_id = path.into_inner();
    debug!("更新插件配置: {} (tenant_id={})", plugin_id, tenant_info.tenant_id);
    
    // 构建新的配置
    let config = PluginConfig {
        plugin_id: plugin_id.clone(),
        parameters: request.parameters.clone(),
        environment: request.environment.clone().unwrap_or_default(),
        resource_limits: Default::default(),
        security_settings: Default::default(),
    };
    
    match plugin_manager.update_plugin_config(&plugin_id, config).await {
        Ok(_) => {
            info!("插件配置更新成功: {}", plugin_id);
            
            // 如果需要重启插件
            if request.restart_plugin.unwrap_or(false) {
                if let Err(e) = plugin_manager.restart_plugin(&plugin_id).await {
                    tracing::warn!("插件重启失败: {} - {}", plugin_id, e);
                }
            }
            
            Ok(HttpResponse::Ok().json(serde_json::json!({
                "message": "插件配置更新成功",
                "plugin_id": plugin_id,
                "restarted": request.restart_plugin.unwrap_or(false)
            })))
        }
        Err(e) => {
            error!("插件配置更新失败: {} - {}", plugin_id, e);
            
            let error_response = match e {
                AiStudioError::NotFound { resource: _ } => HttpResponse::NotFound(),
                AiStudioError::Validation { field: _, message: _ } => HttpResponse::BadRequest(),
                _ => HttpResponse::InternalServerError(),
            };
            
            Ok(error_response.json(serde_json::json!({
                "error": "插件配置更新失败",
                "message": e.to_string(),
                "plugin_id": plugin_id
            })))
        }
    }
}

/// 搜索插件
#[utoipa::path(
    post,
    path = "/api/v1/plugins/search",
    request_body = PluginSearchQuery,
    responses(
        (status = 200, description = "插件搜索成功", body = PluginSearchResult),
        (status = 500, description = "服务器内部错误")
    ),
    tag = "plugins"
)]
pub async fn search_plugins(
    plugin_manager: web::Data<Arc<PluginManager>>,
    tenant_info: web::ReqData<TenantInfo>,
    request: web::Json<PluginSearchQuery>,
) -> ActixResult<HttpResponse> {
    debug!("搜索插件: query={:?} (tenant_id={})", 
           request.query, tenant_info.tenant_id);
    
    // TODO: 通过插件管理器访问注册表进行搜索
    // 目前返回空结果
    let result = PluginSearchResult {
        plugins: Vec::new(),
        total: 0,
        search_time_ms: 0,
    };
    
    Ok(HttpResponse::Ok().json(result))
}

/// 获取插件统计
#[utoipa::path(
    get,
    path = "/api/v1/plugins/statistics",
    responses(
        (status = 200, description = "获取插件统计成功", body = PluginStatistics),
        (status = 500, description = "服务器内部错误")
    ),
    tag = "plugins"
)]
pub async fn get_plugin_statistics(
    plugin_manager: web::Data<Arc<PluginManager>>,
    tenant_info: web::ReqData<TenantInfo>,
) -> ActixResult<HttpResponse> {
    debug!("获取插件统计: tenant_id={}", tenant_info.tenant_id);
    
    // TODO: 通过插件管理器获取统计信息
    // 目前返回空统计
    let stats = PluginStatistics {
        total_plugins: 0,
        registered_plugins: 0,
        deprecated_plugins: 0,
        disabled_plugins: 0,
        plugins_by_type: HashMap::new(),
        plugins_by_author: HashMap::new(),
    };
    
    Ok(HttpResponse::Ok().json(stats))
}

/// 获取插件日志
#[utoipa::path(
    get,
    path = "/api/v1/plugins/{plugin_id}/logs",
    responses(
        (status = 200, description = "获取插件日志成功"),
        (status = 404, description = "插件不存在"),
        (status = 500, description = "服务器内部错误")
    ),
    params(
        ("plugin_id" = String, Path, description = "插件 ID"),
        ("limit" = Option<usize>, Query, description = "日志条数限制")
    ),
    tag = "plugins"
)]
pub async fn get_plugin_logs(
    plugin_manager: web::Data<Arc<PluginManager>>,
    tenant_info: web::ReqData<TenantInfo>,
    path: web::Path<String>,
    query: web::Query<LogQuery>,
) -> ActixResult<HttpResponse> {
    let plugin_id = path.into_inner();
    debug!("获取插件日志: {} (tenant_id={})", plugin_id, tenant_info.tenant_id);
    
    match plugin_manager.get_plugin_logs(&plugin_id, query.limit).await {
        Ok(logs) => {
            Ok(HttpResponse::Ok().json(serde_json::json!({
                "plugin_id": plugin_id,
                "logs": logs,
                "count": logs.len()
            })))
        }
        Err(e) => {
            error!("获取插件日志失败: {} - {}", plugin_id, e);
            
            let error_response = match e {
                AiStudioError::NotFound { resource: _ } => HttpResponse::NotFound(),
                _ => HttpResponse::InternalServerError(),
            };
            
            Ok(error_response.json(serde_json::json!({
                "error": "获取插件日志失败",
                "message": e.to_string(),
                "plugin_id": plugin_id
            })))
        }
    }
}

/// 清理插件数据
#[utoipa::path(
    post,
    path = "/api/v1/plugins/{plugin_id}/cleanup",
    responses(
        (status = 200, description = "插件数据清理成功"),
        (status = 404, description = "插件不存在"),
        (status = 500, description = "服务器内部错误")
    ),
    params(
        ("plugin_id" = String, Path, description = "插件 ID")
    ),
    tag = "plugins"
)]
pub async fn cleanup_plugin_data(
    plugin_manager: web::Data<Arc<PluginManager>>,
    tenant_info: web::ReqData<TenantInfo>,
    path: web::Path<String>,
) -> ActixResult<HttpResponse> {
    let plugin_id = path.into_inner();
    debug!("清理插件数据: {} (tenant_id={})", plugin_id, tenant_info.tenant_id);
    
    match plugin_manager.cleanup_plugin_data(&plugin_id).await {
        Ok(_) => {
            info!("插件数据清理成功: {}", plugin_id);
            Ok(HttpResponse::Ok().json(serde_json::json!({
                "message": "插件数据清理成功",
                "plugin_id": plugin_id
            })))
        }
        Err(e) => {
            error!("插件数据清理失败: {} - {}", plugin_id, e);
            
            let error_response = match e {
                AiStudioError::NotFound { resource: _ } => HttpResponse::NotFound(),
                _ => HttpResponse::InternalServerError(),
            };
            
            Ok(error_response.json(serde_json::json!({
                "error": "插件数据清理失败",
                "message": e.to_string(),
                "plugin_id": plugin_id
            })))
        }
    }
}

/// 日志查询参数
#[derive(Debug, Deserialize)]
pub struct LogQuery {
    pub limit: Option<usize>,
}

/// 配置插件 API 路由
pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/plugins")
            .route("/install", web::post().to(install_plugin))
            .route("", web::get().to(list_plugins))
            .route("/search", web::post().to(search_plugins))
            .route("/statistics", web::get().to(get_plugin_statistics))
            .route("/call", web::post().to(call_plugin))
            .route("/{plugin_id}", web::get().to(get_plugin_info))
            .route("/{plugin_id}", web::delete().to(uninstall_plugin))
            .route("/{plugin_id}/control", web::post().to(control_plugin))
            .route("/{plugin_id}/config", web::put().to(update_plugin_config))
            .route("/{plugin_id}/logs", web::get().to(get_plugin_logs))
            .route("/{plugin_id}/cleanup", web::post().to(cleanup_plugin_data))
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_plugin_call_request_validation() {
        let request = PluginCallRequest {
            plugin_id: "test-plugin".to_string(),
            method: "test_method".to_string(),
            parameters: {
                let mut params = HashMap::new();
                params.insert("param1".to_string(), serde_json::Value::String("value1".to_string()));
                params
            },
        };
        
        let json = serde_json::to_string(&request).unwrap();
        let deserialized: PluginCallRequest = serde_json::from_str(&json).unwrap();
        
        assert_eq!(request.plugin_id, deserialized.plugin_id);
        assert_eq!(request.method, deserialized.method);
    }
    
    #[test]
    fn test_plugin_action_serialization() {
        let action = PluginAction::Start;
        let json = serde_json::to_string(&action).unwrap();
        assert_eq!(json, "\"start\"");
        
        let deserialized: PluginAction = serde_json::from_str(&json).unwrap();
        assert_eq!(action, deserialized);
    }
}