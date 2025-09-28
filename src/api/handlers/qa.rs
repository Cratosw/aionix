// 问答 API 处理器
// 实现基于 RAG 的智能问答接口

use actix_web::{web, HttpResponse, Result as ActixResult};
use actix_web_lab::sse::{self, Sse};
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use tracing::{info, error, debug};
use futures::stream::{self, Stream};
use std::time::Duration;

use crate::api::models::{PaginationQuery, PaginatedResponse, PaginationInfo};
use crate::api::responses::{ApiResponse, ApiError};
use crate::api::extractors::{TenantExtractor, UserContext};
use crate::db::migrations::tenant_filter::TenantContext;
use crate::ai::rag_engine::{RagEngine, RagQueryRequest, RagQueryResponse, RetrievalParams, GenerationParams};

/// 问答请求
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct QaRequest {
    /// 用户问题
    pub question: String,
    /// 知识库 ID（可选）
    pub knowledge_base_id: Option<Uuid>,
    /// 会话 ID（用于上下文保持）
    pub session_id: Option<String>,
    /// 检索参数
    pub retrieval_params: Option<RetrievalParams>,
    /// 生成参数
    pub generation_params: Option<GenerationParams>,
    /// 是否启用流式响应
    pub stream: Option<bool>,
}

/// 问答响应
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct QaResponse {
    /// 查询 ID
    pub query_id: String,
    /// 会话 ID
    pub session_id: String,
    /// 生成的答案
    pub answer: String,
    /// 置信度分数
    pub confidence_score: f32,
    /// 来源文档
    pub sources: Vec<QaSource>,
    /// 相关建议
    pub suggestions: Vec<String>,
    /// 查询统计
    pub stats: QaStats,
    /// 响应时间
    pub response_time: DateTime<Utc>,
}

/// 问答来源
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct QaSource {
    /// 文档 ID
    pub document_id: Uuid,
    /// 文档标题
    pub title: String,
    /// 文档类型
    pub doc_type: String,
    /// 相关性分数
    pub relevance_score: f32,
    /// 引用的文档块
    pub chunks: Vec<QaChunk>,
}

/// 问答文档块
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct QaChunk {
    /// 文档块 ID
    pub chunk_id: Uuid,
    /// 文档块内容（可能被截断）
    pub content: String,
    /// 相似度分数
    pub similarity_score: f32,
    /// 块索引
    pub chunk_index: i32,
}

/// 问答统计
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct QaStats {
    /// 总响应时间（毫秒）
    pub response_time_ms: u64,
    /// 检索到的文档数量
    pub documents_retrieved: u32,
    /// 使用的文档块数量
    pub chunks_used: u32,
    /// 生成的 token 数量
    pub tokens_generated: Option<u32>,
}

/// 会话历史请求
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct SessionHistoryQuery {
    /// 会话 ID
    pub session_id: String,
    /// 分页参数
    #[serde(flatten)]
    pub pagination: PaginationQuery,
}

/// 会话消息
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SessionMessage {
    /// 消息 ID
    pub message_id: String,
    /// 消息类型
    pub message_type: MessageType,
    /// 消息内容
    pub content: String,
    /// 时间戳
    pub timestamp: DateTime<Utc>,
    /// 元数据
    pub metadata: Option<serde_json::Value>,
}

/// 消息类型
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum MessageType {
    Question,
    Answer,
    System,
}

/// 流式响应事件
#[derive(Debug, Clone, Serialize)]
pub struct StreamEvent {
    /// 事件类型
    pub event: String,
    /// 事件数据
    pub data: serde_json::Value,
    /// 时间戳
    pub timestamp: DateTime<Utc>,
}

/// 问答反馈请求
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct QaFeedbackRequest {
    /// 查询 ID
    pub query_id: String,
    /// 反馈类型
    pub feedback_type: FeedbackType,
    /// 评分 (1-5)
    pub rating: Option<u8>,
    /// 反馈内容
    pub comment: Option<String>,
    /// 是否有用
    pub helpful: Option<bool>,
}

/// 反馈类型
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum FeedbackType {
    Helpful,
    NotHelpful,
    Incorrect,
    Incomplete,
    Irrelevant,
    Other,
}

/// 问答建议请求
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct QaSuggestionsRequest {
    /// 部分问题文本
    pub partial_question: String,
    /// 知识库 ID（可选）
    pub knowledge_base_id: Option<Uuid>,
    /// 最大建议数量
    pub max_suggestions: Option<u8>,
}

/// 问答建议响应
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct QaSuggestionsResponse {
    /// 建议列表
    pub suggestions: Vec<String>,
    /// 生成时间
    pub generated_at: DateTime<Utc>,
}

/// 执行问答查询
#[utoipa::path(
    post,
    path = "/api/v1/qa/ask",
    request_body = QaRequest,
    responses(
        (status = 200, description = "问答查询成功", body = QaResponse),
        (status = 400, description = "请求参数错误", body = ApiError),
        (status = 401, description = "未授权", body = ApiError),
        (status = 403, description = "权限不足", body = ApiError),
        (status = 500, description = "服务器内部错误", body = ApiError)
    ),
    tag = "qa",
    security(
        ("bearer_auth" = []),
        ("api_key" = [])
    )
)]
pub async fn ask_question(
    db: web::Data<DatabaseConnection>,
    rag_engine: web::Data<RagEngine>,
    tenant_ctx: TenantContext,
    user_ctx: UserContext,
    req: web::Json<QaRequest>,
) -> ActixResult<HttpResponse> {
    info!("问答查询请求: 租户={}, 用户={}, 问题={}", 
          tenant_ctx.tenant.id, user_ctx.user.id, req.question);
    
    if req.question.trim().is_empty() {
        return Ok(HttpResponse::BadRequest().json(ApiError::bad_request("问题不能为空")));
    }
    
    if req.question.len() > 1000 {
        return Ok(HttpResponse::BadRequest().json(ApiError::bad_request("问题长度不能超过 1000 字符")));
    }
    
    // 生成或使用现有的会话 ID
    let session_id = req.session_id.clone().unwrap_or_else(|| {
        format!("session_{}", Uuid::new_v4())
    });
    
    // 构建 RAG 查询请求
    let rag_request = RagQueryRequest {
        question: req.question.clone(),
        knowledge_base_id: req.knowledge_base_id,
        tenant_id: tenant_ctx.tenant.id,
        retrieval_params: req.retrieval_params.clone(),
        generation_params: req.generation_params.clone(),
        session_id: Some(session_id.clone()),
        user_id: Some(user_ctx.user.id),
    };
    
    // 执行 RAG 查询
    let rag_response = rag_engine.query(rag_request).await.map_err(|e| {
        error!("RAG 查询失败: {}", e);
        ApiError::internal_server_error("查询处理失败")
    })?;
    
    // 转换为 API 响应格式
    let sources = convert_to_qa_sources(&rag_response);
    let suggestions = generate_suggestions(&req.question, &rag_response);
    
    let response = QaResponse {
        query_id: rag_response.query_id,
        session_id,
        answer: rag_response.answer,
        confidence_score: rag_response.confidence_score,
        sources,
        suggestions,
        stats: QaStats {
            response_time_ms: rag_response.query_stats.total_time_ms,
            documents_retrieved: rag_response.source_documents.len() as u32,
            chunks_used: rag_response.query_stats.chunks_used_for_generation,
            tokens_generated: rag_response.query_stats.tokens_generated,
        },
        response_time: rag_response.generated_at,
    };
    
    // TODO: 保存会话历史到数据库
    
    info!("问答查询完成: query_id={}, 置信度={:.2}, 耗时={}ms", 
          response.query_id, response.confidence_score, response.stats.response_time_ms);
    
    Ok(HttpResponse::Ok().json(ApiResponse::ok(response)))
}

/// 流式问答查询
#[utoipa::path(
    post,
    path = "/api/v1/qa/ask-stream",
    request_body = QaRequest,
    responses(
        (status = 200, description = "流式问答查询", content_type = "text/event-stream"),
        (status = 400, description = "请求参数错误", body = ApiError),
        (status = 401, description = "未授权", body = ApiError),
        (status = 500, description = "服务器内部错误", body = ApiError)
    ),
    tag = "qa",
    security(
        ("bearer_auth" = []),
        ("api_key" = [])
    )
)]
pub async fn ask_question_stream(
    db: web::Data<DatabaseConnection>,
    rag_engine: web::Data<RagEngine>,
    tenant_ctx: TenantContext,
    user_ctx: UserContext,
    req: web::Json<QaRequest>,
) -> ActixResult<HttpResponse> {
    info!("流式问答查询请求: 租户={}, 用户={}, 问题={}", 
          tenant_ctx.tenant.id, user_ctx.user.id, req.question);
    
    if req.question.trim().is_empty() {
        return Ok(HttpResponse::BadRequest().json(ApiError::bad_request("问题不能为空")));
    }
    
    let session_id = req.session_id.clone().unwrap_or_else(|| {
        format!("session_{}", Uuid::new_v4())
    });
    
    // 创建流式响应
    let stream = create_qa_stream(
        rag_engine.get_ref().clone(),
        req.into_inner(),
        tenant_ctx.tenant.id,
        user_ctx.user.id,
        session_id,
    );
    
    Ok(Sse::from_stream(stream)
        .with_keep_alive(Duration::from_secs(30))
        .into_response())
}

/// 获取会话历史
#[utoipa::path(
    get,
    path = "/api/v1/qa/sessions/{session_id}/history",
    params(
        ("session_id" = String, Path, description = "会话 ID"),
        SessionHistoryQuery
    ),
    responses(
        (status = 200, description = "获取会话历史成功", body = PaginatedResponse<SessionMessage>),
        (status = 401, description = "未授权", body = ApiError),
        (status = 404, description = "会话不存在", body = ApiError),
        (status = 500, description = "服务器内部错误", body = ApiError)
    ),
    tag = "qa",
    security(
        ("bearer_auth" = []),
        ("api_key" = [])
    )
)]
pub async fn get_session_history(
    db: web::Data<DatabaseConnection>,
    tenant_ctx: TenantContext,
    user_ctx: UserContext,
    path: web::Path<String>,
    query: web::Query<SessionHistoryQuery>,
) -> ActixResult<HttpResponse> {
    let session_id = path.into_inner();
    debug!("获取会话历史: session_id={}, 租户={}", session_id, tenant_ctx.tenant.id);
    
    // TODO: 从数据库查询会话历史
    // 目前返回模拟数据
    
    let messages = vec![
        SessionMessage {
            message_id: format!("msg_{}", Uuid::new_v4()),
            message_type: MessageType::Question,
            content: "什么是人工智能？".to_string(),
            timestamp: Utc::now() - chrono::Duration::minutes(5),
            metadata: None,
        },
        SessionMessage {
            message_id: format!("msg_{}", Uuid::new_v4()),
            message_type: MessageType::Answer,
            content: "人工智能是计算机科学的一个分支...".to_string(),
            timestamp: Utc::now() - chrono::Duration::minutes(4),
            metadata: Some(serde_json::json!({
                "confidence_score": 0.85,
                "sources_count": 3
            })),
        },
    ];
    
    let pagination = PaginationInfo::new(
        query.pagination.page,
        query.pagination.page_size,
        messages.len() as u64,
    );
    
    let response = PaginatedResponse::new(messages, pagination);
    Ok(HttpResponse::Ok().json(ApiResponse::ok(response)))
}

/// 提交问答反馈
#[utoipa::path(
    post,
    path = "/api/v1/qa/feedback",
    request_body = QaFeedbackRequest,
    responses(
        (status = 200, description = "反馈提交成功", body = serde_json::Value),
        (status = 400, description = "请求参数错误", body = ApiError),
        (status = 401, description = "未授权", body = ApiError),
        (status = 500, description = "服务器内部错误", body = ApiError)
    ),
    tag = "qa",
    security(
        ("bearer_auth" = []),
        ("api_key" = [])
    )
)]
pub async fn submit_feedback(
    db: web::Data<DatabaseConnection>,
    tenant_ctx: TenantContext,
    user_ctx: UserContext,
    req: web::Json<QaFeedbackRequest>,
) -> ActixResult<HttpResponse> {
    info!("提交问答反馈: query_id={}, 类型={:?}, 用户={}", 
          req.query_id, req.feedback_type, user_ctx.user.id);
    
    if let Some(rating) = req.rating {
        if rating < 1 || rating > 5 {
            return Ok(ApiError::bad_request("评分必须在 1-5 之间").into());
        }
    }
    
    // TODO: 保存反馈到数据库
    
    let response = serde_json::json!({
        "message": "反馈提交成功",
        "feedback_id": Uuid::new_v4(),
        "submitted_at": Utc::now()
    });
    
    Ok(HttpResponse::Ok().json(ApiResponse::ok(response)))
}

/// 获取问题建议
#[utoipa::path(
    post,
    path = "/api/v1/qa/suggestions",
    request_body = QaSuggestionsRequest,
    responses(
        (status = 200, description = "获取建议成功", body = QaSuggestionsResponse),
        (status = 400, description = "请求参数错误", body = ApiError),
        (status = 401, description = "未授权", body = ApiError),
        (status = 500, description = "服务器内部错误", body = ApiError)
    ),
    tag = "qa",
    security(
        ("bearer_auth" = []),
        ("api_key" = [])
    )
)]
pub async fn get_suggestions(
    db: web::Data<DatabaseConnection>,
    tenant_ctx: TenantContext,
    _user_ctx: UserContext,
    req: web::Json<QaSuggestionsRequest>,
) -> ActixResult<HttpResponse> {
    debug!("获取问题建议: 部分问题={}, 租户={}", 
           req.partial_question, tenant_ctx.tenant.id);
    
    if req.partial_question.trim().is_empty() {
        return Ok(ApiError::bad_request("部分问题不能为空").into());
    }
    
    let max_suggestions = req.max_suggestions.unwrap_or(5).min(10);
    
    // TODO: 基于历史查询和知识库内容生成智能建议
    // 目前返回模拟建议
    
    let suggestions = vec![
        format!("{}相关的最新发展是什么？", req.partial_question),
        format!("{}的主要特点有哪些？", req.partial_question),
        format!("如何理解{}的概念？", req.partial_question),
        format!("{}在实际应用中的案例", req.partial_question),
        format!("{}的优缺点分析", req.partial_question),
    ].into_iter()
    .take(max_suggestions as usize)
    .collect();
    
    let response = QaSuggestionsResponse {
        suggestions,
        generated_at: Utc::now(),
    };
    
    Ok(HttpResponse::Ok().json(ApiResponse::ok(response)))
}

/// 转换 RAG 响应为 QA 来源格式
fn convert_to_qa_sources(rag_response: &RagQueryResponse) -> Vec<QaSource> {
    let mut sources = Vec::new();
    
    // 按文档分组
    let mut doc_chunks: std::collections::HashMap<Uuid, Vec<&crate::ai::rag_engine::RetrievedChunk>> = 
        std::collections::HashMap::new();
    
    for chunk in &rag_response.retrieved_chunks {
        doc_chunks.entry(chunk.document_id)
            .or_insert_with(Vec::new)
            .push(chunk);
    }
    
    // 构建来源信息
    for source_doc in &rag_response.source_documents {
        if let Some(chunks) = doc_chunks.get(&source_doc.document_id) {
            let qa_chunks: Vec<QaChunk> = chunks.iter().map(|chunk| {
                QaChunk {
                    chunk_id: chunk.chunk_id,
                    content: if chunk.content.len() > 200 {
                        format!("{}...", &chunk.content[..200])
                    } else {
                        chunk.content.clone()
                    },
                    similarity_score: chunk.similarity_score,
                    chunk_index: chunk.chunk_index,
                }
            }).collect();
            
            sources.push(QaSource {
                document_id: source_doc.document_id,
                title: source_doc.title.clone(),
                doc_type: source_doc.doc_type.clone(),
                relevance_score: source_doc.relevance_score,
                chunks: qa_chunks,
            });
        }
    }
    
    sources
}

/// 生成相关建议
fn generate_suggestions(question: &str, rag_response: &RagQueryResponse) -> Vec<String> {
    let mut suggestions = Vec::new();
    
    // 基于置信度和来源文档生成建议
    if rag_response.confidence_score < 0.7 {
        suggestions.push("您可以尝试更具体的问题描述".to_string());
    }
    
    if !rag_response.source_documents.is_empty() {
        suggestions.push("您可能还想了解相关文档的更多内容".to_string());
    }
    
    // 基于问题内容生成建议
    if question.contains("什么") {
        suggestions.push("您可以询问具体的应用场景或实例".to_string());
    }
    
    if question.contains("如何") {
        suggestions.push("您可以询问相关的步骤或方法".to_string());
    }
    
    suggestions.truncate(3); // 最多返回 3 个建议
    suggestions
}

/// 创建流式问答响应
fn create_qa_stream(
    rag_engine: RagEngine,
    request: QaRequest,
    tenant_id: Uuid,
    user_id: Uuid,
    session_id: String,
) -> impl Stream<Item = Result<sse::Event, actix_web::Error>> {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    
    // 在后台任务中执行 RAG 查询
    tokio::spawn(async move {
        // 发送开始事件
        let start_event = StreamEvent {
            event: "start".to_string(),
            data: serde_json::json!({
                "session_id": session_id,
                "message": "开始处理您的问题..."
            }),
            timestamp: Utc::now(),
        };
        
        if let Ok(event_data) = serde_json::to_string(&start_event) {
            let _ = tx.send(Ok(sse::Event::default().data(event_data)));
        }
        
        // 发送检索事件
        let retrieval_event = StreamEvent {
            event: "retrieval".to_string(),
            data: serde_json::json!({
                "message": "正在检索相关文档..."
            }),
            timestamp: Utc::now(),
        };
        
        if let Ok(event_data) = serde_json::to_string(&retrieval_event) {
            let _ = tx.send(Ok(sse::Event::default().data(event_data)));
        }
        
        // 构建 RAG 查询请求
        let rag_request = RagQueryRequest {
            question: request.question.clone(),
            knowledge_base_id: request.knowledge_base_id,
            tenant_id,
            retrieval_params: request.retrieval_params,
            generation_params: request.generation_params,
            session_id: Some(session_id.clone()),
            user_id: Some(user_id),
        };
        
        // 执行 RAG 查询
        match rag_engine.query(rag_request).await {
            Ok(rag_response) => {
                // 发送生成事件
                let generation_event = StreamEvent {
                    event: "generation".to_string(),
                    data: serde_json::json!({
                        "message": "正在生成答案..."
                    }),
                    timestamp: Utc::now(),
                };
                
                if let Ok(event_data) = serde_json::to_string(&generation_event) {
                    let _ = tx.send(Ok(sse::Event::default().data(event_data)));
                }
                
                // 模拟流式输出答案
                let words: Vec<&str> = rag_response.answer.split_whitespace().collect();
                let mut current_answer = String::new();
                
                for (i, word) in words.iter().enumerate() {
                    current_answer.push_str(word);
                    if i < words.len() - 1 {
                        current_answer.push(' ');
                    }
                    
                    let chunk_event = StreamEvent {
                        event: "chunk".to_string(),
                        data: serde_json::json!({
                            "content": word,
                            "partial_answer": current_answer
                        }),
                        timestamp: Utc::now(),
                    };
                    
                    if let Ok(event_data) = serde_json::to_string(&chunk_event) {
                        let _ = tx.send(Ok(sse::Event::default().data(event_data)));
                    }
                    
                    // 模拟打字效果
                    tokio::time::sleep(Duration::from_millis(50)).await;
                }
                
                // 发送完成事件
                let sources = convert_to_qa_sources(&rag_response);
                let suggestions = generate_suggestions(&request.question, &rag_response);
                
                let complete_event = StreamEvent {
                    event: "complete".to_string(),
                    data: serde_json::json!({
                        "query_id": rag_response.query_id,
                        "answer": rag_response.answer,
                        "confidence_score": rag_response.confidence_score,
                        "sources": sources,
                        "suggestions": suggestions,
                        "stats": {
                            "response_time_ms": rag_response.query_stats.total_time_ms,
                            "documents_retrieved": rag_response.source_documents.len(),
                            "chunks_used": rag_response.query_stats.chunks_used_for_generation
                        }
                    }),
                    timestamp: Utc::now(),
                };
                
                if let Ok(event_data) = serde_json::to_string(&complete_event) {
                    let _ = tx.send(Ok(sse::Event::default().data(event_data)));
                }
            }
            Err(e) => {
                // 发送错误事件
                let error_event = StreamEvent {
                    event: "error".to_string(),
                    data: serde_json::json!({
                        "error": e.to_string(),
                        "message": "处理您的问题时发生错误"
                    }),
                    timestamp: Utc::now(),
                };
                
                if let Ok(event_data) = serde_json::to_string(&error_event) {
                    let _ = tx.send(Ok(sse::Event::default().data(event_data)));
                }
            }
        }
    });
    
    tokio_stream::wrappers::UnboundedReceiverStream::new(rx)
}

/// 配置问答路由
pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/qa")
            .route("/ask", web::post().to(ask_question))
            .route("/ask-stream", web::post().to(ask_question_stream))
            .route("/sessions/{session_id}/history", web::get().to(get_session_history))
            .route("/feedback", web::post().to(submit_feedback))
            .route("/suggestions", web::post().to(get_suggestions))
    );
}