// 知识库仓储实现

use crate::db::entities::{knowledge_base, prelude::*};
use crate::errors::AiStudioError;
use sea_orm::{prelude::*, *};
use uuid::Uuid;
use tracing::{info, warn, instrument};

/// 知识库仓储
pub struct KnowledgeBaseRepository;

impl KnowledgeBaseRepository {
    /// 创建新知识库
    #[instrument(skip(db))]
    pub async fn create(
        db: &DatabaseConnection,
        tenant_id: Uuid,
        name: String,
        description: Option<String>,
        kb_type: knowledge_base::KnowledgeBaseType,
        embedding_model: String,
        vector_dimension: i32,
    ) -> Result<knowledge_base::Model, AiStudioError> {
        info!(tenant_id = %tenant_id, name = %name, "创建新知识库");

        // 检查知识库名称在租户内是否已存在
        if Self::exists_by_name_in_tenant(db, tenant_id, &name).await? {
            return Err(AiStudioError::conflict(format!("知识库名称 '{}' 在该租户内已存在", name)));
        }

        let knowledge_base = knowledge_base::ActiveModel {
            id: Set(Uuid::new_v4()),
            tenant_id: Set(tenant_id),
            name: Set(name),
            description: Set(description),
            kb_type: Set(kb_type),
            status: Set(knowledge_base::KnowledgeBaseStatus::Active),
            config: Set(serde_json::to_value(knowledge_base::KnowledgeBaseConfig::default())?),
            metadata: Set(serde_json::to_value(knowledge_base::KnowledgeBaseMetadata::default())?),
            document_count: Set(0),
            chunk_count: Set(0),
            total_size_bytes: Set(0),
            vector_dimension: Set(vector_dimension),
            embedding_model: Set(embedding_model),
            last_indexed_at: Set(None),
            created_at: Set(chrono::Utc::now().into()),
            updated_at: Set(chrono::Utc::now().into()),
        };

        let result = knowledge_base.insert(db).await?;
        info!(kb_id = %result.id, "知识库创建成功");
        Ok(result)
    }

    /// 根据 ID 查找知识库
    #[instrument(skip(db))]
    pub async fn find_by_id(
        db: &DatabaseConnection,
        id: Uuid,
    ) -> Result<Option<knowledge_base::Model>, AiStudioError> {
        let kb = KnowledgeBase::find_by_id(id).one(db).await?;
        Ok(kb)
    }

    /// 根据名称和租户 ID 查找知识库
    #[instrument(skip(db))]
    pub async fn find_by_name_in_tenant(
        db: &DatabaseConnection,
        tenant_id: Uuid,
        name: &str,
    ) -> Result<Option<knowledge_base::Model>, AiStudioError> {
        let kb = KnowledgeBase::find()
            .filter(knowledge_base::Column::TenantId.eq(tenant_id))
            .filter(knowledge_base::Column::Name.eq(name))
            .one(db)
            .await?;
        Ok(kb)
    }

    /// 检查知识库名称在租户内是否存在
    #[instrument(skip(db))]
    pub async fn exists_by_name_in_tenant(
        db: &DatabaseConnection,
        tenant_id: Uuid,
        name: &str,
    ) -> Result<bool, AiStudioError> {
        let count = KnowledgeBase::find()
            .filter(knowledge_base::Column::TenantId.eq(tenant_id))
            .filter(knowledge_base::Column::Name.eq(name))
            .count(db)
            .await?;
        Ok(count > 0)
    }

    /// 更新知识库信息
    #[instrument(skip(db, kb))]
    pub async fn update(
        db: &DatabaseConnection,
        kb: knowledge_base::Model,
    ) -> Result<knowledge_base::Model, AiStudioError> {
        info!(kb_id = %kb.id, "更新知识库信息");

        let mut active_model: knowledge_base::ActiveModel = kb.into();
        active_model.updated_at = Set(chrono::Utc::now().into());

        let result = active_model.update(db).await?;
        info!(kb_id = %result.id, "知识库信息更新成功");
        Ok(result)
    }

    /// 更新知识库状态
    #[instrument(skip(db))]
    pub async fn update_status(
        db: &DatabaseConnection,
        id: Uuid,
        status: knowledge_base::KnowledgeBaseStatus,
    ) -> Result<knowledge_base::Model, AiStudioError> {
        info!(kb_id = %id, status = ?status, "更新知识库状态");

        let kb = Self::find_by_id(db, id).await?
            .ok_or_else(|| AiStudioError::not_found("知识库"))?;

        let mut active_model: knowledge_base::ActiveModel = kb.into();
        active_model.status = Set(status);
        active_model.updated_at = Set(chrono::Utc::now().into());

        let result = active_model.update(db).await?;
        info!(kb_id = %result.id, "知识库状态更新成功");
        Ok(result)
    }

    /// 更新知识库配置
    #[instrument(skip(db, config))]
    pub async fn update_config(
        db: &DatabaseConnection,
        id: Uuid,
        config: knowledge_base::KnowledgeBaseConfig,
    ) -> Result<knowledge_base::Model, AiStudioError> {
        info!(kb_id = %id, "更新知识库配置");

        let kb = Self::find_by_id(db, id).await?
            .ok_or_else(|| AiStudioError::not_found("知识库"))?;

        let mut active_model: knowledge_base::ActiveModel = kb.into();
        active_model.config = Set(serde_json::to_value(config)?);
        active_model.updated_at = Set(chrono::Utc::now().into());

        let result = active_model.update(db).await?;
        info!(kb_id = %result.id, "知识库配置更新成功");
        Ok(result)
    }

    /// 更新统计信息
    #[instrument(skip(db))]
    pub async fn update_stats(
        db: &DatabaseConnection,
        id: Uuid,
        document_count: i32,
        chunk_count: i32,
        total_size_bytes: i64,
    ) -> Result<(), AiStudioError> {
        KnowledgeBase::update_many()
            .col_expr(knowledge_base::Column::DocumentCount, Expr::value(document_count))
            .col_expr(knowledge_base::Column::ChunkCount, Expr::value(chunk_count))
            .col_expr(knowledge_base::Column::TotalSizeBytes, Expr::value(total_size_bytes))
            .col_expr(knowledge_base::Column::UpdatedAt, Expr::value(chrono::Utc::now()))
            .filter(knowledge_base::Column::Id.eq(id))
            .exec(db)
            .await?;
        Ok(())
    }

    /// 更新最后索引时间
    #[instrument(skip(db))]
    pub async fn update_last_indexed(
        db: &DatabaseConnection,
        id: Uuid,
    ) -> Result<(), AiStudioError> {
        KnowledgeBase::update_many()
            .col_expr(knowledge_base::Column::LastIndexedAt, Expr::value(chrono::Utc::now()))
            .col_expr(knowledge_base::Column::UpdatedAt, Expr::value(chrono::Utc::now()))
            .filter(knowledge_base::Column::Id.eq(id))
            .exec(db)
            .await?;
        Ok(())
    }

    /// 获取租户内的知识库列表
    #[instrument(skip(db))]
    pub async fn find_by_tenant(
        db: &DatabaseConnection,
        tenant_id: Uuid,
        limit: Option<u64>,
        offset: Option<u64>,
    ) -> Result<Vec<knowledge_base::Model>, AiStudioError> {
        let mut query = KnowledgeBase::find()
            .filter(knowledge_base::Column::TenantId.eq(tenant_id))
            .order_by_desc(knowledge_base::Column::UpdatedAt);

        if let Some(limit) = limit {
            query = query.limit(limit);
        }

        if let Some(offset) = offset {
            query = query.offset(offset);
        }

        let kbs = query.all(db).await?;
        Ok(kbs)
    }

    /// 获取活跃知识库列表
    #[instrument(skip(db))]
    pub async fn find_active_by_tenant(
        db: &DatabaseConnection,
        tenant_id: Uuid,
        limit: Option<u64>,
        offset: Option<u64>,
    ) -> Result<Vec<knowledge_base::Model>, AiStudioError> {
        let mut query = KnowledgeBase::find()
            .filter(knowledge_base::Column::TenantId.eq(tenant_id))
            .filter(knowledge_base::Column::Status.eq(knowledge_base::KnowledgeBaseStatus::Active))
            .order_by_desc(knowledge_base::Column::UpdatedAt);

        if let Some(limit) = limit {
            query = query.limit(limit);
        }

        if let Some(offset) = offset {
            query = query.offset(offset);
        }

        let kbs = query.all(db).await?;
        Ok(kbs)
    }

    /// 获取租户内知识库总数
    #[instrument(skip(db))]
    pub async fn count_by_tenant(
        db: &DatabaseConnection,
        tenant_id: Uuid,
    ) -> Result<u64, AiStudioError> {
        let count = KnowledgeBase::find()
            .filter(knowledge_base::Column::TenantId.eq(tenant_id))
            .count(db)
            .await?;
        Ok(count)
    }

    /// 搜索知识库
    #[instrument(skip(db))]
    pub async fn search_in_tenant(
        db: &DatabaseConnection,
        tenant_id: Uuid,
        query: &str,
        limit: Option<u64>,
        offset: Option<u64>,
    ) -> Result<Vec<knowledge_base::Model>, AiStudioError> {
        let search_pattern = format!("%{}%", query);
        
        let mut search_query = KnowledgeBase::find()
            .filter(knowledge_base::Column::TenantId.eq(tenant_id))
            .filter(
                Condition::any()
                    .add(knowledge_base::Column::Name.like(&search_pattern))
                    .add(knowledge_base::Column::Description.like(&search_pattern))
            )
            .order_by_desc(knowledge_base::Column::UpdatedAt);

        if let Some(limit) = limit {
            search_query = search_query.limit(limit);
        }

        if let Some(offset) = offset {
            search_query = search_query.offset(offset);
        }

        let kbs = search_query.all(db).await?;
        Ok(kbs)
    }

    /// 按类型查找知识库
    #[instrument(skip(db))]
    pub async fn find_by_type_in_tenant(
        db: &DatabaseConnection,
        tenant_id: Uuid,
        kb_type: knowledge_base::KnowledgeBaseType,
        limit: Option<u64>,
        offset: Option<u64>,
    ) -> Result<Vec<knowledge_base::Model>, AiStudioError> {
        let mut query = KnowledgeBase::find()
            .filter(knowledge_base::Column::TenantId.eq(tenant_id))
            .filter(knowledge_base::Column::KbType.eq(kb_type))
            .order_by_desc(knowledge_base::Column::UpdatedAt);

        if let Some(limit) = limit {
            query = query.limit(limit);
        }

        if let Some(offset) = offset {
            query = query.offset(offset);
        }

        let kbs = query.all(db).await?;
        Ok(kbs)
    }

    /// 获取需要重新索引的知识库
    #[instrument(skip(db))]
    pub async fn find_needs_reindexing(
        db: &DatabaseConnection,
        hours_threshold: i64,
        limit: Option<u64>,
    ) -> Result<Vec<knowledge_base::Model>, AiStudioError> {
        let threshold_time = chrono::Utc::now() - chrono::Duration::hours(hours_threshold);
        
        let mut query = KnowledgeBase::find()
            .filter(knowledge_base::Column::Status.eq(knowledge_base::KnowledgeBaseStatus::Active))
            .filter(
                Condition::any()
                    .add(knowledge_base::Column::LastIndexedAt.is_null())
                    .add(knowledge_base::Column::LastIndexedAt.lt(threshold_time))
            )
            .order_by_asc(knowledge_base::Column::LastIndexedAt);

        if let Some(limit) = limit {
            query = query.limit(limit);
        }

        let kbs = query.all(db).await?;
        Ok(kbs)
    }

    /// 软删除知识库
    #[instrument(skip(db))]
    pub async fn soft_delete(
        db: &DatabaseConnection,
        id: Uuid,
    ) -> Result<knowledge_base::Model, AiStudioError> {
        warn!(kb_id = %id, "软删除知识库");

        let result = Self::update_status(db, id, knowledge_base::KnowledgeBaseStatus::Inactive).await?;
        warn!(kb_id = %result.id, "知识库已软删除");
        Ok(result)
    }

    /// 硬删除知识库（谨慎使用）
    #[instrument(skip(db))]
    pub async fn hard_delete(
        db: &DatabaseConnection,
        id: Uuid,
    ) -> Result<(), AiStudioError> {
        warn!(kb_id = %id, "硬删除知识库");

        let result = KnowledgeBase::delete_by_id(id).exec(db).await?;
        if result.rows_affected == 0 {
            return Err(AiStudioError::not_found("知识库"));
        }

        warn!(kb_id = %id, "知识库已硬删除");
        Ok(())
    }

    /// 获取知识库统计信息
    #[instrument(skip(db))]
    pub async fn get_stats_by_tenant(
        db: &DatabaseConnection,
        tenant_id: Uuid,
    ) -> Result<KnowledgeBaseStats, AiStudioError> {
        let kbs = Self::find_by_tenant(db, tenant_id, None, None).await?;
        
        let total_count = kbs.len() as u32;
        let active_count = kbs.iter().filter(|kb| kb.is_active()).count() as u32;
        let total_documents = kbs.iter().map(|kb| kb.document_count).sum::<i32>() as u32;
        let total_chunks = kbs.iter().map(|kb| kb.chunk_count).sum::<i32>() as u32;
        let total_size = kbs.iter().map(|kb| kb.total_size_bytes).sum::<i64>() as u64;

        Ok(KnowledgeBaseStats {
            total_count,
            active_count,
            total_documents,
            total_chunks,
            total_size,
        })
    }
}

/// 知识库统计信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KnowledgeBaseStats {
    /// 知识库总数
    pub total_count: u32,
    /// 活跃知识库数
    pub active_count: u32,
    /// 文档总数
    pub total_documents: u32,
    /// 文档块总数
    pub total_chunks: u32,
    /// 总存储大小
    pub total_size: u64,
}