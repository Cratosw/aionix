// 向量嵌入仓储实现

use crate::db::entities::{embedding, prelude::*};
use crate::errors::AiStudioError;
use sea_orm::{prelude::*, *};
use uuid::Uuid;
use tracing::{info, warn, instrument};

/// 向量嵌入仓储
pub struct EmbeddingRepository;

impl EmbeddingRepository {
    /// 创建新向量嵌入
    #[instrument(skip(db, source_text, vector))]
    pub async fn create(
        db: &DatabaseConnection,
        chunk_id: Uuid,
        document_id: Uuid,
        knowledge_base_id: Uuid,
        embedding_type: embedding::EmbeddingType,
        source_text: String,
        text_hash: String,
        vector: Option<Vec<f32>>,
        dimension: i32,
        model_name: String,
        model_version: String,
    ) -> Result<embedding::Model, AiStudioError> {
        info!(chunk_id = %chunk_id, model = %model_name, "创建新向量嵌入");

        // 转换向量为字符串格式
        let vector_str = if let Some(vec) = vector {
            Some(format!("[{}]", 
                vec.iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            ))
        } else {
            None
        };

        let embedding = embedding::ActiveModel {
            id: Set(Uuid::new_v4()),
            chunk_id: Set(chunk_id),
            document_id: Set(document_id),
            knowledge_base_id: Set(knowledge_base_id),
            embedding_type: Set(embedding_type),
            status: Set(embedding::EmbeddingStatus::Pending),
            vector: Set(vector_str),
            dimension: Set(dimension),
            model_name: Set(model_name),
            model_version: Set(model_version),
            source_text: Set(source_text),
            text_hash: Set(text_hash),
            metadata: Set(serde_json::to_value(embedding::EmbeddingMetadata::default())?),
            processing_started_at: Set(None),
            processing_completed_at: Set(None),
            error_message: Set(None),
            created_at: Set(chrono::Utc::now().into()),
            updated_at: Set(chrono::Utc::now().into()),
        };

        let result = embedding.insert(db).await?;
        info!(embedding_id = %result.id, "向量嵌入创建成功");
        Ok(result)
    }

    /// 根据 ID 查找向量嵌入
    #[instrument(skip(db))]
    pub async fn find_by_id(
        db: &DatabaseConnection,
        id: Uuid,
    ) -> Result<Option<embedding::Model>, AiStudioError> {
        let embedding = Embedding::find_by_id(id).one(db).await?;
        Ok(embedding)
    }

    /// 根据文档块 ID 查找向量嵌入
    #[instrument(skip(db))]
    pub async fn find_by_chunk(
        db: &DatabaseConnection,
        chunk_id: Uuid,
    ) -> Result<Vec<embedding::Model>, AiStudioError> {
        let embeddings = Embedding::find()
            .filter(embedding::Column::ChunkId.eq(chunk_id))
            .order_by_desc(embedding::Column::CreatedAt)
            .all(db)
            .await?;
        Ok(embeddings)
    }

    /// 根据文本哈希查找向量嵌入
    #[instrument(skip(db))]
    pub async fn find_by_text_hash(
        db: &DatabaseConnection,
        knowledge_base_id: Uuid,
        text_hash: &str,
        model_name: &str,
        model_version: &str,
    ) -> Result<Option<embedding::Model>, AiStudioError> {
        let embedding = Embedding::find()
            .filter(embedding::Column::KnowledgeBaseId.eq(knowledge_base_id))
            .filter(embedding::Column::TextHash.eq(text_hash))
            .filter(embedding::Column::ModelName.eq(model_name))
            .filter(embedding::Column::ModelVersion.eq(model_version))
            .one(db)
            .await?;
        Ok(embedding)
    }

    /// 更新向量嵌入状态
    #[instrument(skip(db))]
    pub async fn update_status(
        db: &DatabaseConnection,
        id: Uuid,
        status: embedding::EmbeddingStatus,
        error_message: Option<String>,
    ) -> Result<embedding::Model, AiStudioError> {
        info!(embedding_id = %id, status = ?status, "更新向量嵌入状态");

        let embedding = Self::find_by_id(db, id).await?
            .ok_or_else(|| AiStudioError::not_found("向量嵌入"))?;

        let mut active_model: embedding::ActiveModel = embedding.into();
        active_model.status = Set(status.clone());
        active_model.error_message = Set(error_message);
        active_model.updated_at = Set(chrono::Utc::now().into());

        // 设置处理时间
        match status {
            embedding::EmbeddingStatus::Processing => {
                active_model.processing_started_at = Set(Some(chrono::Utc::now().into()));
            }
            embedding::EmbeddingStatus::Completed | embedding::EmbeddingStatus::Failed => {
                active_model.processing_completed_at = Set(Some(chrono::Utc::now().into()));
            }
            _ => {}
        }

        let result = active_model.update(db).await?;
        info!(embedding_id = %result.id, "向量嵌入状态更新成功");
        Ok(result)
    }

    /// 更新向量数据
    #[instrument(skip(db, vector))]
    pub async fn update_vector(
        db: &DatabaseConnection,
        id: Uuid,
        vector: Vec<f32>,
    ) -> Result<embedding::Model, AiStudioError> {
        info!(embedding_id = %id, dimension = vector.len(), "更新向量数据");

        let embedding = Self::find_by_id(db, id).await?
            .ok_or_else(|| AiStudioError::not_found("向量嵌入"))?;

        let vector_str = format!("[{}]", 
            vector.iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(",")
        );

        let mut active_model: embedding::ActiveModel = embedding.into();
        active_model.vector = Set(Some(vector_str));
        active_model.dimension = Set(vector.len() as i32);
        active_model.status = Set(embedding::EmbeddingStatus::Completed);
        active_model.processing_completed_at = Set(Some(chrono::Utc::now().into()));
        active_model.updated_at = Set(chrono::Utc::now().into());

        let result = active_model.update(db).await?;
        info!(embedding_id = %result.id, "向量数据更新成功");
        Ok(result)
    }

    /// 获取待处理的向量嵌入
    #[instrument(skip(db))]
    pub async fn find_pending_processing(
        db: &DatabaseConnection,
        limit: Option<u64>,
    ) -> Result<Vec<embedding::Model>, AiStudioError> {
        let mut query = Embedding::find()
            .filter(embedding::Column::Status.eq(embedding::EmbeddingStatus::Pending))
            .order_by_asc(embedding::Column::CreatedAt);

        if let Some(limit) = limit {
            query = query.limit(limit);
        }

        let embeddings = query.all(db).await?;
        Ok(embeddings)
    }

    /// 向量相似度搜索（使用 pgvector）
    #[instrument(skip(db, query_vector))]
    pub async fn similarity_search(
        db: &DatabaseConnection,
        knowledge_base_id: Uuid,
        query_vector: Vec<f32>,
        limit: u64,
        similarity_threshold: Option<f32>,
    ) -> Result<Vec<SimilarityResult>, AiStudioError> {
        let query_vector_str = format!("[{}]", 
            query_vector.iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(",")
        );

        // 使用 pgvector 的余弦相似度搜索
        let sql = format!(
            r#"
            SELECT 
                id, chunk_id, document_id, knowledge_base_id, 
                embedding_type, source_text, model_name, model_version,
                1 - (vector <=> '{}') AS similarity
            FROM embeddings 
            WHERE knowledge_base_id = $1 
                AND status = 'completed'
                AND vector IS NOT NULL
                {}
            ORDER BY vector <=> '{}'
            LIMIT ${}
            "#,
            query_vector_str,
            if let Some(threshold) = similarity_threshold {
                format!("AND 1 - (vector <=> '{}') >= {}", query_vector_str, threshold)
            } else {
                String::new()
            },
            query_vector_str,
            if similarity_threshold.is_some() { "3" } else { "2" }
        );

        // 这里需要使用原生 SQL 查询，因为 SeaORM 还不完全支持 pgvector 操作
        // 实际实现中需要根据具体的 pgvector 集成方式调整
        
        // 暂时返回空结果，实际实现需要执行上述 SQL
        Ok(Vec::new())
    }

    /// 删除文档块的所有嵌入
    #[instrument(skip(db))]
    pub async fn delete_by_chunk(
        db: &DatabaseConnection,
        chunk_id: Uuid,
    ) -> Result<u64, AiStudioError> {
        warn!(chunk_id = %chunk_id, "删除文档块的所有嵌入");

        let result = Embedding::delete_many()
            .filter(embedding::Column::ChunkId.eq(chunk_id))
            .exec(db)
            .await?;

        warn!(chunk_id = %chunk_id, deleted_count = result.rows_affected, "嵌入删除完成");
        Ok(result.rows_affected)
    }

    /// 删除文档的所有嵌入
    #[instrument(skip(db))]
    pub async fn delete_by_document(
        db: &DatabaseConnection,
        document_id: Uuid,
    ) -> Result<u64, AiStudioError> {
        warn!(doc_id = %document_id, "删除文档的所有嵌入");

        let result = Embedding::delete_many()
            .filter(embedding::Column::DocumentId.eq(document_id))
            .exec(db)
            .await?;

        warn!(doc_id = %document_id, deleted_count = result.rows_affected, "嵌入删除完成");
        Ok(result.rows_affected)
    }

    /// 删除知识库的所有嵌入
    #[instrument(skip(db))]
    pub async fn delete_by_knowledge_base(
        db: &DatabaseConnection,
        knowledge_base_id: Uuid,
    ) -> Result<u64, AiStudioError> {
        warn!(kb_id = %knowledge_base_id, "删除知识库的所有嵌入");

        let result = Embedding::delete_many()
            .filter(embedding::Column::KnowledgeBaseId.eq(knowledge_base_id))
            .exec(db)
            .await?;

        warn!(kb_id = %knowledge_base_id, deleted_count = result.rows_affected, "嵌入删除完成");
        Ok(result.rows_affected)
    }
}

/// 相似度搜索结果
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SimilarityResult {
    pub id: Uuid,
    pub chunk_id: Uuid,
    pub document_id: Uuid,
    pub knowledge_base_id: Uuid,
    pub embedding_type: embedding::EmbeddingType,
    pub source_text: String,
    pub model_name: String,
    pub model_version: String,
    pub similarity: f32,
}