// RAG (检索增强生成) 查询引擎
// 实现问题向量化、文档检索和答案生成的完整流程

use std::sync::Arc;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{info, warn, error, debug};
use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait, QueryOrder};

use crate::ai::{RigAiClientManager, vector_search::VectorSearchService, chunker::ChunkerService};
use crate::db::entities::{knowledge_base, document, document_chunk, prelude::*};
use crate::errors::AiStudioError;
use crate::services::knowledge_base::KnowledgeBaseService;

/// RAG 查询请求
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RagQueryRequest {
    /// 查询问题
    pub question: String,
    /// 知识库 ID（可选，如果不指定则搜索所有知识库）
    pub knowledge_base_id: Option<Uuid>,
    /// 租户 ID
    pub tenant_id: Uuid,
    /// 检索参数
    pub retrieval_params: Option<RetrievalParams>,
    /// 生成参数
    pub generation_params: Option<GenerationParams>,
    /// 会话 ID（用于上下文保持）
    pub session_id: Option<String>,
    /// 用户 ID
    pub user_id: Option<Uuid>,
}

/// 检索参数
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RetrievalParams {
    /// 检索的文档块数量
    pub top_k: Option<u32>,
    /// 相似度阈值
    pub similarity_threshold: Option<f32>,
    /// 检索方法
    pub retrieval_method: Option<String>,
    /// 是否启用重排序
    pub enable_reranking: Option<bool>,
    /// 文档类型过滤
    pub document_types: Option<Vec<String>>,
    /// 时间范围过滤
    pub date_range: Option<DateRange>,
}

/// 生成参数
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GenerationParams {
    /// 最大生成长度
    pub max_length: Option<u32>,
    /// 温度参数
    pub temperature: Option<f32>,
    /// 是否包含来源引用
    pub include_sources: Option<bool>,
    /// 答案语言
    pub language: Option<String>,
    /// 生成风格
    pub style: Option<String>,
}

/// 时间范围
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DateRange {
    /// 开始时间
    pub start: DateTime<Utc>,
    /// 结束时间
    pub end: DateTime<Utc>,
}

/// RAG 查询响应
#[derive(Debug, Clone, Serialize)]
pub struct RagQueryResponse {
    /// 查询 ID
    pub query_id: String,
    /// 生成的答案
    pub answer: String,
    /// 置信度分数 (0.0-1.0)
    pub confidence_score: f32,
    /// 检索到的文档块
    pub retrieved_chunks: Vec<RetrievedChunk>,
    /// 来源文档
    pub source_documents: Vec<SourceDocument>,
    /// 查询统计信息
    pub query_stats: QueryStats,
    /// 生成时间
    pub generated_at: DateTime<Utc>,
}

/// 检索到的文档块
#[derive(Debug, Clone, Serialize)]
pub struct RetrievedChunk {
    /// 文档块 ID
    pub chunk_id: Uuid,
    /// 文档 ID
    pub document_id: Uuid,
    /// 文档块内容
    pub content: String,
    /// 相似度分数
    pub similarity_score: f32,
    /// 文档块位置
    pub chunk_index: i32,
    /// 元数据
    pub metadata: serde_json::Value,
}

/// 来源文档
#[derive(Debug, Clone, Serialize)]
pub struct SourceDocument {
    /// 文档 ID
    pub document_id: Uuid,
    /// 文档标题
    pub title: String,
    /// 文档类型
    pub doc_type: String,
    /// 相关性分数
    pub relevance_score: f32,
    /// 引用的文档块数量
    pub chunk_count: u32,
}

/// 查询统计信息
#[derive(Debug, Clone, Serialize)]
pub struct QueryStats {
    /// 向量化耗时（毫秒）
    pub vectorization_time_ms: u64,
    /// 检索耗时（毫秒）
    pub retrieval_time_ms: u64,
    /// 生成耗时（毫秒）
    pub generation_time_ms: u64,
    /// 总耗时（毫秒）
    pub total_time_ms: u64,
    /// 检索到的文档块总数
    pub total_chunks_retrieved: u32,
    /// 使用的文档块数量
    pub chunks_used_for_generation: u32,
    /// 生成的 token 数量
    pub tokens_generated: Option<u32>,
}

/// RAG 引擎配置
#[derive(Debug, Clone)]
pub struct RagEngineConfig {
    /// 默认检索数量
    pub default_top_k: u32,
    /// 默认相似度阈值
    pub default_similarity_threshold: f32,
    /// 最大上下文长度
    pub max_context_length: u32,
    /// 是否启用缓存
    pub enable_caching: bool,
    /// 缓存过期时间（秒）
    pub cache_ttl_seconds: u64,
    /// 是否启用查询日志
    pub enable_query_logging: bool,
}

impl Default for RagEngineConfig {
    fn default() -> Self {
        Self {
            default_top_k: 5,
            default_similarity_threshold: 0.7,
            max_context_length: 4000,
            enable_caching: true,
            cache_ttl_seconds: 3600,
            enable_query_logging: true,
        }
    }
}

/// RAG 查询引擎
pub struct RagEngine {
    /// AI 客户端管理器
    ai_client: Arc<RigAiClientManager>,
    /// 数据库连接
    db: Arc<DatabaseConnection>,
    /// 向量搜索服务
    vector_search: Arc<VectorSearchService>,
    /// 知识库服务
    kb_service: Arc<dyn KnowledgeBaseService>,
    /// 引擎配置
    config: RagEngineConfig,
}

impl RagEngine {
    /// 创建新的 RAG 引擎
    pub fn new(
        ai_client: Arc<RigAiClientManager>,
        db: Arc<DatabaseConnection>,
        vector_search: Arc<VectorSearchService>,
        kb_service: Arc<dyn KnowledgeBaseService>,
        config: Option<RagEngineConfig>,
    ) -> Self {
        Self {
            ai_client,
            db,
            vector_search,
            kb_service,
            config: config.unwrap_or_default(),
        }
    }
    
    /// 执行 RAG 查询
    pub async fn query(&self, request: RagQueryRequest) -> Result<RagQueryResponse, AiStudioError> {
        let query_id = format!("rag_{}", Uuid::new_v4());
        let start_time = std::time::Instant::now();
        
        info!("开始 RAG 查询: query_id={}, question={}", query_id, request.question);
        
        // 1. 问题向量化
        let vectorization_start = std::time::Instant::now();
        let question_embedding = self.vectorize_question(&request.question).await?;
        let vectorization_time = vectorization_start.elapsed().as_millis() as u64;
        
        // 2. 检索相关文档块
        let retrieval_start = std::time::Instant::now();
        let retrieved_chunks = self.retrieve_relevant_chunks(
            &request,
            &question_embedding,
        ).await?;
        let retrieval_time = retrieval_start.elapsed().as_millis() as u64;
        
        if retrieved_chunks.is_empty() {
            warn!("未找到相关文档块: query_id={}", query_id);
            return Ok(RagQueryResponse {
                query_id,
                answer: "抱歉，我没有找到相关的信息来回答您的问题。".to_string(),
                confidence_score: 0.0,
                retrieved_chunks: Vec::new(),
                source_documents: Vec::new(),
                query_stats: QueryStats {
                    vectorization_time_ms: vectorization_time,
                    retrieval_time_ms: retrieval_time,
                    generation_time_ms: 0,
                    total_time_ms: start_time.elapsed().as_millis() as u64,
                    total_chunks_retrieved: 0,
                    chunks_used_for_generation: 0,
                    tokens_generated: None,
                },
                generated_at: Utc::now(),
            });
        }
        
        // 3. 构建上下文
        let context = self.build_context(&retrieved_chunks, &request).await?;
        
        // 4. 生成答案
        let generation_start = std::time::Instant::now();
        let (answer, confidence_score, tokens_generated) = self.generate_answer(
            &request.question,
            &context,
            &request.generation_params.unwrap_or_default(),
        ).await?;
        let generation_time = generation_start.elapsed().as_millis() as u64;
        
        // 5. 构建来源文档信息
        let source_documents = self.build_source_documents(&retrieved_chunks).await?;
        
        let total_time = start_time.elapsed().as_millis() as u64;
        
        let response = RagQueryResponse {
            query_id,
            answer,
            confidence_score,
            retrieved_chunks,
            source_documents,
            query_stats: QueryStats {
                vectorization_time_ms: vectorization_time,
                retrieval_time_ms: retrieval_time,
                generation_time_ms: generation_time,
                total_time_ms: total_time,
                total_chunks_retrieved: retrieved_chunks.len() as u32,
                chunks_used_for_generation: retrieved_chunks.len() as u32,
                tokens_generated,
            },
            generated_at: Utc::now(),
        };
        
        info!("RAG 查询完成: query_id={}, 耗时={}ms, 置信度={:.2}", 
              response.query_id, total_time, confidence_score);
        
        // 记录查询日志
        if self.config.enable_query_logging {
            self.log_query(&request, &response).await?;
        }
        
        Ok(response)
    }
    
    /// 向量化问题
    async fn vectorize_question(&self, question: &str) -> Result<Vec<f32>, AiStudioError> {
        debug!("向量化问题: {}", question);
        
        let embedding_response = self.ai_client.generate_embedding(question).await?;
        Ok(embedding_response.embedding)
    }
    
    /// 检索相关文档块
    async fn retrieve_relevant_chunks(
        &self,
        request: &RagQueryRequest,
        question_embedding: &[f32],
    ) -> Result<Vec<RetrievedChunk>, AiStudioError> {
        debug!("检索相关文档块: 租户={}, 知识库={:?}", 
               request.tenant_id, request.knowledge_base_id);
        
        let params = request.retrieval_params.as_ref();
        let top_k = params.and_then(|p| p.top_k).unwrap_or(self.config.default_top_k);
        let similarity_threshold = params.and_then(|p| p.similarity_threshold)
            .unwrap_or(self.config.default_similarity_threshold);
        
        // 使用向量搜索服务检索相似文档块
        let search_results = self.vector_search.search_similar_chunks(
            question_embedding,
            request.tenant_id,
            request.knowledge_base_id,
            top_k,
            similarity_threshold,
        ).await?;
        
        // 转换为 RetrievedChunk 格式
        let mut retrieved_chunks = Vec::new();
        for result in search_results {
            // 查询文档块详细信息
            if let Some(chunk) = DocumentChunk::find_by_id(result.chunk_id)
                .one(self.db.as_ref())
                .await
                .map_err(|e| AiStudioError::database(format!("查询文档块失败: {}", e)))?
            {
                retrieved_chunks.push(RetrievedChunk {
                    chunk_id: chunk.id,
                    document_id: chunk.document_id,
                    content: chunk.content,
                    similarity_score: result.similarity_score,
                    chunk_index: chunk.chunk_index,
                    metadata: chunk.metadata,
                });
            }
        }
        
        debug!("检索到 {} 个相关文档块", retrieved_chunks.len());
        Ok(retrieved_chunks)
    }
    
    /// 构建上下文
    async fn build_context(
        &self,
        chunks: &[RetrievedChunk],
        request: &RagQueryRequest,
    ) -> Result<String, AiStudioError> {
        debug!("构建上下文，文档块数量: {}", chunks.len());
        
        let mut context_parts = Vec::new();
        let mut total_length = 0;
        
        for (i, chunk) in chunks.iter().enumerate() {
            let chunk_text = format!("文档片段 {}:\n{}\n", i + 1, chunk.content);
            
            // 检查是否超过最大上下文长度
            if total_length + chunk_text.len() > self.config.max_context_length as usize {
                debug!("达到最大上下文长度限制，停止添加文档块");
                break;
            }
            
            context_parts.push(chunk_text);
            total_length += chunk_text.len();
        }
        
        let context = context_parts.join("\n");
        debug!("构建的上下文长度: {} 字符", context.len());
        
        Ok(context)
    }
    
    /// 生成答案
    async fn generate_answer(
        &self,
        question: &str,
        context: &str,
        params: &GenerationParams,
    ) -> Result<(String, f32, Option<u32>), AiStudioError> {
        debug!("生成答案，问题: {}", question);
        
        let include_sources = params.include_sources.unwrap_or(true);
        let language = params.language.as_deref().unwrap_or("中文");
        let style = params.style.as_deref().unwrap_or("专业且友好");
        
        let prompt = self.build_generation_prompt(question, context, include_sources, language, style);
        
        let response = self.ai_client.generate_text(&prompt).await?;
        
        // 计算置信度（简单实现，可以根据实际需要改进）
        let confidence_score = self.calculate_confidence_score(&response.text, context);
        
        Ok((response.text, confidence_score, response.tokens_used))
    }
    
    /// 构建生成提示词
    fn build_generation_prompt(
        &self,
        question: &str,
        context: &str,
        include_sources: bool,
        language: &str,
        style: &str,
    ) -> String {
        let source_instruction = if include_sources {
            "请在答案中标注信息来源（如：根据文档片段1...）。"
        } else {
            ""
        };
        
        format!(
            r#"你是一个专业的AI助手，请根据提供的文档内容回答用户的问题。

## 指导原则：
1. 仅基于提供的文档内容回答问题
2. 如果文档中没有相关信息，请明确说明
3. 保持回答的准确性和客观性
4. 使用{}语言回答
5. 回答风格：{}
6. {}

## 文档内容：
{}

## 用户问题：
{}

## 回答：
"#,
            language, style, source_instruction, context, question
        )
    }
    
    /// 计算置信度分数
    fn calculate_confidence_score(&self, answer: &str, context: &str) -> f32 {
        // 简单的置信度计算实现
        // 实际应用中可以使用更复杂的算法
        
        if answer.contains("没有找到") || answer.contains("不确定") || answer.contains("无法回答") {
            return 0.3;
        }
        
        if answer.contains("根据文档") || answer.contains("文档片段") {
            return 0.9;
        }
        
        // 基于答案长度和上下文相关性的简单评分
        let answer_length = answer.len();
        let context_length = context.len();
        
        if answer_length > 50 && context_length > 100 {
            0.8
        } else if answer_length > 20 {
            0.6
        } else {
            0.4
        }
    }
    
    /// 构建来源文档信息
    async fn build_source_documents(
        &self,
        chunks: &[RetrievedChunk],
    ) -> Result<Vec<SourceDocument>, AiStudioError> {
        debug!("构建来源文档信息");
        
        let mut document_map: std::collections::HashMap<Uuid, (document::Model, Vec<&RetrievedChunk>)> = 
            std::collections::HashMap::new();
        
        // 按文档 ID 分组文档块
        for chunk in chunks {
            if let Some(doc) = Document::find_by_id(chunk.document_id)
                .one(self.db.as_ref())
                .await
                .map_err(|e| AiStudioError::database(format!("查询文档失败: {}", e)))?
            {
                document_map.entry(chunk.document_id)
                    .or_insert((doc, Vec::new()))
                    .1
                    .push(chunk);
            }
        }
        
        // 构建来源文档列表
        let mut source_documents = Vec::new();
        for (doc, doc_chunks) in document_map.values() {
            let relevance_score = doc_chunks.iter()
                .map(|chunk| chunk.similarity_score)
                .fold(0.0, |acc, score| acc.max(score));
            
            source_documents.push(SourceDocument {
                document_id: doc.id,
                title: doc.title.clone(),
                doc_type: format!("{:?}", doc.doc_type),
                relevance_score,
                chunk_count: doc_chunks.len() as u32,
            });
        }
        
        // 按相关性分数排序
        source_documents.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap());
        
        Ok(source_documents)
    }
    
    /// 记录查询日志
    async fn log_query(
        &self,
        request: &RagQueryRequest,
        response: &RagQueryResponse,
    ) -> Result<(), AiStudioError> {
        // TODO: 实现查询日志记录
        // 可以记录到数据库或日志文件中，用于分析和优化
        
        debug!("记录查询日志: query_id={}, 耗时={}ms", 
               response.query_id, response.query_stats.total_time_ms);
        
        Ok(())
    }
}

impl Default for RetrievalParams {
    fn default() -> Self {
        Self {
            top_k: Some(5),
            similarity_threshold: Some(0.7),
            retrieval_method: Some("cosine".to_string()),
            enable_reranking: Some(false),
            document_types: None,
            date_range: None,
        }
    }
}

impl Default for GenerationParams {
    fn default() -> Self {
        Self {
            max_length: Some(1000),
            temperature: Some(0.7),
            include_sources: Some(true),
            language: Some("中文".to_string()),
            style: Some("专业且友好".to_string()),
        }
    }
}

/// RAG 引擎工厂
pub struct RagEngineFactory;

impl RagEngineFactory {
    /// 创建 RAG 引擎实例
    pub fn create(
        ai_client: Arc<RigAiClientManager>,
        db: Arc<DatabaseConnection>,
        vector_search: Arc<VectorSearchService>,
        kb_service: Arc<dyn KnowledgeBaseService>,
        config: Option<RagEngineConfig>,
    ) -> Arc<RagEngine> {
        Arc::new(RagEngine::new(
            ai_client,
            db,
            vector_search,
            kb_service,
            config,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_confidence_score_calculation() {
        let engine = RagEngine::new(
            Arc::new(todo!()), // 需要模拟的 AI 客户端
            Arc::new(todo!()), // 需要模拟的数据库连接
            Arc::new(todo!()), // 需要模拟的向量搜索服务
            Arc::new(todo!()), // 需要模拟的知识库服务
            None,
        );
        
        // 测试不同类型答案的置信度计算
        assert_eq!(engine.calculate_confidence_score("没有找到相关信息", "context"), 0.3);
        assert_eq!(engine.calculate_confidence_score("根据文档片段1，答案是...", "context"), 0.9);
    }
    
    #[test]
    fn test_generation_prompt_building() {
        let engine = RagEngine::new(
            Arc::new(todo!()),
            Arc::new(todo!()),
            Arc::new(todo!()),
            Arc::new(todo!()),
            None,
        );
        
        let prompt = engine.build_generation_prompt(
            "什么是人工智能？",
            "人工智能是计算机科学的一个分支...",
            true,
            "中文",
            "专业",
        );
        
        assert!(prompt.contains("什么是人工智能？"));
        assert!(prompt.contains("人工智能是计算机科学的一个分支"));
        assert!(prompt.contains("标注信息来源"));
    }
}