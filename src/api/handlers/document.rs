// 文档管理 API 处理器

use actix_web::{web, HttpResponse, Result as ActixResult};
use actix_multipart::Multipart;
use futures::stream::StreamExt;
use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait, QueryOrder, PaginatorTrait, ActiveModelTrait};
use serde::{Deserialize, Serialize};
use utoipa::{ToSchema, IntoParams};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use tracing::{info, warn, error, debug};
use std::io::Write;

use crate::api::models::{PaginationQuery, PaginatedResponse, PaginationInfo};
use crate::api::responses::{ApiResponse, ApiError, ApiResponseExt};
use crate::api::middleware::tenant::TenantInfo;
use crate::api::extractors::{TenantContext, UserContext};
use crate::api::HttpResponseBuilder;
use crate::db::entities::{document, knowledge_base, prelude::*};
use crate::errors::AiStudioError;
use crate::services::knowledge_base::KnowledgeBaseService;

/// 文档创建请求
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateDocumentRequest {
    /// 知识库 ID
    pub knowledge_base_id: Uuid,
    /// 文档标题
    pub title: String,
    /// 文档内容（可选，如果是文件上传则不需要）
    pub content: Option<String>,
    /// 文档类型
    pub doc_type: document::DocumentType,
    /// 文档元数据
    pub metadata: Option<document::DocumentMetadata>,
    /// 处理配置
    pub processing_config: Option<document::DocumentProcessingConfig>,
}

/// 文档更新请求
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct UpdateDocumentRequest {
    /// 文档标题
    pub title: Option<String>,
    /// 文档内容
    pub content: Option<String>,
    /// 文档状态
    pub status: Option<document::DocumentStatus>,
    /// 文档元数据
    pub metadata: Option<document::DocumentMetadata>,
    /// 处理配置
    pub processing_config: Option<document::DocumentProcessingConfig>,
}

/// 文档响应
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct DocumentResponse {
    /// 文档 ID
    pub id: Uuid,
    /// 知识库 ID
    pub knowledge_base_id: Uuid,
    /// 文档标题
    pub title: String,
    /// 文档内容（可能被截断）
    pub content: String,
    /// 文档摘要
    pub summary: Option<String>,
    /// 文档类型
    pub doc_type: document::DocumentType,
    /// 文档状态
    pub status: document::DocumentStatus,
    /// 文件名
    pub file_name: Option<String>,
    /// 文件大小
    pub file_size: i64,
    /// 格式化的文件大小
    pub formatted_file_size: String,
    /// MIME 类型
    pub mime_type: Option<String>,
    /// 文档元数据
    pub metadata: document::DocumentMetadata,
    /// 处理配置
    pub processing_config: document::DocumentProcessingConfig,
    /// 文档块数量
    pub chunk_count: i32,
    /// 处理开始时间
    pub processing_started_at: Option<DateTime<Utc>>,
    /// 处理完成时间
    pub processing_completed_at: Option<DateTime<Utc>>,
    /// 处理耗时（毫秒）
    pub processing_duration_ms: Option<i64>,
    /// 错误信息
    pub error_message: Option<String>,
    /// 版本号
    pub version: i32,
    /// 进度百分比
    pub progress_percentage: f32,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

/// 文档搜索查询
#[derive(Debug, Clone, Deserialize, ToSchema, IntoParams)]
pub struct DocumentSearchQuery {
    /// 知识库 ID（可选，如果不指定则搜索所有知识库）
    pub knowledge_base_id: Option<Uuid>,
    /// 搜索关键词
    pub q: Option<String>,
    /// 文档类型过滤
    pub doc_type: Option<document::DocumentType>,
    /// 状态过滤
    pub status: Option<document::DocumentStatus>,
    /// 标签过滤
    pub tags: Option<Vec<String>>,
    /// 作者过滤
    pub author: Option<String>,
    /// 创建时间范围（开始）
    pub created_after: Option<DateTime<Utc>>,
    /// 创建时间范围（结束）
    pub created_before: Option<DateTime<Utc>>,
    /// 分页参数
    #[serde(flatten)]
    pub pagination: PaginationQuery,
}

/// 文档统计信息
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct DocumentStats {
    /// 文档 ID
    pub id: Uuid,
    /// 文档标题
    pub title: String,
    /// 文件大小
    pub file_size: i64,
    /// 格式化的文件大小
    pub formatted_file_size: String,
    /// 文档块数量
    pub chunk_count: i32,
    /// 字数
    pub word_count: Option<i32>,
    /// 字符数
    pub char_count: Option<i32>,
    /// 处理耗时（毫秒）
    pub processing_duration_ms: Option<i64>,
    /// 创建时间
    pub created_at: DateTime<Utc>,
}

/// 文档上传响应
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct DocumentUploadResponse {
    /// 文档 ID
    pub id: Uuid,
    /// 上传状态
    pub status: String,
    /// 文件名
    pub file_name: String,
    /// 文件大小
    pub file_size: i64,
    /// 消息
    pub message: String,
}

impl From<document::Model> for DocumentResponse {
    fn from(model: document::Model) -> Self {
        let metadata = model.get_metadata().unwrap_or_default();
        let processing_config = model.get_processing_config().unwrap_or_default();
        let formatted_file_size = model.formatted_file_size();
        let processing_duration_ms = model.processing_duration().map(|d| d.num_milliseconds());
        let progress_percentage = model.progress_percentage();
        
        // 截断内容以避免响应过大
        let content = if model.content.len() > 1000 {
            format!("{}...", &model.content[..1000])
        } else {
            model.content
        };
        
        Self {
            id: model.id,
            knowledge_base_id: model.knowledge_base_id,
            title: model.title,
            content,
            summary: model.summary,
            doc_type: model.doc_type,
            status: model.status,
            file_name: model.file_name,
            file_size: model.file_size,
            formatted_file_size,
            mime_type: model.mime_type,
            metadata,
            processing_config,
            chunk_count: model.chunk_count,
            processing_started_at: model.processing_started_at.map(|dt| dt.with_timezone(&Utc)),
            processing_completed_at: model.processing_completed_at.map(|dt| dt.with_timezone(&Utc)),
            processing_duration_ms,
            error_message: model.error_message,
            version: model.version,
            progress_percentage,
            created_at: model.created_at.with_timezone(&Utc),
            updated_at: model.updated_at.with_timezone(&Utc),
        }
    }
}

impl From<document::Model> for DocumentStats {
    fn from(model: document::Model) -> Self {
        let formatted_file_size = model.formatted_file_size();
        let processing_duration_ms = model.processing_duration().map(|d| d.num_milliseconds());
        let metadata = model.get_metadata().unwrap_or_default();
        
        Self {
            id: model.id,
            title: model.title,
            file_size: model.file_size,
            formatted_file_size,
            chunk_count: model.chunk_count,
            word_count: metadata.word_count,
            char_count: metadata.char_count,
            processing_duration_ms,
            created_at: model.created_at.with_timezone(&Utc),
        }
    }
}

/// 创建文档
#[utoipa::path(
    post,
    path = "/api/v1/documents",
    request_body = CreateDocumentRequest,
    responses(
        (status = 201, description = "文档创建成功", body = DocumentResponse),
        (status = 400, description = "请求参数错误", body = ApiError),
        (status = 401, description = "未授权", body = ApiError),
        (status = 403, description = "权限不足", body = ApiError),
        (status = 404, description = "知识库不存在", body = ApiError),
        (status = 500, description = "服务器内部错误", body = ApiError)
    ),
    tag = "documents",
    security(
        ("bearer_auth" = []),
        ("api_key" = [])
    )
)]
pub async fn create_document(
    db: web::Data<DatabaseConnection>,
    tenant_info: web::ReqData<TenantInfo>,
    req: web::Json<CreateDocumentRequest>,
) -> ActixResult<HttpResponse> {
    info!("创建文档请求: 租户={}, 知识库={}, 标题={}", 
          tenant_info.id, req.knowledge_base_id, req.title);
    
    // 检查知识库是否存在且属于当前租户
    let kb = KnowledgeBase::find_by_id(req.knowledge_base_id)
        .filter(knowledge_base::Column::TenantId.eq(tenant_info.id))
        .one(db.as_ref())
        .await
        .map_err(|e| {
            error!("查询知识库失败: {}", e);
            ApiError::internal_server_error("查询知识库失败")
        })?;
    
    if kb.is_none() {
        warn!("知识库不存在或无权访问: {}", req.knowledge_base_id);
        return Ok(HttpResponseBuilder::not_found::<()>("知识库不存在").unwrap());
    }
    
    // 准备文档数据
    let doc_id = Uuid::new_v4();
    let now = Utc::now().with_timezone(&chrono::FixedOffset::east_opt(8 * 3600).unwrap());
    let content = req.content.clone().unwrap_or_default();
    let metadata = req.metadata.clone().unwrap_or_default();
    let processing_config = req.processing_config.clone().unwrap_or_default();
    
    // 计算内容哈希
    let content_hash = format!("{:x}", md5::compute(&content));
    
    // 创建文档
    let new_doc = document::ActiveModel {
        id: sea_orm::Set(doc_id),
        knowledge_base_id: sea_orm::Set(req.knowledge_base_id),
        title: sea_orm::Set(req.title.clone()),
        content: sea_orm::Set(content.clone()),
        raw_content: sea_orm::Set(Some(content.clone())),
        summary: sea_orm::Set(None),
        doc_type: sea_orm::Set(req.doc_type.clone()),
        status: sea_orm::Set(document::DocumentStatus::Pending),
        file_path: sea_orm::Set(None),
        file_name: sea_orm::Set(None),
        file_size: sea_orm::Set(content.len() as i64),
        mime_type: sea_orm::Set(Some("text/plain".to_string())),
        content_hash: sea_orm::Set(Some(content_hash)),
        metadata: sea_orm::Set(serde_json::to_value(&metadata).unwrap().into()),
        processing_config: sea_orm::Set(serde_json::to_value(&processing_config).unwrap().into()),
        chunk_count: sea_orm::Set(0),
        processing_started_at: sea_orm::Set(None),
        processing_completed_at: sea_orm::Set(None),
        error_message: sea_orm::Set(None),
        version: sea_orm::Set(1),
        created_at: sea_orm::Set(now),
        updated_at: sea_orm::Set(now),
    };
    
    let doc = Document::insert(new_doc)
        .exec_with_returning(db.as_ref())
        .await
        .map_err(|e| {
            error!("创建文档失败: {}", e);
            ApiError::internal_server_error("创建文档失败")
        })?;
    
    info!("文档创建成功: id={}, 标题={}", doc.id, doc.title);
    
    let response = DocumentResponse::from(doc);
    Ok(ApiResponse::created(response).into_http_response().unwrap())
}

/// 上传文档文件
#[utoipa::path(
    post,
    path = "/api/v1/documents/upload",
    request_body(content = String, description = "文档文件", content_type = "multipart/form-data"),
    responses(
        (status = 201, description = "文档上传成功", body = DocumentUploadResponse),
        (status = 400, description = "请求参数错误", body = ApiError),
        (status = 401, description = "未授权", body = ApiError),
        (status = 413, description = "文件过大", body = ApiError),
        (status = 500, description = "服务器内部错误", body = ApiError)
    ),
    tag = "documents",
    security(
        ("bearer_auth" = []),
        ("api_key" = [])
    )
)]
pub async fn upload_document(
    db: web::Data<DatabaseConnection>,
    tenant_info: web::ReqData<TenantInfo>,
    mut payload: Multipart,
) -> ActixResult<HttpResponse> {
    info!("文档上传请求: 租户={}", tenant_info.id);
    
    let mut knowledge_base_id: Option<Uuid> = None;
    let mut title: Option<String> = None;
    let mut file_data: Option<Vec<u8>> = None;
    let mut file_name: Option<String> = None;
    let mut content_type: Option<String> = None;
    
    // 处理 multipart 数据
    while let Some(Ok(mut field)) = payload.next().await {
        let field_name = field.name().to_string();
        
        match field_name.as_str() {
            "knowledge_base_id" => {
                let mut data = Vec::new();
                while let Some(Ok(chunk)) = field.next().await {
                    data.extend_from_slice(&chunk);
                }
                let kb_id_str = String::from_utf8(data).map_err(|e| {
                    error!("知识库 ID 格式错误: {}", e);
                    ApiError::bad_request("知识库 ID 格式错误")
                })?;
                knowledge_base_id = Some(Uuid::parse_str(&kb_id_str).map_err(|e| {
                    error!("知识库 ID 解析失败: {}", e);
                    ApiError::bad_request("无效的知识库 ID 格式")
                })?);
            }
            "title" => {
                let mut data = Vec::new();
                while let Some(Ok(chunk)) = field.next().await {
                    data.extend_from_slice(&chunk);
                }
                title = Some(String::from_utf8(data).map_err(|e| {
                    error!("标题格式错误: {}", e);
                    ApiError::bad_request("标题格式错误")
                })?);
            }
            "file" => {
                file_name = field.content_disposition().get_filename().map(|s| s.to_string());
                content_type = field.content_type().map(|ct| ct.to_string());
                
                let mut data = Vec::new();
                while let Some(Ok(chunk)) = field.next().await {
                    data.extend_from_slice(&chunk);
                    
                    // 限制文件大小（例如 10MB）
                    if data.len() > 10 * 1024 * 1024 {
                        return Ok(HttpResponseBuilder::payload_too_large::<()>("文件大小超过限制（10MB）").unwrap());
                    }
                }
                file_data = Some(data);
            }
            _ => {
                // 忽略未知字段
                while let Some(_) = field.next().await {}
            }
        }
    }
    
    // 验证必需字段
    let knowledge_base_id = knowledge_base_id.ok_or_else(|| {
        ApiError::bad_request("缺少知识库 ID")
    })?;
    
    let file_data = file_data.ok_or_else(|| {
        ApiError::bad_request("缺少文件数据")
    })?;
    
    let file_name = file_name.ok_or_else(|| {
        ApiError::bad_request("缺少文件名")
    })?;
    
    let title = title.unwrap_or_else(|| {
        // 如果没有提供标题，使用文件名（去掉扩展名）
        std::path::Path::new(&file_name)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(&file_name)
            .to_string()
    });
    
    // 检查知识库是否存在且属于当前租户
    let kb = KnowledgeBase::find_by_id(knowledge_base_id)
        .filter(knowledge_base::Column::TenantId.eq(tenant_info.id))
        .one(db.as_ref())
        .await
        .map_err(|e| {
            error!("查询知识库失败: {}", e);
            ApiError::internal_server_error("查询知识库失败")
        })?;
    
    if kb.is_none() {
        warn!("知识库不存在或无权访问: {}", knowledge_base_id);
        return Ok(HttpResponseBuilder::not_found::<()>("知识库不存在").unwrap());
    }
    
    // 确定文档类型
    let doc_type = determine_document_type(&file_name, content_type.as_deref());
    
    // 提取文本内容（简单实现，实际应该使用专门的文档处理服务）
    let content = extract_text_content(&file_data, &doc_type)?;
    
    // 计算内容哈希
    let content_hash = format!("{:x}", md5::compute(&content));
    
    // 创建文档
    let doc_id = Uuid::new_v4();
    let now = Utc::now().with_timezone(&chrono::FixedOffset::east_opt(8 * 3600).unwrap());
    
    // 保存文件（这里简化处理，实际应该保存到文件系统或对象存储）
    let file_path = format!("uploads/{}/{}", tenant_info.id, doc_id);
    
    let new_doc = document::ActiveModel {
        id: sea_orm::Set(doc_id),
        knowledge_base_id: sea_orm::Set(knowledge_base_id),
        title: sea_orm::Set(title),
        content: sea_orm::Set(content),
        raw_content: sea_orm::Set(Some(String::from_utf8_lossy(&file_data).to_string())),
        summary: sea_orm::Set(None),
        doc_type: sea_orm::Set(doc_type),
        status: sea_orm::Set(document::DocumentStatus::Pending),
        file_path: sea_orm::Set(Some(file_path)),
        file_name: sea_orm::Set(Some(file_name.clone())),
        file_size: sea_orm::Set(file_data.len() as i64),
        mime_type: sea_orm::Set(content_type),
        content_hash: sea_orm::Set(Some(content_hash)),
        metadata: sea_orm::Set(serde_json::to_value(&document::DocumentMetadata::default()).unwrap().into()),
        processing_config: sea_orm::Set(serde_json::to_value(&document::DocumentProcessingConfig::default()).unwrap().into()),
        chunk_count: sea_orm::Set(0),
        processing_started_at: sea_orm::Set(None),
        processing_completed_at: sea_orm::Set(None),
        error_message: sea_orm::Set(None),
        version: sea_orm::Set(1),
        created_at: sea_orm::Set(now),
        updated_at: sea_orm::Set(now),
    };
    
    let doc = Document::insert(new_doc)
        .exec_with_returning(db.as_ref())
        .await
        .map_err(|e| {
            error!("创建文档失败: {}", e);
            ApiError::internal_server_error("创建文档失败")
        })?;
    
    info!("文档上传成功: id={}, 文件名={}, 大小={}", doc.id, file_name, file_data.len());
    
    let response = DocumentUploadResponse {
        id: doc.id,
        status: "uploaded".to_string(),
        file_name,
        file_size: file_data.len() as i64,
        message: "文档上传成功，正在处理中".to_string(),
    };
    
    Ok(ApiResponse::created(response).into_http_response().unwrap())
}

/// 辅助函数：确定文档类型
fn determine_document_type(file_name: &str, content_type: Option<&str>) -> document::DocumentType {
    // 首先根据文件扩展名判断
    if let Some(extension) = std::path::Path::new(file_name).extension() {
        match extension.to_str().unwrap_or("").to_lowercase().as_str() {
            "pdf" => return document::DocumentType::Pdf,
            "doc" | "docx" => return document::DocumentType::Word,
            "md" | "markdown" => return document::DocumentType::Markdown,
            "html" | "htm" => return document::DocumentType::Html,
            "csv" => return document::DocumentType::Csv,
            "json" => return document::DocumentType::Json,
            "xml" => return document::DocumentType::Xml,
            "txt" => return document::DocumentType::Text,
            _ => {}
        }
    }
    
    // 然后根据 MIME 类型判断
    if let Some(mime) = content_type {
        match mime {
            "application/pdf" => return document::DocumentType::Pdf,
            "application/msword" | "application/vnd.openxmlformats-officedocument.wordprocessingml.document" => {
                return document::DocumentType::Word;
            }
            "text/markdown" => return document::DocumentType::Markdown,
            "text/html" => return document::DocumentType::Html,
            "text/csv" => return document::DocumentType::Csv,
            "application/json" => return document::DocumentType::Json,
            "application/xml" | "text/xml" => return document::DocumentType::Xml,
            "text/plain" => return document::DocumentType::Text,
            _ => {}
        }
    }
    
    // 默认为文本类型
    document::DocumentType::Text
}

/// 辅助函数：提取文本内容
fn extract_text_content(file_data: &[u8], doc_type: &document::DocumentType) -> Result<String, ApiError> {
    match doc_type {
        document::DocumentType::Text | document::DocumentType::Markdown => {
            String::from_utf8(file_data.to_vec()).map_err(|e| {
                error!("文本文件编码错误: {}", e);
                ApiError::bad_request("文件编码格式不支持")
            })
        }
        document::DocumentType::Json => {
            // 验证 JSON 格式并提取文本
            let json_str = String::from_utf8(file_data.to_vec()).map_err(|e| {
                error!("JSON 文件编码错误: {}", e);
                ApiError::bad_request("JSON 文件编码格式不支持")
            })?;
            
            // 验证 JSON 格式
            serde_json::from_str::<serde_json::Value>(&json_str).map_err(|e| {
                error!("JSON 格式错误: {}", e);
                ApiError::bad_request("无效的 JSON 格式")
            })?;
            
            Ok(json_str)
        }
        _ => {
            // 对于其他类型，暂时返回原始内容
            // 实际应该使用专门的文档处理库
            Ok(String::from_utf8_lossy(file_data).to_string())
        }
    }
}



/// 获取文档列表
#[utoipa::path(
    get,
    path = "/api/v1/documents",
    params(DocumentSearchQuery),
    responses(
        (status = 200, description = "获取文档列表成功", body = PaginatedResponse<DocumentResponse>),
        (status = 400, description = "请求参数错误", body = ApiError),
        (status = 401, description = "未授权", body = ApiError),
        (status = 500, description = "服务器内部错误", body = ApiError)
    ),
    tag = "documents",
    security(
        ("bearer_auth" = []),
        ("api_key" = [])
    )
)]
pub async fn list_documents(
    db: web::Data<DatabaseConnection>,
    tenant_info: web::ReqData<TenantInfo>,
    query: web::Query<DocumentSearchQuery>,
) -> ActixResult<HttpResponse> {
    debug!("获取文档列表: 租户={}", tenant_info.id);
    
    let mut query_params = query.into_inner();
    query_params.pagination.validate();
    
    // 构建查询 - 首先通过知识库过滤租户
    let mut select = Document::find()
        .inner_join(KnowledgeBase)
        .filter(knowledge_base::Column::TenantId.eq(tenant_info.id));
    
    // 添加知识库过滤
    if let Some(kb_id) = query_params.knowledge_base_id {
        select = select.filter(document::Column::KnowledgeBaseId.eq(kb_id));
    }
    
    // 添加搜索条件
    if let Some(q) = &query_params.q {
        select = select.filter(
            document::Column::Title.contains(q)
                .or(document::Column::Content.contains(q))
                .or(document::Column::Summary.contains(q))
        );
    }
    
    if let Some(doc_type) = &query_params.doc_type {
        select = select.filter(document::Column::DocType.eq(doc_type.clone()));
    }
    
    if let Some(status) = &query_params.status {
        select = select.filter(document::Column::Status.eq(status.clone()));
    }
    
    if let Some(created_after) = query_params.created_after {
        let created_after = created_after.with_timezone(&chrono::FixedOffset::east_opt(8 * 3600).unwrap());
        select = select.filter(document::Column::CreatedAt.gte(created_after));
    }
    
    if let Some(created_before) = query_params.created_before {
        let created_before = created_before.with_timezone(&chrono::FixedOffset::east_opt(8 * 3600).unwrap());
        select = select.filter(document::Column::CreatedAt.lte(created_before));
    }
    
    // TODO: 实现标签和作者过滤（需要在元数据中搜索）
    
    // 添加排序
    let sort_column = query_params.pagination.sort_by.as_deref().unwrap_or("created_at");
    select = match sort_column {
        "title" => match query_params.pagination.sort_order {
            crate::api::models::SortOrder::Asc => select.order_by_asc(document::Column::Title),
            crate::api::models::SortOrder::Desc => select.order_by_desc(document::Column::Title),
        },
        "updated_at" => match query_params.pagination.sort_order {
            crate::api::models::SortOrder::Asc => select.order_by_asc(document::Column::UpdatedAt),
            crate::api::models::SortOrder::Desc => select.order_by_desc(document::Column::UpdatedAt),
        },
        "file_size" => match query_params.pagination.sort_order {
            crate::api::models::SortOrder::Asc => select.order_by_asc(document::Column::FileSize),
            crate::api::models::SortOrder::Desc => select.order_by_desc(document::Column::FileSize),
        },
        "status" => match query_params.pagination.sort_order {
            crate::api::models::SortOrder::Asc => select.order_by_asc(document::Column::Status),
            crate::api::models::SortOrder::Desc => select.order_by_desc(document::Column::Status),
        },
        _ => match query_params.pagination.sort_order {
            crate::api::models::SortOrder::Asc => select.order_by_asc(document::Column::CreatedAt),
            crate::api::models::SortOrder::Desc => select.order_by_desc(document::Column::CreatedAt),
        },
    };
    
    // 执行分页查询
    let paginator = select.paginate(db.as_ref(), query_params.pagination.page_size as u64);
    let total = paginator.num_items().await.map_err(|e| {
        error!("查询文档总数失败: {}", e);
        ApiError::internal_server_error("查询文档失败")
    })?;
    
    let documents = paginator
        .fetch_page((query_params.pagination.page - 1) as u64)
        .await
        .map_err(|e| {
            error!("查询文档列表失败: {}", e);
            ApiError::internal_server_error("查询文档失败")
        })?;
    
    let responses: Vec<DocumentResponse> = documents
        .into_iter()
        .map(DocumentResponse::from)
        .collect();
    
    let pagination = PaginationInfo::new(
        query_params.pagination.page,
        query_params.pagination.page_size,
        total,
    );
    
    let response = PaginatedResponse::new(responses, pagination);
    Ok(ApiResponse::ok(response).into_http_response().unwrap())
}

/// 获取文档详情
#[utoipa::path(
    get,
    path = "/api/v1/documents/{id}",
    params(
        ("id" = Uuid, Path, description = "文档 ID")
    ),
    responses(
        (status = 200, description = "获取文档详情成功", body = DocumentResponse),
        (status = 401, description = "未授权", body = ApiError),
        (status = 403, description = "权限不足", body = ApiError),
        (status = 404, description = "文档不存在", body = ApiError),
        (status = 500, description = "服务器内部错误", body = ApiError)
    ),
    tag = "documents",
    security(
        ("bearer_auth" = []),
        ("api_key" = [])
    )
)]
pub async fn get_document(
    db: web::Data<DatabaseConnection>,
    tenant_info: web::ReqData<TenantInfo>,
    path: web::Path<Uuid>,
) -> ActixResult<HttpResponse> {
    let doc_id = path.into_inner();
    debug!("获取文档详情: id={}, 租户={}", doc_id, tenant_info.id);
    
    let doc = Document::find_by_id(doc_id)
        .inner_join(KnowledgeBase)
        .filter(knowledge_base::Column::TenantId.eq(tenant_info.id))
        .one(db.as_ref())
        .await
        .map_err(|e| {
            error!("查询文档失败: {}", e);
            ApiError::internal_server_error("查询文档失败")
        })?;
    
    let doc = match doc {
        Some(doc) => doc,
        None => {
            warn!("文档不存在或无权访问: id={}", doc_id);
            return Ok(HttpResponseBuilder::not_found::<()>("文档").unwrap());
        }
    };
    
    let response = DocumentResponse::from(doc);
    Ok(ApiResponse::ok(response).into_http_response().unwrap())
}

/// 更新文档
#[utoipa::path(
    put,
    path = "/api/v1/documents/{id}",
    params(
        ("id" = Uuid, Path, description = "文档 ID")
    ),
    request_body = UpdateDocumentRequest,
    responses(
        (status = 200, description = "更新文档成功", body = DocumentResponse),
        (status = 400, description = "请求参数错误", body = ApiError),
        (status = 401, description = "未授权", body = ApiError),
        (status = 403, description = "权限不足", body = ApiError),
        (status = 404, description = "文档不存在", body = ApiError),
        (status = 500, description = "服务器内部错误", body = ApiError)
    ),
    tag = "documents",
    security(
        ("bearer_auth" = []),
        ("api_key" = [])
    )
)]
pub async fn update_document(
    db: web::Data<DatabaseConnection>,
    tenant_info: web::ReqData<TenantInfo>,
    path: web::Path<Uuid>,
    req: web::Json<UpdateDocumentRequest>,
) -> ActixResult<HttpResponse> {
    let doc_id = path.into_inner();
    info!("更新文档请求: id={}, 租户={}", doc_id, tenant_info.id);
    
    // 查找文档
    let doc = Document::find_by_id(doc_id)
        .inner_join(KnowledgeBase)
        .filter(knowledge_base::Column::TenantId.eq(tenant_info.id))
        .one(db.as_ref())
        .await
        .map_err(|e| {
            error!("查询文档失败: {}", e);
            ApiError::internal_server_error("查询文档失败")
        })?;
    
    let doc = match doc {
        Some(doc) => doc,
        None => {
            warn!("文档不存在或无权访问: id={}", doc_id);
            return Ok(HttpResponseBuilder::not_found::<()>("文档").unwrap());
        }
    };
    
    // 准备更新数据
    let mut active_model: document::ActiveModel = doc.into();
    let now = Utc::now().with_timezone(&chrono::FixedOffset::east_opt(8 * 3600).unwrap());
    
    if let Some(title) = &req.title {
        active_model.title = sea_orm::Set(title.clone());
    }
    
    if let Some(content) = &req.content {
        active_model.content = sea_orm::Set(content.clone());
        active_model.file_size = sea_orm::Set(content.len() as i64);
        
        // 更新内容哈希
        let content_hash = format!("{:x}", md5::compute(content));
        active_model.content_hash = sea_orm::Set(Some(content_hash));
        
        // 如果内容发生变化，重置处理状态
        active_model.status = sea_orm::Set(document::DocumentStatus::Pending);
        active_model.chunk_count = sea_orm::Set(0);
        active_model.processing_started_at = sea_orm::Set(None);
        active_model.processing_completed_at = sea_orm::Set(None);
        active_model.error_message = sea_orm::Set(None);
        
        // 增加版本号
        if let sea_orm::ActiveValue::Unchanged(version) = &active_model.version {
            active_model.version = sea_orm::Set(version + 1);
        }
    }
    
    if let Some(status) = &req.status {
        active_model.status = sea_orm::Set(status.clone());
        
        // 根据状态更新时间戳
        match status {
            document::DocumentStatus::Processing => {
                active_model.processing_started_at = sea_orm::Set(Some(now));
            }
            document::DocumentStatus::Completed => {
                active_model.processing_completed_at = sea_orm::Set(Some(now));
                active_model.error_message = sea_orm::Set(None);
            }
            document::DocumentStatus::Failed => {
                active_model.processing_completed_at = sea_orm::Set(Some(now));
            }
            _ => {}
        }
    }
    
    if let Some(metadata) = &req.metadata {
        active_model.metadata = sea_orm::Set(serde_json::to_value(metadata).unwrap().into());
    }
    
    if let Some(processing_config) = &req.processing_config {
        active_model.processing_config = sea_orm::Set(serde_json::to_value(processing_config).unwrap().into());
    }
    
    active_model.updated_at = sea_orm::Set(now);
    
    // 执行更新
    let updated_doc = active_model.update(db.as_ref()).await.map_err(|e| {
        error!("更新文档失败: {}", e);
        ApiError::internal_server_error("更新文档失败")
    })?;
    
    info!("文档更新成功: id={}, 标题={}", updated_doc.id, updated_doc.title);
    
    let response = DocumentResponse::from(updated_doc);
    Ok(ApiResponse::ok(response).into_http_response().unwrap())
}

/// 删除文档
#[utoipa::path(
    delete,
    path = "/api/v1/documents/{id}",
    params(
        ("id" = Uuid, Path, description = "文档 ID")
    ),
    responses(
        (status = 204, description = "删除文档成功"),
        (status = 401, description = "未授权", body = ApiError),
        (status = 403, description = "权限不足", body = ApiError),
        (status = 404, description = "文档不存在", body = ApiError),
        (status = 500, description = "服务器内部错误", body = ApiError)
    ),
    tag = "documents",
    security(
        ("bearer_auth" = []),
        ("api_key" = [])
    )
)]
pub async fn delete_document(
    db: web::Data<DatabaseConnection>,
    tenant_info: web::ReqData<TenantInfo>,
    path: web::Path<Uuid>,
) -> ActixResult<HttpResponse> {
    let doc_id = path.into_inner();
    info!("删除文档请求: id={}, 租户={}", doc_id, tenant_info.id);
    
    // 查找文档
    let doc = Document::find_by_id(doc_id)
        .inner_join(KnowledgeBase)
        .filter(knowledge_base::Column::TenantId.eq(tenant_info.id))
        .one(db.as_ref())
        .await
        .map_err(|e| {
            error!("查询文档失败: {}", e);
            ApiError::internal_server_error("查询文档失败")
        })?;
    
    if doc.is_none() {
        warn!("文档不存在或无权访问: id={}", doc_id);
        return Ok(HttpResponseBuilder::not_found::<()>("文档不存在").unwrap());
    }
    
    // 执行删除
    Document::delete_by_id(doc_id)
        .exec(db.as_ref())
        .await
        .map_err(|e| {
            error!("删除文档失败: {}", e);
            ApiError::internal_server_error("删除文档失败")
        })?;
    
    info!("文档删除成功: id={}", doc_id);
    Ok(HttpResponseBuilder::no_content().unwrap())
}

/// 获取文档统计信息
#[utoipa::path(
    get,
    path = "/api/v1/documents/{id}/stats",
    params(
        ("id" = Uuid, Path, description = "文档 ID")
    ),
    responses(
        (status = 200, description = "获取文档统计信息成功", body = DocumentStats),
        (status = 401, description = "未授权", body = ApiError),
        (status = 403, description = "权限不足", body = ApiError),
        (status = 404, description = "文档不存在", body = ApiError),
        (status = 500, description = "服务器内部错误", body = ApiError)
    ),
    tag = "documents",
    security(
        ("bearer_auth" = []),
        ("api_key" = [])
    )
)]
pub async fn get_document_stats(
    db: web::Data<DatabaseConnection>,
    tenant_info: web::ReqData<TenantInfo>,
    path: web::Path<Uuid>,
) -> ActixResult<HttpResponse> {
    let doc_id = path.into_inner();
    debug!("获取文档统计信息: id={}, 租户={}", doc_id, tenant_info.id);
    
    let doc = Document::find_by_id(doc_id)
        .inner_join(KnowledgeBase)
        .filter(knowledge_base::Column::TenantId.eq(tenant_info.id))
        .one(db.as_ref())
        .await
        .map_err(|e| {
            error!("查询文档失败: {}", e);
            ApiError::internal_server_error("查询文档失败")
        })?;
    
    let doc = match doc {
        Some(doc) => doc,
        None => {
            warn!("文档不存在或无权访问: id={}", doc_id);
            return Ok(HttpResponseBuilder::not_found::<()>("文档").unwrap());
        }
    };
    
    let stats = DocumentStats::from(doc);
    Ok(ApiResponse::ok(stats).into_http_response().unwrap())
}

/// 重新处理文档
#[utoipa::path(
    post,
    path = "/api/v1/documents/{id}/reprocess",
    params(
        ("id" = Uuid, Path, description = "文档 ID")
    ),
    responses(
        (status = 202, description = "重新处理任务已启动", body = serde_json::Value),
        (status = 401, description = "未授权", body = ApiError),
        (status = 403, description = "权限不足", body = ApiError),
        (status = 404, description = "文档不存在", body = ApiError),
        (status = 409, description = "文档正在处理中", body = ApiError),
        (status = 500, description = "服务器内部错误", body = ApiError)
    ),
    tag = "documents",
    security(
        ("bearer_auth" = []),
        ("api_key" = [])
    )
)]
pub async fn reprocess_document(
    db: web::Data<DatabaseConnection>,
    tenant_info: web::ReqData<TenantInfo>,
    path: web::Path<Uuid>,
) -> ActixResult<HttpResponse> {
    let doc_id = path.into_inner();
    info!("重新处理文档请求: id={}, 租户={}", doc_id, tenant_info.id);
    
    // 查找文档
    let doc = Document::find_by_id(doc_id)
        .inner_join(KnowledgeBase)
        .filter(knowledge_base::Column::TenantId.eq(tenant_info.id))
        .one(db.as_ref())
        .await
        .map_err(|e| {
            error!("查询文档失败: {}", e);
            ApiError::internal_server_error("查询文档失败")
        })?;
    
    // 检查文档是否存在
    let doc = match doc {
        Some(d) => d,
        None => {
            warn!("文档不存在或无权访问: id={}", doc_id);
            return Ok(HttpResponseBuilder::not_found::<()>("文档").unwrap());
        }
    };
    
    // 检查文档状态
    if doc.status == document::DocumentStatus::Processing {
        return Ok(HttpResponseBuilder::conflict::<()>("文档正在处理中，请稍后再试".to_string()).unwrap());
    }
    
    // 更新文档状态为处理中
    let mut active_model: document::ActiveModel = doc.into();
    let now = Utc::now().with_timezone(&chrono::FixedOffset::east_opt(8 * 3600).unwrap());
    
    active_model.status = sea_orm::Set(document::DocumentStatus::Processing);
    active_model.processing_started_at = sea_orm::Set(Some(now));
    active_model.processing_completed_at = sea_orm::Set(None);
    active_model.error_message = sea_orm::Set(None);
    active_model.updated_at = sea_orm::Set(now);
    
    let _updated_doc = active_model.update(db.as_ref()).await.map_err(|e| {
        error!("更新文档状态失败: {}", e);
        ApiError::internal_server_error("更新文档状态失败")
    })?;
    
    // TODO: 这里应该启动异步文档处理任务
    // 目前只是返回任务已启动的响应
    
    info!("文档重新处理任务已启动: id={}", doc_id);

    let response = serde_json::json!({
        "message": "重新处理任务已启动",
        "document_id": doc_id,
        "status": "processing",
        "started_at": now
    });

    Ok(ApiResponse::ok(response).into_http_response().unwrap())
}



/// 批量操作类型
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum BatchDocumentOperation {
    Delete,
    Update,
    Reprocess,
    Export,
}

/// 批量文档操作请求
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct BatchDocumentRequest {
    /// 操作类型
    pub operation: BatchDocumentOperation,
    /// 文档 ID 列表
    pub document_ids: Vec<Uuid>,
    /// 操作参数（可选）
    pub parameters: Option<serde_json::Value>,
}

/// 批量操作响应
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct BatchDocumentResponse {
    /// 批量操作 ID
    pub batch_id: Uuid,
    /// 操作类型
    pub operation: BatchDocumentOperation,
    /// 总数量
    pub total_count: u32,
    /// 成功数量
    pub success_count: u32,
    /// 失败数量
    pub error_count: u32,
    /// 处理状态
    pub status: String,
    /// 成功的文档 ID
    pub success_ids: Vec<Uuid>,
    /// 失败的详情
    pub errors: Vec<BatchDocumentError>,
    /// 开始时间
    pub started_at: DateTime<Utc>,
    /// 完成时间
    pub completed_at: Option<DateTime<Utc>>,
}

/// 批量操作错误
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct BatchDocumentError {
    /// 文档 ID
    pub document_id: Uuid,
    /// 错误代码
    pub error_code: String,
    /// 错误消息
    pub error_message: String,
}

/// 批量导入请求
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct BatchImportRequest {
    /// 知识库 ID
    pub knowledge_base_id: Uuid,
    /// 导入选项
    pub options: BatchImportOptions,
}

/// 批量导入选项
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct BatchImportOptions {
    /// 是否覆盖已存在的文档
    pub overwrite_existing: bool,
    /// 是否跳过重复文档
    pub skip_duplicates: bool,
    /// 默认文档类型
    pub default_doc_type: Option<document::DocumentType>,
    /// 批处理大小
    pub batch_size: Option<u32>,
    /// 是否异步处理
    pub async_processing: bool,
}

/// 批量导入响应
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct BatchImportResponse {
    /// 导入任务 ID
    pub import_id: Uuid,
    /// 上传的文件数量
    pub uploaded_count: u32,
    /// 处理状态
    pub status: String,
    /// 消息
    pub message: String,
    /// 开始时间
    pub started_at: DateTime<Utc>,
}

/// 批量导出请求
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct BatchExportRequest {
    /// 知识库 ID（可选，如果不指定则导出所有）
    pub knowledge_base_id: Option<Uuid>,
    /// 文档 ID 列表（可选，如果不指定则导出知识库所有文档）
    pub document_ids: Option<Vec<Uuid>>,
    /// 导出格式
    pub format: ExportFormat,
    /// 导出选项
    pub options: BatchExportOptions,
}

/// 导出格式
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    Json,
    Csv,
    Zip,
}

/// 批量导出选项
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct BatchExportOptions {
    /// 是否包含内容
    pub include_content: bool,
    /// 是否包含元数据
    pub include_metadata: bool,
    /// 是否包含文档块
    pub include_chunks: bool,
    /// 压缩级别（对于 ZIP 格式）
    pub compression_level: Option<u8>,
}

/// 批量导出响应
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct BatchExportResponse {
    /// 导出任务 ID
    pub export_id: Uuid,
    /// 导出的文档数量
    pub document_count: u32,
    /// 文件下载 URL
    pub download_url: Option<String>,
    /// 处理状态
    pub status: String,
    /// 文件大小（字节）
    pub file_size: Option<i64>,
    /// 开始时间
    pub started_at: DateTime<Utc>,
    /// 完成时间
    pub completed_at: Option<DateTime<Utc>>,
}

/// 批量文档操作
#[utoipa::path(
    post,
    path = "/api/v1/documents/batch",
    request_body = BatchDocumentRequest,
    responses(
        (status = 202, description = "批量操作已启动", body = BatchDocumentResponse),
        (status = 400, description = "请求参数错误", body = ApiError),
        (status = 401, description = "未授权", body = ApiError),
        (status = 403, description = "权限不足", body = ApiError),
        (status = 500, description = "服务器内部错误", body = ApiError)
    ),
    tag = "documents",
    security(
        ("bearer_auth" = []),
        ("api_key" = [])
    )
)]
pub async fn batch_document_operation(
    db: web::Data<DatabaseConnection>,
    tenant_info: web::ReqData<TenantInfo>,
    req: web::Json<BatchDocumentRequest>,
) -> ActixResult<HttpResponse> {
    info!("批量文档操作请求: 租户={}, 操作={:?}, 数量={}", 
          tenant_info.id, req.operation, req.document_ids.len());
    
    if req.document_ids.is_empty() {
        return Ok(HttpResponseBuilder::bad_request::<()>("文档 ID 列表不能为空".to_string()).unwrap());
    }
    
    if req.document_ids.len() > 1000 {
        return Ok(HttpResponseBuilder::bad_request::<()>("批量操作文档数量不能超过 1000".to_string()).unwrap());
    }
    
    let batch_id = Uuid::new_v4();
    let now = Utc::now();
    
    // 验证所有文档都属于当前租户
    let valid_docs = Document::find()
        .inner_join(KnowledgeBase)
        .filter(knowledge_base::Column::TenantId.eq(tenant_info.id))
        .filter(document::Column::Id.is_in(req.document_ids.clone()))
        .all(db.as_ref())
        .await
        .map_err(|e| {
            error!("查询文档失败: {}", e);
            ApiError::internal_server_error("查询文档失败")
        })?;
    
    let valid_ids: Vec<Uuid> = valid_docs.iter().map(|doc| doc.id).collect();
    let invalid_ids: Vec<Uuid> = req.document_ids.iter()
        .filter(|id| !valid_ids.contains(id))
        .cloned()
        .collect();
    
    let mut response = BatchDocumentResponse {
        batch_id,
        operation: req.operation.clone(),
        total_count: req.document_ids.len() as u32,
        success_count: 0,
        error_count: 0,
        status: "processing".to_string(),
        success_ids: Vec::new(),
        errors: Vec::new(),
        started_at: now,
        completed_at: None,
    };
    
    // 添加无效文档的错误
    for invalid_id in invalid_ids {
        response.errors.push(BatchDocumentError {
            document_id: invalid_id,
            error_code: "NOT_FOUND".to_string(),
            error_message: "文档不存在或无权访问".to_string(),
        });
        response.error_count += 1;
    }
    
    // 执行批量操作
    match req.operation {
        BatchDocumentOperation::Delete => {
            for doc in valid_docs {
                match Document::delete_by_id(doc.id).exec(db.as_ref()).await {
                    Ok(_) => {
                        response.success_ids.push(doc.id);
                        response.success_count += 1;
                    }
                    Err(e) => {
                        error!("删除文档失败: id={}, error={}", doc.id, e);
                        response.errors.push(BatchDocumentError {
                            document_id: doc.id,
                            error_code: "DELETE_FAILED".to_string(),
                            error_message: format!("删除失败: {}", e),
                        });
                        response.error_count += 1;
                    }
                }
            }
        }
        BatchDocumentOperation::Update => {
            // 从参数中获取更新数据
            if let Some(params) = &req.parameters {
                if let Ok(update_data) = serde_json::from_value::<UpdateDocumentRequest>(params.clone()) {
                    for doc in valid_docs {
                        match update_document_internal(db.as_ref(), doc.clone(), &update_data).await {
                            Ok(updated_doc) => {
                                response.success_ids.push(updated_doc.id);
                                response.success_count += 1;
                            }
                            Err(e) => {
                                error!("更新文档失败: id={}, error={}", doc.id, e);
                                response.errors.push(BatchDocumentError {
                                    document_id: doc.id,
                                    error_code: "UPDATE_FAILED".to_string(),
                                    error_message: format!("更新失败: {}", e),
                                });
                                response.error_count += 1;
                            }
                        }
                    }
                } else {
                    return Ok(HttpResponseBuilder::bad_request::<()>("无效的更新参数".to_string()).unwrap());
                }
            } else {
                return Ok(HttpResponseBuilder::bad_request::<()>("批量更新需要提供更新参数".to_string()).unwrap());
            }
        }
        BatchDocumentOperation::Reprocess => {
            let now = Utc::now().with_timezone(&chrono::FixedOffset::east_opt(8 * 3600).unwrap());
            
            for doc in valid_docs {
                let mut active_model: document::ActiveModel = doc.into();
                active_model.status = sea_orm::Set(document::DocumentStatus::Processing);
                active_model.processing_started_at = sea_orm::Set(Some(now));
                active_model.processing_completed_at = sea_orm::Set(None);
                active_model.error_message = sea_orm::Set(None);
                active_model.updated_at = sea_orm::Set(now);
                
                let active_model_id = active_model.id.clone();
                match active_model.update(db.as_ref()).await {
                    Ok(updated_doc) => {
                        response.success_ids.push(updated_doc.id);
                        response.success_count += 1;
                    }
                    Err(e) => {
                        error!("重新处理文档失败: id={:?}, error={}", active_model_id, e);
                        response.errors.push(BatchDocumentError {
                            document_id: active_model_id.unwrap(),
                            error_code: "REPROCESS_FAILED".to_string(),
                            error_message: format!("重新处理失败: {}", e),
                        });
                        response.error_count += 1;
                    }
                }
            }
        }
        BatchDocumentOperation::Export => {
            // 导出操作通常是异步的，这里只是标记为成功
            // 实际的导出逻辑应该在后台任务中处理
            for doc in valid_docs {
                response.success_ids.push(doc.id);
                response.success_count += 1;
            }
        }
    }
    
    response.completed_at = Some(Utc::now());
    response.status = if response.error_count == 0 {
        "completed".to_string()
    } else if response.success_count == 0 {
        "failed".to_string()
    } else {
        "partial".to_string()
    };
    
    info!("批量文档操作完成: batch_id={}, 成功={}, 失败={}", 
          batch_id, response.success_count, response.error_count);
    
    Ok(ApiResponse::ok(response).into_http_response().unwrap())
}

/// 内部更新文档函数
async fn update_document_internal(
    db: &DatabaseConnection,
    doc: document::Model,
    req: &UpdateDocumentRequest,
) -> Result<document::Model, AiStudioError> {
    let mut active_model: document::ActiveModel = doc.into();
    let now = Utc::now().with_timezone(&chrono::FixedOffset::east_opt(8 * 3600).unwrap());
    
    if let Some(title) = &req.title {
        active_model.title = sea_orm::Set(title.clone());
    }
    
    if let Some(content) = &req.content {
        active_model.content = sea_orm::Set(content.clone());
        active_model.file_size = sea_orm::Set(content.len() as i64);
        
        let content_hash = format!("{:x}", md5::compute(content));
        active_model.content_hash = sea_orm::Set(Some(content_hash));
        
        active_model.status = sea_orm::Set(document::DocumentStatus::Pending);
        active_model.chunk_count = sea_orm::Set(0);
        active_model.processing_started_at = sea_orm::Set(None);
        active_model.processing_completed_at = sea_orm::Set(None);
        active_model.error_message = sea_orm::Set(None);
        
        if let sea_orm::ActiveValue::Unchanged(version) = &active_model.version {
            active_model.version = sea_orm::Set(version + 1);
        }
    }
    
    if let Some(status) = &req.status {
        active_model.status = sea_orm::Set(status.clone());
    }
    
    if let Some(metadata) = &req.metadata {
        active_model.metadata = sea_orm::Set(serde_json::to_value(metadata).unwrap().into());
    }
    
    if let Some(processing_config) = &req.processing_config {
        active_model.processing_config = sea_orm::Set(serde_json::to_value(processing_config).unwrap().into());
    }
    
    active_model.updated_at = sea_orm::Set(now);
    
    Ok(document::Entity::update(active_model).exec(db).await.map_err(|e| {
        AiStudioError::database(format!("更新文档失败: {}", e))
    })?)
}

/// 批量导入文档
#[utoipa::path(
    post,
    path = "/api/v1/documents/batch-import",
    request_body(content = String, description = "批量文档文件", content_type = "multipart/form-data"),
    responses(
        (status = 202, description = "批量导入已启动", body = BatchImportResponse),
        (status = 400, description = "请求参数错误", body = ApiError),
        (status = 401, description = "未授权", body = ApiError),
        (status = 413, description = "文件过大", body = ApiError),
        (status = 500, description = "服务器内部错误", body = ApiError)
    ),
    tag = "documents",
    security(
        ("bearer_auth" = []),
        ("api_key" = [])
    )
)]
pub async fn batch_import_documents(
    db: web::Data<DatabaseConnection>,
    tenant_info: web::ReqData<TenantInfo>,
    _user_ctx: web::ReqData<UserContext>,
    mut payload: Multipart,
) -> ActixResult<HttpResponse> {
    info!("批量导入文档请求: 租户={}", tenant_info.id);
    
    let import_id = Uuid::new_v4();
    let now = Utc::now();
    let mut uploaded_count = 0u32;
    let mut knowledge_base_id: Option<Uuid> = None;
    let mut options = BatchImportOptions {
        overwrite_existing: false,
        skip_duplicates: true,
        default_doc_type: Some(document::DocumentType::Text),
        batch_size: Some(10),
        async_processing: true,
    };
    
    // 处理 multipart 数据
    while let Some(Ok(mut field)) = payload.next().await {
        let field_name = field.name().to_string();
        
        match field_name.as_str() {
            "knowledge_base_id" => {
                let mut data = Vec::new();
                while let Some(Ok(chunk)) = field.next().await {
                    data.extend_from_slice(&chunk);
                }
                let kb_id_str = String::from_utf8(data).map_err(|e| {
                    error!("知识库 ID 格式错误: {}", e);
                    ApiError::bad_request("知识库 ID 格式错误")
                })?;
                knowledge_base_id = Some(Uuid::parse_str(&kb_id_str).map_err(|e| {
                    error!("知识库 ID 解析失败: {}", e);
                    ApiError::bad_request("无效的知识库 ID 格式")
                })?);
            }
            "options" => {
                let mut data = Vec::new();
                while let Some(Ok(chunk)) = field.next().await {
                    data.extend_from_slice(&chunk);
                }
                let options_str = String::from_utf8(data).map_err(|e| {
                    error!("选项格式错误: {}", e);
                    ApiError::bad_request("选项格式错误")
                })?;
                options = serde_json::from_str(&options_str).map_err(|e| {
                    error!("选项解析失败: {}", e);
                    ApiError::bad_request("无效的选项格式")
                })?;
            }
            "files" => {
                // 处理文件上传
                let file_name = field.content_disposition().get_filename().unwrap_or("unknown").to_string();
                let content_type = field.content_type().map(|ct| ct.to_string());
                
                let mut file_data = Vec::new();
                while let Some(Ok(chunk)) = field.next().await {
                    file_data.extend_from_slice(&chunk);
                    
                    // 限制单个文件大小（例如 50MB）
                    if file_data.len() > 50 * 1024 * 1024 {
                        return Ok(HttpResponseBuilder::payload_too_large::<()>("单个文件大小超过限制（50MB）").unwrap());
                    }
                }
                
                // 这里应该将文件保存到临时位置，并添加到处理队列
                // 目前只是计数
                uploaded_count += 1;
                
                debug!("上传文件: {}, 大小: {}", file_name, file_data.len());
            }
            _ => {
                // 忽略未知字段
                while let Some(_) = field.next().await {}
            }
        }
    }
    
    // 验证必需字段
    let knowledge_base_id = knowledge_base_id.ok_or_else(|| {
        ApiError::bad_request("缺少知识库 ID")
    })?;
    
    // 检查知识库是否存在且属于当前租户
    let kb = KnowledgeBase::find_by_id(knowledge_base_id)
        .filter(knowledge_base::Column::TenantId.eq(tenant_info.id))
        .one(db.as_ref())
        .await
        .map_err(|e| {
            error!("查询知识库失败: {}", e);
            ApiError::internal_server_error("查询知识库失败")
        })?;
    
    if kb.is_none() {
        warn!("知识库不存在或无权访问: {}", knowledge_base_id);
        return Ok(HttpResponseBuilder::not_found::<()>("知识库不存在").unwrap());
    }
    
    // TODO: 这里应该启动异步批量导入任务
    // 目前只是返回导入已启动的响应
    
    info!("批量导入任务已启动: import_id={}, 文件数={}", import_id, uploaded_count);
    
    let response = BatchImportResponse {
        import_id,
        uploaded_count,
        status: "processing".to_string(),
        message: format!("已上传 {} 个文件，正在处理中", uploaded_count),
        started_at: now,
    };
    
    Ok(ApiResponse::accepted(response).into_http_response().unwrap())
}

/// 批量导出文档
#[utoipa::path(
    post,
    path = "/api/v1/documents/batch-export",
    request_body = BatchExportRequest,
    responses(
        (status = 202, description = "批量导出已启动", body = BatchExportResponse),
        (status = 400, description = "请求参数错误", body = ApiError),
        (status = 401, description = "未授权", body = ApiError),
        (status = 403, description = "权限不足", body = ApiError),
        (status = 404, description = "知识库不存在", body = ApiError),
        (status = 500, description = "服务器内部错误", body = ApiError)
    ),
    tag = "documents",
    security(
        ("bearer_auth" = []),
        ("api_key" = [])
    )
)]
pub async fn batch_export_documents(
    db: web::Data<DatabaseConnection>,
    tenant_info: web::ReqData<TenantInfo>,
    _user_ctx: web::ReqData<UserContext>,
    req: web::Json<BatchExportRequest>,
) -> ActixResult<HttpResponse> {
    info!("批量导出文档请求: 租户={}, 知识库={:?}", 
          tenant_info.id, req.knowledge_base_id);
    
    let export_id = Uuid::new_v4();
    let now = Utc::now();
    
    // 构建查询条件
    let mut query = Document::find()
        .inner_join(KnowledgeBase)
        .filter(knowledge_base::Column::TenantId.eq(tenant_info.id));
    
    if let Some(kb_id) = req.knowledge_base_id {
        // 检查知识库是否存在
        let kb = KnowledgeBase::find_by_id(kb_id)
            .filter(knowledge_base::Column::TenantId.eq(tenant_info.id))
            .one(db.as_ref())
            .await
            .map_err(|e| {
                error!("查询知识库失败: {}", e);
                ApiError::internal_server_error("查询知识库失败")
            })?;
        
        if kb.is_none() {
            warn!("知识库不存在或无权访问: {}", kb_id);
            return Ok(HttpResponseBuilder::not_found::<()>("知识库").unwrap());
        }
        
        query = query.filter(document::Column::KnowledgeBaseId.eq(kb_id));
    }
    
    if let Some(doc_ids) = &req.document_ids {
        if !doc_ids.is_empty() {
            query = query.filter(document::Column::Id.is_in(doc_ids.clone()));
        }
    }
    
    // 获取文档数量
    let document_count = query.count(db.as_ref()).await.map_err(|e| {
        error!("查询文档数量失败: {}", e);
        ApiError::internal_server_error("查询文档失败")
    })? as u32;
    
    if document_count == 0 {
        return Ok(HttpResponseBuilder::bad_request::<()>("没有找到要导出的文档".to_string()).unwrap());
    }
    
    // TODO: 这里应该启动异步导出任务
    // 实际的导出逻辑应该根据格式生成相应的文件
    
    let download_url = format!("/api/v1/downloads/export/{}", export_id);
    
    info!("批量导出任务已启动: export_id={}, 文档数={}", export_id, document_count);
    
    let response = BatchExportResponse {
        export_id,
        document_count,
        download_url: Some(download_url),
        status: "processing".to_string(),
        file_size: None,
        started_at: now,
        completed_at: None,
    };
    
    Ok(ApiResponse::accepted(response).into_http_response().unwrap())
}

/// 获取批量操作状态
#[utoipa::path(
    get,
    path = "/api/v1/documents/batch/{batch_id}/status",
    params(
        ("batch_id" = Uuid, Path, description = "批量操作 ID")
    ),
    responses(
        (status = 200, description = "获取批量操作状态成功", body = serde_json::Value),
        (status = 401, description = "未授权", body = ApiError),
        (status = 404, description = "批量操作不存在", body = ApiError),
        (status = 500, description = "服务器内部错误", body = ApiError)
    ),
    tag = "documents",
    security(
        ("bearer_auth" = []),
        ("api_key" = [])
    )
)]
pub async fn get_batch_operation_status(
    _db: web::Data<DatabaseConnection>,
    _tenant_info: web::ReqData<TenantInfo>,
    _user_ctx: web::ReqData<UserContext>,
    path: web::Path<Uuid>,
) -> ActixResult<HttpResponse> {
    let batch_id = path.into_inner();
    debug!("获取批量操作状态: batch_id={}", batch_id);
    
    // TODO: 这里应该从数据库或缓存中查询实际的批量操作状态
    // 目前返回模拟数据
    
    let status = serde_json::json!({
        "batch_id": batch_id,
        "status": "completed",
        "progress": 100,
        "total_count": 10,
        "success_count": 8,
        "error_count": 2,
        "started_at": Utc::now() - chrono::Duration::minutes(5),
        "completed_at": Utc::now(),
        "message": "批量操作已完成"
    });
    
    Ok(ApiResponse::ok(status).into_http_response().unwrap())
}

/// 配置文档路由
pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/documents")
            .route("", web::post().to(create_document))
            .route("", web::get().to(list_documents))
            .route("/upload", web::post().to(upload_document))
            .route("/batch", web::post().to(batch_document_operation))
            .route("/batch-import", web::post().to(batch_import_documents))
            .route("/batch-export", web::post().to(batch_export_documents))
            .route("/batch/{batch_id}/status", web::get().to(get_batch_operation_status))
            .route("/{id}", web::get().to(get_document))
            .route("/{id}", web::put().to(update_document))
            .route("/{id}", web::delete().to(delete_document))
            .route("/{id}/stats", web::get().to(get_document_stats))
            .route("/{id}/reprocess", web::post().to(reprocess_document))
    );
}