// 文档仓储实现

use crate::db::entities::{document, prelude::*};
use crate::errors::AiStudioError;
use sea_orm::{prelude::*, *};
use uuid::Uuid;
use tracing::{info, warn, instrument};

/// 文档仓储
pub struct DocumentRepository;

impl DocumentRepository {
    /// 创建新文档
    #[instrument(skip(db, content))]
    pub async fn create(
        db: &DatabaseConnection,
        knowledge_base_id: Uuid,
        title: String,
        content: String,
        doc_type: document::DocumentType,
        file_path: Option<String>,
        file_name: Option<String>,
        file_size: i64,
        mime_type: Option<String>,
        content_hash: Option<String>,
    ) -> Result<document::Model, AiStudioError> {
        info!(kb_id = %knowledge_base_id, title = %title, "创建新文档");

        let document = document::ActiveModel {
            id: Set(Uuid::new_v4()),
            knowledge_base_id: Set(knowledge_base_id),
            title: Set(title),
            content: Set(content.clone()),
            raw_content: Set(Some(content)),
            summary: Set(None),
            doc_type: Set(doc_type),
            status: Set(document::DocumentStatus::Pending),
            file_path: Set(file_path),
            file_name: Set(file_name),
            file_size: Set(file_size),
            mime_type: Set(mime_type),
            content_hash: Set(content_hash),
            metadata: Set(serde_json::to_value(document::DocumentMetadata::default())?),
            processing_config: Set(serde_json::to_value(document::DocumentProcessingConfig::default())?),
            chunk_count: Set(0),
            processing_started_at: Set(None),
            processing_completed_at: Set(None),
            error_message: Set(None),
            version: Set(1),
            created_at: Set(chrono::Utc::now().into()),
            updated_at: Set(chrono::Utc::now().into()),
        };

        let result = document.insert(db).await?;
        info!(doc_id = %result.id, "文档创建成功");
        Ok(result)
    }

    /// 根据 ID 查找文档
    #[instrument(skip(db))]
    pub async fn find_by_id(
        db: &DatabaseConnection,
        id: Uuid,
    ) -> Result<Option<document::Model>, AiStudioError> {
        let doc = Document::find_by_id(id).one(db).await?;
        Ok(doc)
    }

    /// 根据内容哈希查找文档
    #[instrument(skip(db))]
    pub async fn find_by_content_hash(
        db: &DatabaseConnection,
        knowledge_base_id: Uuid,
        content_hash: &str,
    ) -> Result<Option<document::Model>, AiStudioError> {
        let doc = Document::find()
            .filter(document::Column::KnowledgeBaseId.eq(knowledge_base_id))
            .filter(document::Column::ContentHash.eq(content_hash))
            .one(db)
            .await?;
        Ok(doc)
    }

    /// 更新文档信息
    #[instrument(skip(db, doc))]
    pub async fn update(
        db: &DatabaseConnection,
        doc: document::Model,
    ) -> Result<document::Model, AiStudioError> {
        info!(doc_id = %doc.id, "更新文档信息");

        let mut active_model: document::ActiveModel = doc.into();
        active_model.updated_at = Set(chrono::Utc::now().into());

        let result = active_model.update(db).await?;
        info!(doc_id = %result.id, "文档信息更新成功");
        Ok(result)
    }

    /// 更新文档状态
    #[instrument(skip(db))]
    pub async fn update_status(
        db: &DatabaseConnection,
        id: Uuid,
        status: document::DocumentStatus,
        error_message: Option<String>,
    ) -> Result<document::Model, AiStudioError> {
        info!(doc_id = %id, status = ?status, "更新文档状态");

        let doc = Self::find_by_id(db, id).await?
            .ok_or_else(|| AiStudioError::not_found("文档"))?;

        let mut active_model: document::ActiveModel = doc.into();
        active_model.status = Set(status.clone());
        active_model.error_message = Set(error_message);
        active_model.updated_at = Set(chrono::Utc::now().into());

        // 设置处理时间
        match status {
            document::DocumentStatus::Processing => {
                active_model.processing_started_at = Set(Some(chrono::Utc::now().into()));
            }
            document::DocumentStatus::Completed | document::DocumentStatus::Failed => {
                active_model.processing_completed_at = Set(Some(chrono::Utc::now().into()));
            }
            _ => {}
        }

        let result = active_model.update(db).await?;
        info!(doc_id = %result.id, "文档状态更新成功");
        Ok(result)
    }

    /// 更新文档内容
    #[instrument(skip(db, content))]
    pub async fn update_content(
        db: &DatabaseConnection,
        id: Uuid,
        content: String,
        content_hash: Option<String>,
    ) -> Result<document::Model, AiStudioError> {
        info!(doc_id = %id, "更新文档内容");

        let doc = Self::find_by_id(db, id).await?
            .ok_or_else(|| AiStudioError::not_found("文档"))?;

        let mut active_model: document::ActiveModel = doc.into();
        active_model.content = Set(content);
        active_model.content_hash = Set(content_hash);
        active_model.version = Set(active_model.version.unwrap() + 1);
        active_model.updated_at = Set(chrono::Utc::now().into());

        let result = active_model.update(db).await?;
        info!(doc_id = %result.id, "文档内容更新成功");
        Ok(result)
    }

    /// 更新文档块数量
    #[instrument(skip(db))]
    pub async fn update_chunk_count(
        db: &DatabaseConnection,
        id: Uuid,
        chunk_count: i32,
    ) -> Result<(), AiStudioError> {
        Document::update_many()
            .col_expr(document::Column::ChunkCount, Expr::value(chunk_count))
            .col_expr(document::Column::UpdatedAt, Expr::value(chrono::Utc::now()))
            .filter(document::Column::Id.eq(id))
            .exec(db)
            .await?;
        Ok(())
    }

    /// 获取知识库内的文档列表
    #[instrument(skip(db))]
    pub async fn find_by_knowledge_base(
        db: &DatabaseConnection,
        knowledge_base_id: Uuid,
        limit: Option<u64>,
        offset: Option<u64>,
    ) -> Result<Vec<document::Model>, AiStudioError> {
        let mut query = Document::find()
            .filter(document::Column::KnowledgeBaseId.eq(knowledge_base_id))
            .order_by_desc(document::Column::UpdatedAt);

        if let Some(limit) = limit {
            query = query.limit(limit);
        }

        if let Some(offset) = offset {
            query = query.offset(offset);
        }

        let docs = query.all(db).await?;
        Ok(docs)
    }

    /// 按状态查找文档
    #[instrument(skip(db))]
    pub async fn find_by_status(
        db: &DatabaseConnection,
        knowledge_base_id: Option<Uuid>,
        status: document::DocumentStatus,
        limit: Option<u64>,
        offset: Option<u64>,
    ) -> Result<Vec<document::Model>, AiStudioError> {
        let mut query = Document::find()
            .filter(document::Column::Status.eq(status));

        if let Some(kb_id) = knowledge_base_id {
            query = query.filter(document::Column::KnowledgeBaseId.eq(kb_id));
        }

        query = query.order_by_asc(document::Column::CreatedAt);

        if let Some(limit) = limit {
            query = query.limit(limit);
        }

        if let Some(offset) = offset {
            query = query.offset(offset);
        }

        let docs = query.all(db).await?;
        Ok(docs)
    }

    /// 按类型查找文档
    #[instrument(skip(db))]
    pub async fn find_by_type(
        db: &DatabaseConnection,
        knowledge_base_id: Uuid,
        doc_type: document::DocumentType,
        limit: Option<u64>,
        offset: Option<u64>,
    ) -> Result<Vec<document::Model>, AiStudioError> {
        let mut query = Document::find()
            .filter(document::Column::KnowledgeBaseId.eq(knowledge_base_id))
            .filter(document::Column::DocType.eq(doc_type))
            .order_by_desc(document::Column::UpdatedAt);

        if let Some(limit) = limit {
            query = query.limit(limit);
        }

        if let Some(offset) = offset {
            query = query.offset(offset);
        }

        let docs = query.all(db).await?;
        Ok(docs)
    }

    /// 搜索文档
    #[instrument(skip(db))]
    pub async fn search_in_knowledge_base(
        db: &DatabaseConnection,
        knowledge_base_id: Uuid,
        query: &str,
        limit: Option<u64>,
        offset: Option<u64>,
    ) -> Result<Vec<document::Model>, AiStudioError> {
        let search_pattern = format!("%{}%", query);
        
        let mut search_query = Document::find()
            .filter(document::Column::KnowledgeBaseId.eq(knowledge_base_id))
            .filter(
                Condition::any()
                    .add(document::Column::Title.like(&search_pattern))
                    .add(document::Column::Content.like(&search_pattern))
                    .add(document::Column::Summary.like(&search_pattern))
            )
            .order_by_desc(document::Column::UpdatedAt);

        if let Some(limit) = limit {
            search_query = search_query.limit(limit);
        }

        if let Some(offset) = offset {
            search_query = search_query.offset(offset);
        }

        let docs = search_query.all(db).await?;
        Ok(docs)
    }

    /// 获取文档总数
    #[instrument(skip(db))]
    pub async fn count_by_knowledge_base(
        db: &DatabaseConnection,
        knowledge_base_id: Uuid,
    ) -> Result<u64, AiStudioError> {
        let count = Document::find()
            .filter(document::Column::KnowledgeBaseId.eq(knowledge_base_id))
            .count(db)
            .await?;
        Ok(count)
    }

    /// 按状态统计文档数量
    #[instrument(skip(db))]
    pub async fn count_by_status(
        db: &DatabaseConnection,
        knowledge_base_id: Uuid,
        status: document::DocumentStatus,
    ) -> Result<u64, AiStudioError> {
        let count = Document::find()
            .filter(document::Column::KnowledgeBaseId.eq(knowledge_base_id))
            .filter(document::Column::Status.eq(status))
            .count(db)
            .await?;
        Ok(count)
    }

    /// 获取待处理的文档
    #[instrument(skip(db))]
    pub async fn find_pending_processing(
        db: &DatabaseConnection,
        limit: Option<u64>,
    ) -> Result<Vec<document::Model>, AiStudioError> {
        let mut query = Document::find()
            .filter(document::Column::Status.eq(document::DocumentStatus::Pending))
            .order_by_asc(document::Column::CreatedAt);

        if let Some(limit) = limit {
            query = query.limit(limit);
        }

        let docs = query.all(db).await?;
        Ok(docs)
    }

    /// 获取处理超时的文档
    #[instrument(skip(db))]
    pub async fn find_processing_timeout(
        db: &DatabaseConnection,
        timeout_minutes: i64,
        limit: Option<u64>,
    ) -> Result<Vec<document::Model>, AiStudioError> {
        let timeout_time = chrono::Utc::now() - chrono::Duration::minutes(timeout_minutes);
        
        let mut query = Document::find()
            .filter(document::Column::Status.eq(document::DocumentStatus::Processing))
            .filter(document::Column::ProcessingStartedAt.lt(timeout_time))
            .order_by_asc(document::Column::ProcessingStartedAt);

        if let Some(limit) = limit {
            query = query.limit(limit);
        }

        let docs = query.all(db).await?;
        Ok(docs)
    }

    /// 批量更新文档状态
    #[instrument(skip(db))]
    pub async fn batch_update_status(
        db: &DatabaseConnection,
        document_ids: Vec<Uuid>,
        status: document::DocumentStatus,
    ) -> Result<u64, AiStudioError> {
        let result = Document::update_many()
            .col_expr(document::Column::Status, Expr::value(status))
            .col_expr(document::Column::UpdatedAt, Expr::value(chrono::Utc::now()))
            .filter(document::Column::Id.is_in(document_ids))
            .exec(db)
            .await?;

        Ok(result.rows_affected)
    }

    /// 删除文档
    #[instrument(skip(db))]
    pub async fn delete(
        db: &DatabaseConnection,
        id: Uuid,
    ) -> Result<(), AiStudioError> {
        warn!(doc_id = %id, "删除文档");

        let result = Document::delete_by_id(id).exec(db).await?;
        if result.rows_affected == 0 {
            return Err(AiStudioError::not_found("文档"));
        }

        warn!(doc_id = %id, "文档已删除");
        Ok(())
    }

    /// 批量删除文档
    #[instrument(skip(db))]
    pub async fn batch_delete(
        db: &DatabaseConnection,
        document_ids: Vec<Uuid>,
    ) -> Result<u64, AiStudioError> {
        warn!(count = document_ids.len(), "批量删除文档");

        let result = Document::delete_many()
            .filter(document::Column::Id.is_in(document_ids))
            .exec(db)
            .await?;

        warn!(deleted_count = result.rows_affected, "文档批量删除完成");
        Ok(result.rows_affected)
    }

    /// 获取文档统计信息
    #[instrument(skip(db))]
    pub async fn get_stats_by_knowledge_base(
        db: &DatabaseConnection,
        knowledge_base_id: Uuid,
    ) -> Result<DocumentStats, AiStudioError> {
        let docs = Self::find_by_knowledge_base(db, knowledge_base_id, None, None).await?;
        
        let total_count = docs.len() as u32;
        let completed_count = docs.iter().filter(|doc| doc.is_completed()).count() as u32;
        let processing_count = docs.iter().filter(|doc| doc.is_processing()).count() as u32;
        let failed_count = docs.iter().filter(|doc| doc.has_failed()).count() as u32;
        let total_size = docs.iter().map(|doc| doc.file_size).sum::<i64>() as u64;
        let total_chunks = docs.iter().map(|doc| doc.chunk_count).sum::<i32>() as u32;

        Ok(DocumentStats {
            total_count,
            completed_count,
            processing_count,
            failed_count,
            total_size,
            total_chunks,
        })
    }
}

/// 文档统计信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DocumentStats {
    /// 文档总数
    pub total_count: u32,
    /// 已完成文档数
    pub completed_count: u32,
    /// 处理中文档数
    pub processing_count: u32,
    /// 失败文档数
    pub failed_count: u32,
    /// 总文件大小
    pub total_size: u64,
    /// 总文档块数
    pub total_chunks: u32,
}