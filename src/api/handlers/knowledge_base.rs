// 知识库管理 API 处理器

use actix_web::{web, HttpResponse, Result as ActixResult};
use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait, QueryOrder, PaginatorTrait, QuerySelect};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use tracing::{info, warn, error, debug};

use crate::api::models::{PaginationQuery, PaginatedResponse, PaginationInfo};
use crate::api::responses::{ApiResponse, ApiError, SuccessResponse, ErrorResponse, HttpResponseBuilder};
use crate::api::extractors::{TenantContext, UserContext};
use crate::db::entities::{knowledge_base, prelude::*};
use crate::errors::AiStudioError;
use crate::services::knowledge_base::{KnowledgeBaseService, KnowledgeBaseServiceFactory};

/// 知识库创建请求
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateKnowledgeBaseRequest {
    /// 知识库名称
    pub name: String,
    /// 知识库描述
    pub description: Option<String>,
    /// 知识库类型
    pub kb_type: knowledge_base::KnowledgeBaseType,
    /// 知识库配置
    pub config: Option<knowledge_base::KnowledgeBaseConfig>,
    /// 知识库元数据
    pub metadata: Option<knowledge_base::KnowledgeBaseMetadata>,
    /// 嵌入模型名称
    pub embedding_model: Option<String>,
}

/// 知识库更新请求
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct UpdateKnowledgeBaseRequest {
    /// 知识库名称
    pub name: Option<String>,
    /// 知识库描述
    pub description: Option<String>,
    /// 知识库类型
    pub kb_type: Option<knowledge_base::KnowledgeBaseType>,
    /// 知识库状态
    pub status: Option<knowledge_base::KnowledgeBaseStatus>,
    /// 知识库配置
    pub config: Option<knowledge_base::KnowledgeBaseConfig>,
    /// 知识库元数据
    pub metadata: Option<knowledge_base::KnowledgeBaseMetadata>,
    /// 嵌入模型名称
    pub embedding_model: Option<String>,
}

/// 知识库响应
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct KnowledgeBaseResponse {
    /// 知识库 ID
    pub id: Uuid,
    /// 租户 ID
    pub tenant_id: Uuid,
    /// 知识库名称
    pub name: String,
    /// 知识库描述
    pub description: Option<String>,
    /// 知识库类型
    pub kb_type: knowledge_base::KnowledgeBaseType,
    /// 知识库状态
    pub status: knowledge_base::KnowledgeBaseStatus,
    /// 知识库配置
    pub config: knowledge_base::KnowledgeBaseConfig,
    /// 知识库元数据
    pub metadata: knowledge_base::KnowledgeBaseMetadata,
    /// 文档数量
    pub document_count: i32,
    /// 总文档块数量
    pub chunk_count: i32,
    /// 总存储大小（字节）
    pub total_size_bytes: i64,
    /// 格式化的存储大小
    pub formatted_size: String,
    /// 向量维度
    pub vector_dimension: i32,
    /// 嵌入模型名称
    pub embedding_model: String,
    /// 最后索引时间
    pub last_indexed_at: Option<DateTime<Utc>>,
    /// 是否需要重新索引
    pub needs_reindexing: bool,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

/// 知识库统计信息
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct KnowledgeBaseStats {
    /// 知识库 ID
    pub id: Uuid,
    /// 知识库名称
    pub name: String,
    /// 文档数量
    pub document_count: i32,
    /// 文档块数量
    pub chunk_count: i32,
    /// 总存储大小
    pub total_size_bytes: i64,
    /// 格式化的存储大小
    pub formatted_size: String,
    /// 平均文档大小
    pub average_document_size: f64,
    /// 平均块大小
    pub average_chunk_size: f64,
    /// 最后索引时间
    pub last_indexed_at: Option<DateTime<Utc>>,
    /// 创建时间
    pub created_at: DateTime<Utc>,
}

/// 知识库搜索查询
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct KnowledgeBaseSearchQuery {
    /// 搜索关键词
    pub q: Option<String>,
    /// 知识库类型过滤
    pub kb_type: Option<knowledge_base::KnowledgeBaseType>,
    /// 状态过滤
    pub status: Option<knowledge_base::KnowledgeBaseStatus>,
    /// 标签过滤
    pub tags: Option<Vec<String>>,
    /// 分页参数
    #[serde(flatten)]
    pub pagination: PaginationQuery,
}

impl From<knowledge_base::Model> for KnowledgeBaseResponse {
    fn from(model: knowledge_base::Model) -> Self {
        let config = model.get_config().unwrap_or_default();
        let metadata = model.get_metadata().unwrap_or_default();
        let formatted_size = model.formatted_size();
        let needs_reindexing = model.needs_reindexing();
        
        Self {
            id: model.id,
            tenant_id: model.tenant_id,
            name: model.name,
            description: model.description,
            kb_type: model.kb_type,
            status: model.status,
            config,
            metadata,
            document_count: model.document_count,
            chunk_count: model.chunk_count,
            total_size_bytes: model.total_size_bytes,
            formatted_size,
            vector_dimension: model.vector_dimension,
            embedding_model: model.embedding_model,
            last_indexed_at: model.last_indexed_at.map(|dt| dt.with_timezone(&Utc)),
            needs_reindexing,
            created_at: model.created_at.with_timezone(&Utc),
            updated_at: model.updated_at.with_timezone(&Utc),
        }
    }
}

impl From<knowledge_base::Model> for KnowledgeBaseStats {
    fn from(model: knowledge_base::Model) -> Self {
        let formatted_size = model.formatted_size();
        let average_document_size = model.average_document_size();
        let average_chunk_size = model.average_chunk_size();
        
        Self {
            id: model.id,
            name: model.name,
            document_count: model.document_count,
            chunk_count: model.chunk_count,
            total_size_bytes: model.total_size_bytes,
            formatted_size,
            average_document_size,
            average_chunk_size,
            last_indexed_at: model.last_indexed_at.map(|dt| dt.with_timezone(&Utc)),
            created_at: model.created_at.with_timezone(&Utc),
        }
    }
}

/// 创建知识库
#[utoipa::path(
    post,
    path = "/api/v1/knowledge-bases",
    request_body = CreateKnowledgeBaseRequest,
    responses(
        (status = 201, description = "知识库创建成功", body = KnowledgeBaseResponse),
        (status = 400, description = "请求参数错误", body = ApiError),
        (status = 401, description = "未授权", body = ApiError),
        (status = 403, description = "权限不足", body = ApiError),
        (status = 409, description = "知识库名称已存在", body = ApiError),
        (status = 500, description = "服务器内部错误", body = ApiError)
    ),
    tag = "knowledge-bases",
    security(
        ("bearer_auth" = []),
        ("api_key" = [])
    )
)]
pub async fn create_knowledge_base(
    db: web::Data<DatabaseConnection>,
    tenant_ctx: TenantContext,
    _user_ctx: UserContext,
    req: web::Json<CreateKnowledgeBaseRequest>,
) -> ActixResult<HttpResponse> {
    info!("创建知识库请求: 租户={}, 名称={}", tenant_ctx.tenant.id, req.name);
    
    // 检查知识库名称是否已存在
    let existing = KnowledgeBase::find()
        .filter(knowledge_base::Column::TenantId.eq(tenant_ctx.tenant.id))
        .filter(knowledge_base::Column::Name.eq(&req.name))
        .one(db.as_ref())
        .await
        .map_err(|e| {
            error!("查询知识库失败: {}", e);
            ErrorResponse::internal_server_error::<()>("查询知识库失败")
        })?;
    
    if existing.is_some() {
        warn!("知识库名称已存在: {}", req.name);
        return Ok(ErrorResponse::conflict::<()>("知识库名称已存在").into_http_response()?);
    }
    
    // 准备配置和元数据
    let config = req.config.clone().unwrap_or_default();
    let metadata = req.metadata.clone().unwrap_or_default();
    let embedding_model = req.embedding_model.clone().unwrap_or_else(|| {
        config.vectorization_settings.model_name.clone()
    });
    
    // 创建知识库
    let kb_id = Uuid::new_v4();
    let now = Utc::now().with_timezone(&chrono::FixedOffset::east_opt(8 * 3600).unwrap());
    
    let new_kb = knowledge_base::ActiveModel {
        id: sea_orm::Set(kb_id),
        tenant_id: sea_orm::Set(tenant_ctx.tenant.id),
        name: sea_orm::Set(req.name.clone()),
        description: sea_orm::Set(req.description.clone()),
        kb_type: sea_orm::Set(req.kb_type.clone()),
        status: sea_orm::Set(knowledge_base::KnowledgeBaseStatus::Active),
        config: sea_orm::Set(serde_json::to_value(&config).unwrap().into()),
        metadata: sea_orm::Set(serde_json::to_value(&metadata).unwrap().into()),
        document_count: sea_orm::Set(0),
        chunk_count: sea_orm::Set(0),
        total_size_bytes: sea_orm::Set(0),
        vector_dimension: sea_orm::Set(config.vectorization_settings.dimension as i32),
        embedding_model: sea_orm::Set(embedding_model),
        last_indexed_at: sea_orm::Set(None),
        created_at: sea_orm::Set(now),
        updated_at: sea_orm::Set(now),
    };
    
    let kb = KnowledgeBase::insert(new_kb)
        .exec_with_returning(db.as_ref())
        .await
        .map_err(|e| {
            error!("创建知识库失败: {}", e);
            ErrorResponse::internal_server_error::<()>("创建知识库失败")
        })?;
    
    info!("知识库创建成功: id={}, 名称={}", kb.id, kb.name);
    
    let response = KnowledgeBaseResponse::from(kb);
    Ok(SuccessResponse::created(response).into_http_response()?)
}

/// 获取知识库列表
#[utoipa::path(
    get,
    path = "/api/v1/knowledge-bases",
    params(KnowledgeBaseSearchQuery),
    responses(
        (status = 200, description = "获取知识库列表成功", body = PaginatedResponse<KnowledgeBaseResponse>),
        (status = 400, description = "请求参数错误", body = ApiError),
        (status = 401, description = "未授权", body = ApiError),
        (status = 500, description = "服务器内部错误", body = ApiError)
    ),
    tag = "knowledge-bases",
    security(
        ("bearer_auth" = []),
        ("api_key" = [])
    )
)]
pub async fn list_knowledge_bases(
    db: web::Data<DatabaseConnection>,
    tenant_ctx: TenantContext,
    _user_ctx: UserContext,
    query: web::Query<KnowledgeBaseSearchQuery>,
) -> ActixResult<HttpResponse> {
    debug!("获取知识库列表: 租户={}", tenant_ctx.tenant.id);
    
    let mut query_params = query.into_inner();
    query_params.pagination.validate();
    
    // 构建查询
    let mut select = KnowledgeBase::find()
        .filter(knowledge_base::Column::TenantId.eq(tenant_ctx.tenant.id));
    
    // 添加搜索条件
    if let Some(q) = &query_params.q {
        select = select.filter(
            knowledge_base::Column::Name.contains(q)
                .or(knowledge_base::Column::Description.contains(q))
        );
    }
    
    if let Some(kb_type) = &query_params.kb_type {
        select = select.filter(knowledge_base::Column::KbType.eq(kb_type.clone()));
    }
    
    if let Some(status) = &query_params.status {
        select = select.filter(knowledge_base::Column::Status.eq(status.clone()));
    }
    
    // 添加排序
    let sort_column = query_params.pagination.sort_by.as_deref().unwrap_or("created_at");
    select = match sort_column {
        "name" => match query_params.pagination.sort_order {
            crate::api::models::SortOrder::Asc => select.order_by_asc(knowledge_base::Column::Name),
            crate::api::models::SortOrder::Desc => select.order_by_desc(knowledge_base::Column::Name),
        },
        "updated_at" => match query_params.pagination.sort_order {
            crate::api::models::SortOrder::Asc => select.order_by_asc(knowledge_base::Column::UpdatedAt),
            crate::api::models::SortOrder::Desc => select.order_by_desc(knowledge_base::Column::UpdatedAt),
        },
        "document_count" => match query_params.pagination.sort_order {
            crate::api::models::SortOrder::Asc => select.order_by_asc(knowledge_base::Column::DocumentCount),
            crate::api::models::SortOrder::Desc => select.order_by_desc(knowledge_base::Column::DocumentCount),
        },
        _ => match query_params.pagination.sort_order {
            crate::api::models::SortOrder::Asc => select.order_by_asc(knowledge_base::Column::CreatedAt),
            crate::api::models::SortOrder::Desc => select.order_by_desc(knowledge_base::Column::CreatedAt),
        },
    };
    
    // 执行分页查询
    let paginator = select.paginate(db.as_ref(), query_params.pagination.page_size as u64);
    let total = paginator.num_items().await.map_err(|e| {
        error!("查询知识库总数失败: {}", e);
        ErrorResponse::internal_server_error::<()>("查询知识库失败")
    })?;
    
    let knowledge_bases = paginator
        .fetch_page((query_params.pagination.page - 1) as u64)
        .await
        .map_err(|e| {
            error!("查询知识库列表失败: {}", e);
            ErrorResponse::internal_server_error::<()>("查询知识库失败")
        })?;
    
    let responses: Vec<KnowledgeBaseResponse> = knowledge_bases
        .into_iter()
        .map(KnowledgeBaseResponse::from)
        .collect();
    
    let pagination = PaginationInfo::new(
        query_params.pagination.page,
        query_params.pagination.page_size,
        total,
    );
    
    let response = PaginatedResponse::new(responses, pagination);
    Ok(SuccessResponse::ok(response).into_http_response()?)
}

/// 获取知识库详情
#[utoipa::path(
    get,
    path = "/api/v1/knowledge-bases/{id}",
    params(
        ("id" = Uuid, Path, description = "知识库 ID")
    ),
    responses(
        (status = 200, description = "获取知识库详情成功", body = KnowledgeBaseResponse),
        (status = 401, description = "未授权", body = ApiError),
        (status = 403, description = "权限不足", body = ApiError),
        (status = 404, description = "知识库不存在", body = ApiError),
        (status = 500, description = "服务器内部错误", body = ApiError)
    ),
    tag = "knowledge-bases",
    security(
        ("bearer_auth" = []),
        ("api_key" = [])
    )
)]
pub async fn get_knowledge_base(
    db: web::Data<DatabaseConnection>,
    tenant_ctx: TenantContext,
    user_ctx: UserContext,
    path: web::Path<Uuid>,
) -> ActixResult<HttpResponse> {
    let kb_id = path.into_inner();
    debug!("获取知识库详情: id={}, 租户={}", kb_id, tenant_ctx.tenant.id);
    
    let kb = KnowledgeBase::find_by_id(kb_id)
        .filter(knowledge_base::Column::TenantId.eq(tenant_ctx.tenant.id))
        .one(db.as_ref())
        .await
        .map_err(|e| {
            error!("查询知识库失败: {}", e);
            ErrorResponse::internal_server_error::<()>("查询知识库失败")
        })?;
    
    let kb = match kb {
        Some(kb) => kb,
        None => {
            warn!("知识库不存在: id={}", kb_id);
            return Ok(ErrorResponse::not_found::<()>("知识库不存在").into_http_response()?);
        }
    };
    
    // 检查访问权限
    if !kb.has_access(&user_ctx.user.role, &user_ctx.user.id.to_string()).unwrap_or(false) {
        warn!("用户无权访问知识库: user={}, kb={}", user_ctx.user.id, kb_id);
        return Ok(ErrorResponse::forbidden::<()>("无权访问此知识库").into_http_response()?);
    }
    
    let response = KnowledgeBaseResponse::from(kb);
    Ok(SuccessResponse::ok(response).into_http_response()?)
}

/// 更新知识库
#[utoipa::path(
    put,
    path = "/api/v1/knowledge-bases/{id}",
    params(
        ("id" = Uuid, Path, description = "知识库 ID")
    ),
    request_body = UpdateKnowledgeBaseRequest,
    responses(
        (status = 200, description = "更新知识库成功", body = KnowledgeBaseResponse),
        (status = 400, description = "请求参数错误", body = ApiError),
        (status = 401, description = "未授权", body = ApiError),
        (status = 403, description = "权限不足", body = ApiError),
        (status = 404, description = "知识库不存在", body = ApiError),
        (status = 409, description = "知识库名称已存在", body = ApiError),
        (status = 500, description = "服务器内部错误", body = ApiError)
    ),
    tag = "knowledge-bases",
    security(
        ("bearer_auth" = []),
        ("api_key" = [])
    )
)]
pub async fn update_knowledge_base(
    db: web::Data<DatabaseConnection>,
    tenant_ctx: TenantContext,
    user_ctx: UserContext,
    path: web::Path<Uuid>,
    req: web::Json<UpdateKnowledgeBaseRequest>,
) -> ActixResult<HttpResponse> {
    let kb_id = path.into_inner();
    info!("更新知识库请求: id={}, 租户={}", kb_id, tenant_ctx.tenant.id);
    
    // 查找知识库
    let kb = KnowledgeBase::find_by_id(kb_id)
        .filter(knowledge_base::Column::TenantId.eq(tenant_ctx.tenant.id))
        .one(db.as_ref())
        .await
        .map_err(|e| {
            error!("查询知识库失败: {}", e);
            ErrorResponse::internal_server_error::<()>("查询知识库失败")
        })?;
    
    let kb = match kb {
        Some(kb) => kb,
        None => {
            warn!("知识库不存在: id={}", kb_id);
            return Ok(ErrorResponse::not_found::<()>("知识库不存在").into_http_response()?);
        }
    };
    
    // 检查访问权限
    if !kb.has_access(&user_ctx.user.role, &user_ctx.user.id.to_string()).unwrap_or(false) {
        warn!("用户无权修改知识库: user={}, kb={}", user_ctx.user.id, kb_id);
        return Ok(ErrorResponse::forbidden::<()>("无权修改此知识库").into_http_response()?);
    }
    
    // 检查名称冲突
    if let Some(new_name) = &req.name {
        if new_name != &kb.name {
            let existing = KnowledgeBase::find()
                .filter(knowledge_base::Column::TenantId.eq(tenant_ctx.tenant.id))
                .filter(knowledge_base::Column::Name.eq(new_name))
                .filter(knowledge_base::Column::Id.ne(kb_id))
                .one(db.as_ref())
                .await
                .map_err(|e| {
                    error!("查询知识库名称冲突失败: {}", e);
                    ErrorResponse::internal_server_error::<()>("查询知识库失败")
                })?;
            
            if existing.is_some() {
                warn!("知识库名称已存在: {}", new_name);
                return Ok(ErrorResponse::conflict::<()>("知识库名称已存在").into_http_response()?);
            }
        }
    }
    
    // 准备更新数据
    let mut active_model: knowledge_base::ActiveModel = kb.into();
    let now = Utc::now().with_timezone(&chrono::FixedOffset::east_opt(8 * 3600).unwrap());
    
    if let Some(name) = &req.name {
        active_model.name = sea_orm::Set(name.clone());
    }
    
    if let Some(description) = &req.description {
        active_model.description = sea_orm::Set(Some(description.clone()));
    }
    
    if let Some(kb_type) = &req.kb_type {
        active_model.kb_type = sea_orm::Set(kb_type.clone());
    }
    
    if let Some(status) = &req.status {
        active_model.status = sea_orm::Set(status.clone());
    }
    
    if let Some(config) = &req.config {
        active_model.config = sea_orm::Set(serde_json::to_value(config).unwrap().into());
        active_model.vector_dimension = sea_orm::Set(config.vectorization_settings.dimension as i32);
    }
    
    if let Some(metadata) = &req.metadata {
        active_model.metadata = sea_orm::Set(serde_json::to_value(metadata).unwrap().into());
    }
    
    if let Some(embedding_model) = &req.embedding_model {
        active_model.embedding_model = sea_orm::Set(embedding_model.clone());
    }
    
    active_model.updated_at = sea_orm::Set(now);
    
    // 执行更新
    let updated_kb = active_model.update(db.as_ref()).await.map_err(|e| {
        error!("更新知识库失败: {}", e);
        ErrorResponse::internal_server_error::<()>("更新知识库失败")
    })?;
    
    info!("知识库更新成功: id={}, 名称={}", updated_kb.id, updated_kb.name);
    
    let response = KnowledgeBaseResponse::from(updated_kb);
    Ok(SuccessResponse::ok(response).into_http_response()?)
}

/// 删除知识库
#[utoipa::path(
    delete,
    path = "/api/v1/knowledge-bases/{id}",
    params(
        ("id" = Uuid, Path, description = "知识库 ID")
    ),
    responses(
        (status = 204, description = "删除知识库成功"),
        (status = 401, description = "未授权", body = ApiError),
        (status = 403, description = "权限不足", body = ApiError),
        (status = 404, description = "知识库不存在", body = ApiError),
        (status = 409, description = "知识库包含文档，无法删除", body = ApiError),
        (status = 500, description = "服务器内部错误", body = ApiError)
    ),
    tag = "knowledge-bases",
    security(
        ("bearer_auth" = []),
        ("api_key" = [])
    )
)]
pub async fn delete_knowledge_base(
    db: web::Data<DatabaseConnection>,
    tenant_ctx: TenantContext,
    user_ctx: UserContext,
    path: web::Path<Uuid>,
) -> ActixResult<HttpResponse> {
    let kb_id = path.into_inner();
    info!("删除知识库请求: id={}, 租户={}", kb_id, tenant_ctx.tenant.id);
    
    // 查找知识库
    let kb = KnowledgeBase::find_by_id(kb_id)
        .filter(knowledge_base::Column::TenantId.eq(tenant_ctx.tenant.id))
        .one(db.as_ref())
        .await
        .map_err(|e| {
            error!("查询知识库失败: {}", e);
            ErrorResponse::internal_server_error::<()>("查询知识库失败")
        })?;
    
    let kb = match kb {
        Some(kb) => kb,
        None => {
            warn!("知识库不存在: id={}", kb_id);
            return Ok(ErrorResponse::not_found::<()>("知识库不存在").into_http_response()?);
        }
    };
    
    // 检查访问权限
    if !kb.has_access(&user_ctx.user.role, &user_ctx.user.id.to_string()).unwrap_or(false) {
        warn!("用户无权删除知识库: user={}, kb={}", user_ctx.user.id, kb_id);
        return Ok(ErrorResponse::forbidden::<()>("无权删除此知识库").into_http_response()?);
    }
    
    // 检查是否包含文档
    if kb.document_count > 0 {
        warn!("知识库包含文档，无法删除: id={}, 文档数={}", kb_id, kb.document_count);
        return Ok(ErrorResponse::conflict::<()>("知识库包含文档，请先删除所有文档").into_http_response()?);
    }
    
    // 执行删除
    KnowledgeBase::delete_by_id(kb_id)
        .exec(db.as_ref())
        .await
        .map_err(|e| {
            error!("删除知识库失败: {}", e);
            ErrorResponse::internal_server_error::<()>("删除知识库失败")
        })?;
    
    info!("知识库删除成功: id={}", kb_id);
    Ok(SuccessResponse::no_content().into_http_response()?)
}

/// 获取知识库统计信息
#[utoipa::path(
    get,
    path = "/api/v1/knowledge-bases/{id}/stats",
    params(
        ("id" = Uuid, Path, description = "知识库 ID")
    ),
    responses(
        (status = 200, description = "获取知识库统计信息成功", body = KnowledgeBaseStats),
        (status = 401, description = "未授权", body = ApiError),
        (status = 403, description = "权限不足", body = ApiError),
        (status = 404, description = "知识库不存在", body = ApiError),
        (status = 500, description = "服务器内部错误", body = ApiError)
    ),
    tag = "knowledge-bases",
    security(
        ("bearer_auth" = []),
        ("api_key" = [])
    )
)]
pub async fn get_knowledge_base_stats(
    db: web::Data<DatabaseConnection>,
    tenant_ctx: TenantContext,
    user_ctx: UserContext,
    path: web::Path<Uuid>,
) -> ActixResult<HttpResponse> {
    let kb_id = path.into_inner();
    debug!("获取知识库统计信息: id={}, 租户={}", kb_id, tenant_ctx.tenant.id);
    
    let kb = KnowledgeBase::find_by_id(kb_id)
        .filter(knowledge_base::Column::TenantId.eq(tenant_ctx.tenant.id))
        .one(db.as_ref())
        .await
        .map_err(|e| {
            error!("查询知识库失败: {}", e);
            ErrorResponse::internal_server_error::<()>("查询知识库失败")
        })?;
    
    let kb = match kb {
        Some(kb) => kb,
        None => {
            warn!("知识库不存在: id={}", kb_id);
            return Ok(ErrorResponse::not_found::<()>("知识库不存在").into_http_response()?);
        }
    };
    
    // 检查访问权限
    if !kb.has_access(&user_ctx.user.role, &user_ctx.user.id.to_string()).unwrap_or(false) {
        warn!("用户无权访问知识库统计: user={}, kb={}", user_ctx.user.id, kb_id);
        return Ok(ErrorResponse::forbidden::<()>("无权访问此知识库").into_http_response()?);
    }
    
    let stats = KnowledgeBaseStats::from(kb);
    Ok(SuccessResponse::ok(stats).into_http_response()?)
}

/// 重新索引知识库
#[utoipa::path(
    post,
    path = "/api/v1/knowledge-bases/{id}/reindex",
    params(
        ("id" = Uuid, Path, description = "知识库 ID")
    ),
    responses(
        (status = 202, description = "重新索引任务已启动", body = serde_json::Value),
        (status = 401, description = "未授权", body = ApiError),
        (status = 403, description = "权限不足", body = ApiError),
        (status = 404, description = "知识库不存在", body = ApiError),
        (status = 409, description = "知识库正在处理中", body = ApiError),
        (status = 500, description = "服务器内部错误", body = ApiError)
    ),
    tag = "knowledge-bases",
    security(
        ("bearer_auth" = []),
        ("api_key" = [])
    )
)]
pub async fn reindex_knowledge_base(
    db: web::Data<DatabaseConnection>,
    tenant_ctx: TenantContext,
    user_ctx: UserContext,
    path: web::Path<Uuid>,
) -> ActixResult<HttpResponse> {
    let kb_id = path.into_inner();
    info!("重新索引知识库请求: id={}, 租户={}", kb_id, tenant_ctx.tenant.id);
    
    // 查找知识库
    let kb = KnowledgeBase::find_by_id(kb_id)
        .filter(knowledge_base::Column::TenantId.eq(tenant_ctx.tenant.id))
        .one(db.as_ref())
        .await
        .map_err(|e| {
            error!("查询知识库失败: {}", e);
            ErrorResponse::internal_server_error::<()>("查询知识库失败")
        })?;
    
    let kb = match kb {
        Some(kb) => kb,
        None => {
            warn!("知识库不存在: id={}", kb_id);
            return Ok(ErrorResponse::not_found::<()>("知识库不存在").into_http_response()?);
        }
    };
    
    // 检查访问权限
    if !kb.has_access(&user_ctx.user.role, &user_ctx.user.id.to_string()).unwrap_or(false) {
        warn!("用户无权重新索引知识库: user={}, kb={}", user_ctx.user.id, kb_id);
        return Ok(ErrorResponse::forbidden::<()>("无权操作此知识库").into_http_response()?);
    }
    
    // 检查知识库状态
    if kb.is_processing() {
        warn!("知识库正在处理中，无法重新索引: id={}", kb_id);
        return Ok(ErrorResponse::conflict::<()>("知识库正在处理中，请稍后再试").into_http_response()?);
    }
    
    // 更新知识库状态为处理中
    let mut active_model: knowledge_base::ActiveModel = kb.into();
    let now = Utc::now().with_timezone(&chrono::FixedOffset::east_opt(8 * 3600).unwrap());
    
    active_model.status = sea_orm::Set(knowledge_base::KnowledgeBaseStatus::Processing);
    active_model.updated_at = sea_orm::Set(now);
    
    let updated_kb = active_model.update(db.as_ref()).await.map_err(|e| {
        error!("更新知识库状态失败: {}", e);
        ErrorResponse::internal_server_error::<()>("更新知识库状态失败")
    })?;
    
    // TODO: 这里应该启动异步重新索引任务
    // 目前只是返回任务已启动的响应
    
    info!("知识库重新索引任务已启动: id={}", kb_id);
    
    let response = serde_json::json!({
        "message": "重新索引任务已启动",
        "knowledge_base_id": kb_id,
        "status": "processing",
        "started_at": now
    });
    
    Ok(SuccessResponse::accepted(response).into_http_response()?)
}

/// 配置知识库路由
pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/knowledge-bases")
            .route("", web::post().to(create_knowledge_base))
            .route("", web::get().to(list_knowledge_bases))
            .route("/{id}", web::get().to(get_knowledge_base))
            .route("/{id}", web::put().to(update_knowledge_base))
            .route("/{id}", web::delete().to(delete_knowledge_base))
            .route("/{id}/stats", web::get().to(get_knowledge_base_stats))
            .route("/{id}/reindex", web::post().to(reindex_knowledge_base))
    );
}