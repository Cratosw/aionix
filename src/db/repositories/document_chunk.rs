// 文档块仓储实现

use crate::db::entities::{document_chunk, prelude::*};
use crate::errors::AiStudioError;
use sea_orm::{prelude::*, *};
use uuid::Uuid;
use tracing::{info, warn, instrument};

/// 文档块仓储
pub struct DocumentChunkRepository;

impl DocumentChunkRepository {
    /// 创建新文档块
    #[instrument(skip(db, content))]
    pub async fn create(
        db: &DatabaseConnection,
        document_id: Uuid,
        knowledge_base_id: Uuid,
        chunk_index: i32,
        content: String,
        title: Option<String>,
        content_hash: String,
    ) -> Result<document_chunk::Model, AiStudioError> {
        info!(doc_id = %document_id, chunk_index = chunk_index, "创建新文档块");

        let word_count = content.split_whitespace().count() as i32;
        let content_length = content.len() as i32;

        let chunk = document_chunk::ActiveModel {
            id: Set(Uuid::new_v4()),
            document_id: Set(document_id),
            knowledge_base_id: Set(knowledge_base_id),
            chunk_index: Set(chunk_index),
            content: Set(content),
            title: Set(title),
            summary: Set(None),
            status: Set(document_chunk::ChunkStatus::Pending),
            content_length: Set(content_length),
            word_count: Set(word_count),
            content_hash: Set(content_hash),
            metadata: Set(serde_json::to_value(document_chunk::ChunkMetadata::default())?),
            position_info: Set(serde_json::to_value(document_chunk::PositionInfo::default())?),
            processing_started_at: Set(None),
            processing_completed_at: Set(None),
            error_message: Set(None),
            created_at: Set(chrono::Utc::now().into()),
            updated_at: Set(chrono::Utc::now().into()),
        };

        let result = chunk.insert(db).await?;
        info!(chunk_id = %result.id, "文档块创建成功");
        Ok(result)
    }

    /// 根据 ID 查找文档块
    #[instrument(skip(db))]
    pub async fn find_by_id(
        db: &DatabaseConnection,
        id: Uuid,
    ) -> Result<Option<document_chunk::Model>, AiStudioError> {
        let chunk = DocumentChunk::find_by_id(id).one(db).await?;
        Ok(chunk)
    }

    /// 根据文档 ID 查找所有文档块
    #[instrument(skip(db))]
    pub async fn find_by_document(
        db: &DatabaseConnection,
        document_id: Uuid,
        limit: Option<u64>,
        offset: Option<u64>,
    ) -> Result<Vec<document_chunk::Model>, AiStudioError> {
        let mut query = DocumentChunk::find()
            .filter(document_chunk::Column::DocumentId.eq(document_id))
            .order_by_asc(document_chunk::Column::ChunkIndex);

        if let Some(limit) = limit {
            query = query.limit(limit);
        }

        if let Some(offset) = offset {
            query = query.offset(offset);
        }

        let chunks = query.all(db).await?;
        Ok(chunks)
    }

    /// 更新文档块状态
    #[instrument(skip(db))]
    pub async fn update_status(
        db: &DatabaseConnection,
        id: Uuid,
        status: document_chunk::ChunkStatus,
        error_message: Option<String>,
    ) -> Result<document_chunk::Model, AiStudioError> {
        info!(chunk_id = %id, status = ?status, "更新文档块状态");

        let chunk = Self::find_by_id(db, id).await?
            .ok_or_else(|| AiStudioError::not_found("文档块"))?;

        let mut active_model: document_chunk::ActiveModel = chunk.into();
        active_model.status = Set(status.clone());
        active_model.error_message = Set(error_message);
        active_model.updated_at = Set(chrono::Utc::now().into());

        // 设置处理时间
        match status {
            document_chunk::ChunkStatus::Processing => {
                active_model.processing_started_at = Set(Some(chrono::Utc::now().into()));
            }
            document_chunk::ChunkStatus::Completed | document_chunk::ChunkStatus::Failed => {
                active_model.processing_completed_at = Set(Some(chrono::Utc::now().into()));
            }
            _ => {}
        }

        let result = active_model.update(db).await?;
        info!(chunk_id = %result.id, "文档块状态更新成功");
        Ok(result)
    }

    /// 获取待处理的文档块
    #[instrument(skip(db))]
    pub async fn find_pending_processing(
        db: &DatabaseConnection,
        limit: Option<u64>,
    ) -> Result<Vec<document_chunk::Model>, AiStudioError> {
        let mut query = DocumentChunk::find()
            .filter(document_chunk::Column::Status.eq(document_chunk::ChunkStatus::Pending))
            .order_by_asc(document_chunk::Column::CreatedAt);

        if let Some(limit) = limit {
            query = query.limit(limit);
        }

        let chunks = query.all(db).await?;
        Ok(chunks)
    }

    /// 删除文档的所有块
    #[instrument(skip(db))]
    pub async fn delete_by_document(
        db: &DatabaseConnection,
        document_id: Uuid,
    ) -> Result<u64, AiStudioError> {
        warn!(doc_id = %document_id, "删除文档的所有块");

        let result = DocumentChunk::delete_many()
            .filter(document_chunk::Column::DocumentId.eq(document_id))
            .exec(db)
            .await?;

        warn!(doc_id = %document_id, deleted_count = result.rows_affected, "文档块删除完成");
        Ok(result.rows_affected)
    }
}