// API 路由定义
// 定义所有 API 端点的路由配置

use actix_web::{web, HttpResponse, Result as ActixResult};
use utoipa::{OpenApi, ToSchema};

use crate::api::handlers::{self, health, version, tenant, quota, rate_limit, monitoring, auth, knowledge_base, document, qa, agent, tool, workflow};
use crate::api::models::*;
// use crate::api::middleware::{
//     RequestIdMiddleware, ApiVersionMiddleware, RequestLoggingMiddleware,
//     SecurityHeadersMiddleware, ResponseTimeMiddleware, ContentTypeMiddleware,
//     MiddlewareConfig,
// };
use crate::api::responses::HttpResponseBuilder;
use crate::services::tenant::{TenantResponse, TenantStatsResponse, CreateTenantRequest, UpdateTenantRequest};
use crate::services::auth::{LoginRequest, LoginResponse, RegisterRequest, RegisterResponse, RefreshTokenRequest, PasswordResetRequest, PasswordResetConfirmRequest, UserInfo};
use crate::services::quota::{QuotaCheckResult, QuotaUpdateRequest, QuotaStatsResponse};
use crate::api::handlers::rate_limit::RateLimitCheckRequest;
use crate::services::rate_limit::RateLimitPolicy;
use crate::services::monitoring::{SystemHealth};
use crate::services::tenant::TenantInfo;


/// API 文档聚合
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Aionix AI Studio API",
        description = "企业级 AI Studio API 接口文档",
        version = "1.0.0",
        contact(
            name = "Aionix Team",
            url = "https://aionix.ai",
            email = "support@aionix.ai"
        ),
        license(
            name = "MIT",
            url = "https://opensource.org/licenses/MIT"
        )
    ),
    servers(
        (url = "/api/v1", description = "API v1")
    ),
    paths(
        // 健康检查
        health::health_check,
        // 版本信息
        version::get_version,
        // 租户管理
        tenant::create_tenant,
        tenant::get_tenant,
        tenant::list_tenants,
        tenant::update_tenant,
        tenant::delete_tenant,
        tenant::get_tenant_stats,
        tenant::suspend_tenant,
        tenant::activate_tenant,
        // 配额管理
        quota::check_quota,
        quota::update_quota,
        quota::get_quota_usage,
        // 速率限制
        rate_limit::get_rate_limits,
        // rate_limit::update_rate_limit,
        // rate_limit::delete_rate_limit,
        rate_limit::check_rate_limit,
        // 监控
        monitoring::get_system_health,
        monitoring::get_tenant_usage_stats,
        // 认证
        auth::login,
        auth::logout,
        auth::refresh_token,
        auth::register,
        auth::request_password_reset,
        auth::confirm_password_reset,
        auth::get_current_user,
        auth::update_user_profile,
        // 知识库管理
        knowledge_base::create_knowledge_base,
        knowledge_base::list_knowledge_bases,
        knowledge_base::get_knowledge_base,
        knowledge_base::update_knowledge_base,
        knowledge_base::delete_knowledge_base,
        knowledge_base::get_knowledge_base_stats,
        knowledge_base::reindex_knowledge_base,
        // 文档管理
        document::create_document,
        document::upload_document,
        document::list_documents,
        document::get_document,
        document::update_document,
        document::delete_document,
        document::get_document_stats,
        document::reprocess_document,
        // 批量文档操作
        document::batch_document_operation,
        document::batch_import_documents,
        document::batch_export_documents,
        document::get_batch_operation_status,
        // 问答管理
        qa::ask_question,
        qa::ask_question_stream,
        qa::get_session_history,
        qa::submit_feedback,
        qa::get_suggestions,
        // Agent 管理
        agent::create_agent,
        agent::execute_task,
        agent::get_agent_status,
        agent::stop_agent,
        agent::list_agents,
        agent::cleanup_agents,
        // 工具管理
        tool::call_tool,
        tool::list_tools,
        tool::get_tool_metadata,
        tool::update_tool_permissions,
        tool::get_tool_usage_stats,
        tool::get_all_tool_usage_stats,
        tool::reload_tool,
        tool::reload_all_tools,
        // 工作流管理
        workflow::create_workflow,
        workflow::execute_workflow,
        workflow::list_workflows,
        workflow::get_workflow,
        workflow::get_execution_status,
        workflow::cancel_execution,
        workflow::get_execution_history,
        workflow::publish_workflow,
    ),
    components(
        schemas(
            // 通用响应
            crate::api::responses::ApiError,
            
            // 版本信息
            ApiVersion,
            
            // 认证相关
            LoginRequest,
            LoginResponse,
            RegisterRequest,
            RegisterResponse,
            RefreshTokenRequest,
            PasswordResetRequest,
            PasswordResetConfirmRequest,
            UserInfo,
            TenantInfo,
            
            // 租户相关
            CreateTenantRequest,
            UpdateTenantRequest,
            TenantResponse,
            TenantStatsResponse,
            
            // 配额相关
            QuotaCheckResult,
            QuotaUpdateRequest,
            QuotaStatsResponse,
            
            // 速率限制相关
            RateLimitPolicy,
            RateLimitCheckRequest,
            
            // 监控相关
            SystemHealth,
            
            // 分页相关
            PaginationQuery,
            PaginationInfo,
            
            // 知识库相关
            knowledge_base::CreateKnowledgeBaseRequest,
            knowledge_base::UpdateKnowledgeBaseRequest,
            knowledge_base::KnowledgeBaseResponse,
            knowledge_base::KnowledgeBaseStats,
            knowledge_base::KnowledgeBaseSearchQuery,
            crate::db::entities::knowledge_base::KnowledgeBaseType,
            crate::db::entities::knowledge_base::KnowledgeBaseStatus,
            crate::db::entities::knowledge_base::KnowledgeBaseConfig,
            crate::db::entities::knowledge_base::KnowledgeBaseMetadata,
            
            // 文档相关
            document::CreateDocumentRequest,
            document::UpdateDocumentRequest,
            document::DocumentResponse,
            document::DocumentStats,
            document::DocumentSearchQuery,
            document::DocumentUploadResponse,
            crate::db::entities::document::DocumentType,
            crate::db::entities::document::DocumentStatus,
            crate::db::entities::document::DocumentMetadata,
            crate::db::entities::document::DocumentProcessingConfig,
            
            // 批量操作相关
            document::BatchDocumentOperation,
            document::BatchDocumentRequest,
            document::BatchDocumentResponse,
            document::BatchDocumentError,
            document::BatchImportRequest,
            document::BatchImportOptions,
            document::BatchImportResponse,
            document::BatchExportRequest,
            document::BatchExportOptions,
            document::BatchExportResponse,
            document::ExportFormat,
            
            // 问答相关
            qa::QaRequest,
            qa::QaResponse,
            qa::QaSource,
            qa::QaChunk,
            qa::QaStats,
            qa::SessionMessage,
            qa::MessageType,
            qa::QaFeedbackRequest,
            qa::FeedbackType,
            qa::QaSuggestionsRequest,
            qa::QaSuggestionsResponse,
            qa::SessionHistoryQuery,
            
            // Agent 相关
            agent::CreateAgentRequest,
            agent::CreateAgentResponse,
            agent::ExecuteTaskRequest,
            agent::ExecuteTaskResponse,
            agent::AgentStatusResponse,
            agent::AgentTaskInfo,
            agent::ExecutionStats,
            agent::ListAgentsResponse,
            agent::AgentInfo,
            crate::ai::agent_runtime::ReasoningStrategy,
            crate::ai::agent_runtime::AgentState,
            crate::ai::agent_runtime::TaskPriority,
            crate::ai::agent_runtime::TaskStatus,
            
            // 工具相关
            tool::ToolCallRequest,
            tool::ToolCallResponse,
            tool::UpdateToolPermissionsRequest,
            tool::ToolListQuery,
            tool::ReloadToolRequest,
            tool::ReloadToolResponse,
            crate::ai::tool_manager::ToolUsageStats,
            crate::ai::tool_manager::ToolListResponse,
            crate::ai::tool_manager::PermissionLevel,
            crate::ai::agent_runtime::ToolMetadata,
            
            // 工作流相关
            workflow::CreateWorkflowRequest,
            workflow::CreateWorkflowResponse,
            workflow::ExecuteWorkflowRequest,
            workflow::ExecuteWorkflowResponse,
            workflow::WorkflowListQuery,
            workflow::WorkflowListResponse,
            workflow::WorkflowSummary,
            workflow::WorkflowExecutionStats,
            workflow::ExecutionHistoryQuery,
            workflow::ExecutionHistoryResponse,
            workflow::ExecutionSummary,
            workflow::StepStats,
            workflow::PaginationInfo,
            workflow::ValidationSummary,
            crate::ai::workflow_engine::WorkflowDefinition,
            crate::ai::workflow_engine::WorkflowStatus,
            crate::ai::workflow_executor::WorkflowExecution,
        )
    ),
    tags(
        (name = "health", description = "健康检查端点"),
        (name = "version", description = "版本信息端点"),
        (name = "auth", description = "认证相关端点"),
        (name = "tenant", description = "租户管理端点"),
        (name = "quota", description = "配额管理端点"),
        (name = "rate-limit", description = "速率限制端点"),
        (name = "monitoring", description = "监控端点"),
        (name = "knowledge-bases", description = "知识库管理端点"),
        (name = "documents", description = "文档管理端点"),
        (name = "qa", description = "智能问答端点"),
        (name = "agents", description = "Agent 管理端点"),
        (name = "tools", description = "工具管理端点"),
        (name = "workflows", description = "工作流管理端点"),
    )
)]
pub struct ApiDoc;

/// 根路径处理器
async fn api_root() -> ActixResult<HttpResponse> {
    let info = serde_json::json!({
        "name": "Aionix AI Studio API",
        "version": env!("CARGO_PKG_VERSION"),
        "description": "企业级 AI Studio API 接口",
        "documentation": "/api/v1/docs",
        "health": "/api/v1/health",
        "version": "/api/v1/version",
        "timestamp": chrono::Utc::now(),
        "endpoints": {
            "health": {
                "simple": "/api/v1/health",
                "detailed": "/api/v1/health/detailed",
                "ready": "/api/v1/ready",
                "live": "/api/v1/live"
            },
            "version": {
                "info": "/api/v1/version",
                "build": "/api/v1/version/build-info",
                "spec": "/api/v1/version/spec"
            },
            "docs": {
                "openapi": "/api/v1/openapi.json",
                "swagger": "/api/v1/docs"
            }
        }
    });

    HttpResponseBuilder::ok(info)
}

/// 配置 API 路由
pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            .service(
                web::scope("/v1")
                    // API 根路径
                    .route("", web::get().to(api_root))
                    // 健康检查路由
                    .configure(health::configure_health_routes)
                    // 版本信息路由
                    .configure(version::configure_version_routes)
                    // 租户管理路由
                    .configure(tenant::configure_tenant_routes)
                    // 配额管理路由
                    .configure(quota::configure_quota_routes)
                    // 限流管理路由
                    .configure(rate_limit::configure_rate_limit_routes)
                    // 监控管理路由
                    .configure(monitoring::configure_monitoring_routes)
                    // 知识库管理路由
                    .configure(knowledge_base::configure_routes)
                    // 文档管理路由
                    .configure(document::configure_routes)
                    // 问答管理路由
                    .configure(qa::configure_routes)
                    // Agent 管理路由
                    .configure(agent::configure_routes)
                    // 工具管理路由
                    .configure(tool::configure_routes)
                    // 工作流管理路由
                    .configure(workflow::configure_routes)
                    // OpenAPI JSON 端点
                    .route("/openapi.json", web::get().to(get_openapi_spec))
                    // 未来的路由将在这里添加：
                    // - 租户管理 (/tenants)
                    // - 用户管理 (/users)
                    // - 认证 (/auth)
                    // - 知识库 (/knowledge-bases)
                    // - 文档 (/documents)
                    // - 问答 (/qa)
                    // - Agent (/agents)
                    // - 工作流 (/workflows)
            )
    );
}

/// 获取 OpenAPI 规范
async fn get_openapi_spec() -> ActixResult<HttpResponse> {
    let openapi = ApiDoc::openapi();
    // 直接返回 OpenAPI 规范，不包装在响应格式中
    Ok(HttpResponse::Ok()
        .content_type("application/json")
        .json(openapi))
}

/// 配置 Swagger UI
pub fn configure_swagger_ui(cfg: &mut web::ServiceConfig) {
    cfg.service(
        utoipa_swagger_ui::SwaggerUi::new("/api/v1/docs/{_:.*}")
            .url("/api/v1/openapi.json", ApiDoc::openapi())
    )
    // 添加根级别的 docs 路由重定向
    .service(
        utoipa_swagger_ui::SwaggerUi::new("/docs/{_:.*}")
            .url("/api/v1/openapi.json", ApiDoc::openapi())
    );
}

/// API 路由配置辅助函数
pub struct ApiRouteConfig;

impl ApiRouteConfig {
    /// 配置所有 API 路由
    pub fn configure_all(cfg: &mut web::ServiceConfig) {
        // 配置主要路由
        configure_routes(cfg);
        
        // 配置 Swagger UI
        configure_swagger_ui(cfg);
    }

    /// 配置开发环境路由
    pub fn configure_dev(cfg: &mut web::ServiceConfig) {
        Self::configure_all(cfg);
        
        // 开发环境特有的路由
        cfg.service(
            web::scope("/dev")
                .route("/test", web::get().to(dev_test_endpoint))
                .route("/debug", web::get().to(dev_debug_endpoint))
        );
    }

    /// 配置生产环境路由
    pub fn configure_prod(cfg: &mut web::ServiceConfig) {
        Self::configure_all(cfg);
        
        // 生产环境可能需要额外的安全中间件
        // 这里可以添加生产环境特有的配置
    }
}

/// 开发测试端点
async fn dev_test_endpoint() -> ActixResult<HttpResponse> {
    HttpResponseBuilder::ok(serde_json::json!({
        "message": "开发测试端点",
        "timestamp": chrono::Utc::now(),
        "environment": "development"
    }))
}

/// 开发调试端点
async fn dev_debug_endpoint() -> ActixResult<HttpResponse> {
    HttpResponseBuilder::ok(serde_json::json!({
        "debug_info": {
            "version": env!("CARGO_PKG_VERSION"),
            "build_profile": if cfg!(debug_assertions) { "debug" } else { "release" },
            "features": crate::api::handlers::version::get_enabled_features(),
            "memory_usage": "模拟内存使用信息",
            "active_connections": 42,
            "uptime": "模拟运行时间"
        },
        "timestamp": chrono::Utc::now()
    }))
}
