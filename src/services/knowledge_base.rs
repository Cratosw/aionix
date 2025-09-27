// 知识库服务层
// 提供知识库管理的业务逻辑

use std::sync::Arc;
use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait, QueryOrder, PaginatorTrait, QuerySelect};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use tracing::{info, warn, error, debug};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::db::entities::{knowledge_base, prelude::*};
use crate::errors::AiStudioError;
use crate::api::models::{PaginationQuery, PaginatedResponse, PaginationInfo};

/// 知识库服务接口
#[async_trait::async_trait]
pub trait KnowledgeBaseService: Send + Sync {
    /// 创建知识库
    async fn create_knowledge_base(
        &self,
        tenant_id: Uuid,
        request: CreateKnowledgeBaseRequest,
    ) -> Result<knowledge_base::Model, AiStudioError>;
    
    /// 获取知识库列表
    async fn list_knowledge_bases(
        &self,
        tenant_id: Uuid,
        query: KnowledgeBaseQuery,
    ) -> Result<PaginatedResponse<knowledge_base::Model>, AiStudioError>;
    
    /// 获取知识库详情
    async fn get_knowledge_base(
        &self,
        tenant_id: Uuid,
        kb_id: Uuid,
    ) -> Result<Option<knowledge_base::Model>, AiStudioError>;
    
    /// 更新知识库
    async fn update_knowledge_base(
        &self,
        tenant_id: Uuid,
        kb_id: Uuid,
        request: UpdateKnowledgeBaseRequest,
    ) -> Result<Option<knowledge_base::Model>, AiStudioError>;
    
    /// 删除知识库
    async fn delete_knowledge_base(
        &self,
        tenant_id: Uuid,
        kb_id: Uuid,
    ) -> Result<bool, AiStudioError>;
    
    /// 检查知识库是否存在
    async fn exists(
        &self,
        tenant_id: Uuid,
        kb_id: Uuid,
    ) -> Result<bool, AiStudioError>;
    
    /// 检查知识库名称是否可用
    async fn is_name_available(
        &self,
        tenant_id: Uuid,
        name: &str,
        exclude_id: Option<Uuid>,
    ) -> Result<bool, AiStudioError>;
    
    /// 更新知识库统计信息
    async fn update_stats(
        &self,
        kb_id: Uuid,
        document_count: Option<i32>,
        chunk_count: Option<i32>,
        total_size_bytes: Option<i64>,
    ) -> Result<(), AiStudioError>;
    
    /// 标记知识库为已索引
    async fn mark_indexed(
        &self,
        kb_id: Uuid,
    ) -> Result<(), AiStudioError>;
    
    /// 获取需要重新索引的知识库
    async fn get_knowledge_bases_needing_reindex(
        &self,
        tenant_id: Option<Uuid>,
        limit: Option<u64>,
    ) -> Result<Vec<knowledge_base::Model>, AiStudioError>;
}

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

/// 知识库查询参数
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct KnowledgeBaseQuery {
    /// 搜索关键词
    pub q: Option<String>,
    /// 知识库类型过滤
    pub kb_type: Option<knowledge_base::KnowledgeBaseType>,
    /// 状态过滤
    pub status: Option<knowledge_base::KnowledgeBaseStatus>,
    /// 标签过滤
    pub tags: Option<Vec<String>>,
    /// 分页参数
    pub pagination: PaginationQuery,
}

/// 知识库服务实现
pub struct KnowledgeBaseServiceImpl {
    db: Arc<DatabaseConnection>,
}

impl KnowledgeBaseServiceImpl {
    /// 创建新的知识库服务实例
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

#[async_trait::async_trait]
impl KnowledgeBaseService for KnowledgeBaseServiceImpl {
    async fn create_knowledge_base(
        &self,
        tenant_id: Uuid,
        request: CreateKnowledgeBaseRequest,
    ) -> Result<knowledge_base::Model, AiStudioError> {
        debug!("创建知识库: 租户={}, 名称={}", tenant_id, request.name);
        
        // 检查名称是否可用
        if !self.is_name_available(tenant_id, &request.name, None).await? {
            return Err(AiStudioError::conflict("知识库名称已存在"));
        }
        
        // 准备配置和元数据
        let config = request.config.unwrap_or_default();
        let metadata = request.metadata.unwrap_or_default();
        let embedding_model = request.embedding_model.unwrap_or_else(|| {
            config.vectorization_settings.model_name.clone()
        });
        
        // 创建知识库
        let kb_id = Uuid::new_v4();
        let now = Utc::now().with_timezone(&chrono::FixedOffset::east_opt(8 * 3600).unwrap());
        
        let new_kb = knowledge_base::ActiveModel {
            id: sea_orm::Set(kb_id),
            tenant_id: sea_orm::Set(tenant_id),
            name: sea_orm::Set(request.name),
            description: sea_orm::Set(request.description),
            kb_type: sea_orm::Set(request.kb_type),
            status: sea_orm::Set(knowledge_base::KnowledgeBaseStatus::Active),
            config: sea_orm::Set(serde_json::to_value(&config)?.into()),
            metadata: sea_orm::Set(serde_json::to_value(&metadata)?.into()),
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
            .exec_with_returning(self.db.as_ref())
            .await
            .map_err(|e| {
                error!("创建知识库失败: {}", e);
                AiStudioError::database(format!("创建知识库失败: {}", e))
            })?;
        
        info!("知识库创建成功: id={}, 名称={}", kb.id, kb.name);
        Ok(kb)
    }
    
    async fn list_knowledge_bases(
        &self,
        tenant_id: Uuid,
        mut query: KnowledgeBaseQuery,
    ) -> Result<PaginatedResponse<knowledge_base::Model>, AiStudioError> {
        debug!("获取知识库列表: 租户={}", tenant_id);
        
        query.pagination.validate();
        
        // 构建查询
        let mut select = KnowledgeBase::find()
            .filter(knowledge_base::Column::TenantId.eq(tenant_id));
        
        // 添加搜索条件
        if let Some(q) = &query.q {
            select = select.filter(
                knowledge_base::Column::Name.contains(q)
                    .or(knowledge_base::Column::Description.contains(q))
            );
        }
        
        if let Some(kb_type) = &query.kb_type {
            select = select.filter(knowledge_base::Column::KbType.eq(kb_type.clone()));
        }
        
        if let Some(status) = &query.status {
            select = select.filter(knowledge_base::Column::Status.eq(status.clone()));
        }
        
        // TODO: 实现标签过滤
        // if let Some(tags) = &query.tags {
        //     // 需要在元数据中搜索标签
        // }
        
        // 添加排序
        let sort_column = query.pagination.sort_by.as_deref().unwrap_or("created_at");
        select = match sort_column {
            "name" => match query.pagination.sort_order {
                crate::api::models::SortOrder::Asc => select.order_by_asc(knowledge_base::Column::Name),
                crate::api::models::SortOrder::Desc => select.order_by_desc(knowledge_base::Column::Name),
            },
            "updated_at" => match query.pagination.sort_order {
                crate::api::models::SortOrder::Asc => select.order_by_asc(knowledge_base::Column::UpdatedAt),
                crate::api::models::SortOrder::Desc => select.order_by_desc(knowledge_base::Column::UpdatedAt),
            },
            "document_count" => match query.pagination.sort_order {
                crate::api::models::SortOrder::Asc => select.order_by_asc(knowledge_base::Column::DocumentCount),
                crate::api::models::SortOrder::Desc => select.order_by_desc(knowledge_base::Column::DocumentCount),
            },
            _ => match query.pagination.sort_order {
                crate::api::models::SortOrder::Asc => select.order_by_asc(knowledge_base::Column::CreatedAt),
                crate::api::models::SortOrder::Desc => select.order_by_desc(knowledge_base::Column::CreatedAt),
            },
        };
        
        // 执行分页查询
        let paginator = select.paginate(self.db.as_ref(), query.pagination.page_size as u64);
        let total = paginator.num_items().await.map_err(|e| {
            error!("查询知识库总数失败: {}", e);
            AiStudioError::database(format!("查询知识库总数失败: {}", e))
        })?;
        
        let knowledge_bases = paginator
            .fetch_page((query.pagination.page - 1) as u64)
            .await
            .map_err(|e| {
                error!("查询知识库列表失败: {}", e);
                AiStudioError::database(format!("查询知识库列表失败: {}", e))
            })?;
        
        let pagination = PaginationInfo::new(
            query.pagination.page,
            query.pagination.page_size,
            total,
        );
        
        Ok(PaginatedResponse::new(knowledge_bases, pagination))
    }
    
    async fn get_knowledge_base(
        &self,
        tenant_id: Uuid,
        kb_id: Uuid,
    ) -> Result<Option<knowledge_base::Model>, AiStudioError> {
        debug!("获取知识库详情: id={}, 租户={}", kb_id, tenant_id);
        
        let kb = KnowledgeBase::find_by_id(kb_id)
            .filter(knowledge_base::Column::TenantId.eq(tenant_id))
            .one(self.db.as_ref())
            .await
            .map_err(|e| {
                error!("查询知识库失败: {}", e);
                AiStudioError::database(format!("查询知识库失败: {}", e))
            })?;
        
        Ok(kb)
    }
    
    async fn update_knowledge_base(
        &self,
        tenant_id: Uuid,
        kb_id: Uuid,
        request: UpdateKnowledgeBaseRequest,
    ) -> Result<Option<knowledge_base::Model>, AiStudioError> {
        debug!("更新知识库: id={}, 租户={}", kb_id, tenant_id);
        
        // 查找知识库
        let kb = match self.get_knowledge_base(tenant_id, kb_id).await? {
            Some(kb) => kb,
            None => return Ok(None),
        };
        
        // 检查名称冲突
        if let Some(new_name) = &request.name {
            if new_name != &kb.name {
                if !self.is_name_available(tenant_id, new_name, Some(kb_id)).await? {
                    return Err(AiStudioError::conflict("知识库名称已存在"));
                }
            }
        }
        
        // 准备更新数据
        let mut active_model: knowledge_base::ActiveModel = kb.into();
        let now = Utc::now().with_timezone(&chrono::FixedOffset::east_opt(8 * 3600).unwrap());
        
        if let Some(name) = request.name {
            active_model.name = sea_orm::Set(name);
        }
        
        if let Some(description) = request.description {
            active_model.description = sea_orm::Set(Some(description));
        }
        
        if let Some(kb_type) = request.kb_type {
            active_model.kb_type = sea_orm::Set(kb_type);
        }
        
        if let Some(status) = request.status {
            active_model.status = sea_orm::Set(status);
        }
        
        if let Some(config) = request.config {
            active_model.config = sea_orm::Set(serde_json::to_value(&config)?.into());
            active_model.vector_dimension = sea_orm::Set(config.vectorization_settings.dimension as i32);
        }
        
        if let Some(metadata) = request.metadata {
            active_model.metadata = sea_orm::Set(serde_json::to_value(&metadata)?.into());
        }
        
        if let Some(embedding_model) = request.embedding_model {
            active_model.embedding_model = sea_orm::Set(embedding_model);
        }
        
        active_model.updated_at = sea_orm::Set(now);
        
        // 执行更新
        let updated_kb = active_model.update(self.db.as_ref()).await.map_err(|e| {
            error!("更新知识库失败: {}", e);
            AiStudioError::database(format!("更新知识库失败: {}", e))
        })?;
        
        info!("知识库更新成功: id={}, 名称={}", updated_kb.id, updated_kb.name);
        Ok(Some(updated_kb))
    }
    
    async fn delete_knowledge_base(
        &self,
        tenant_id: Uuid,
        kb_id: Uuid,
    ) -> Result<bool, AiStudioError> {
        debug!("删除知识库: id={}, 租户={}", kb_id, tenant_id);
        
        // 检查知识库是否存在
        let kb = match self.get_knowledge_base(tenant_id, kb_id).await? {
            Some(kb) => kb,
            None => return Ok(false),
        };
        
        // 检查是否包含文档
        if kb.document_count > 0 {
            return Err(AiStudioError::conflict("知识库包含文档，请先删除所有文档"));
        }
        
        // 执行删除
        let result = KnowledgeBase::delete_by_id(kb_id)
            .exec(self.db.as_ref())
            .await
            .map_err(|e| {
                error!("删除知识库失败: {}", e);
                AiStudioError::database(format!("删除知识库失败: {}", e))
            })?;
        
        let deleted = result.rows_affected > 0;
        if deleted {
            info!("知识库删除成功: id={}", kb_id);
        }
        
        Ok(deleted)
    }
    
    async fn exists(
        &self,
        tenant_id: Uuid,
        kb_id: Uuid,
    ) -> Result<bool, AiStudioError> {
        let count = KnowledgeBase::find_by_id(kb_id)
            .filter(knowledge_base::Column::TenantId.eq(tenant_id))
            .count(self.db.as_ref())
            .await
            .map_err(|e| {
                error!("检查知识库存在性失败: {}", e);
                AiStudioError::database(format!("检查知识库存在性失败: {}", e))
            })?;
        
        Ok(count > 0)
    }
    
    async fn is_name_available(
        &self,
        tenant_id: Uuid,
        name: &str,
        exclude_id: Option<Uuid>,
    ) -> Result<bool, AiStudioError> {
        let mut query = KnowledgeBase::find()
            .filter(knowledge_base::Column::TenantId.eq(tenant_id))
            .filter(knowledge_base::Column::Name.eq(name));
        
        if let Some(exclude_id) = exclude_id {
            query = query.filter(knowledge_base::Column::Id.ne(exclude_id));
        }
        
        let count = query.count(self.db.as_ref()).await.map_err(|e| {
            error!("检查知识库名称可用性失败: {}", e);
            AiStudioError::database(format!("检查知识库名称可用性失败: {}", e))
        })?;
        
        Ok(count == 0)
    }
    
    async fn update_stats(
        &self,
        kb_id: Uuid,
        document_count: Option<i32>,
        chunk_count: Option<i32>,
        total_size_bytes: Option<i64>,
    ) -> Result<(), AiStudioError> {
        debug!("更新知识库统计信息: id={}", kb_id);
        
        // 查找知识库
        let kb = KnowledgeBase::find_by_id(kb_id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| {
                error!("查询知识库失败: {}", e);
                AiStudioError::database(format!("查询知识库失败: {}", e))
            })?;
        
        let kb = match kb {
            Some(kb) => kb,
            None => return Err(AiStudioError::not_found("知识库不存在")),
        };
        
        // 准备更新数据
        let mut active_model: knowledge_base::ActiveModel = kb.into();
        let now = Utc::now().with_timezone(&chrono::FixedOffset::east_opt(8 * 3600).unwrap());
        
        if let Some(count) = document_count {
            active_model.document_count = sea_orm::Set(count);
        }
        
        if let Some(count) = chunk_count {
            active_model.chunk_count = sea_orm::Set(count);
        }
        
        if let Some(size) = total_size_bytes {
            active_model.total_size_bytes = sea_orm::Set(size);
        }
        
        active_model.updated_at = sea_orm::Set(now);
        
        // 执行更新
        active_model.update(self.db.as_ref()).await.map_err(|e| {
            error!("更新知识库统计信息失败: {}", e);
            AiStudioError::database(format!("更新知识库统计信息失败: {}", e))
        })?;
        
        debug!("知识库统计信息更新成功: id={}", kb_id);
        Ok(())
    }
    
    async fn mark_indexed(
        &self,
        kb_id: Uuid,
    ) -> Result<(), AiStudioError> {
        debug!("标记知识库为已索引: id={}", kb_id);
        
        // 查找知识库
        let kb = KnowledgeBase::find_by_id(kb_id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| {
                error!("查询知识库失败: {}", e);
                AiStudioError::database(format!("查询知识库失败: {}", e))
            })?;
        
        let kb = match kb {
            Some(kb) => kb,
            None => return Err(AiStudioError::not_found("知识库不存在")),
        };
        
        // 更新索引时间和状态
        let mut active_model: knowledge_base::ActiveModel = kb.into();
        let now = Utc::now().with_timezone(&chrono::FixedOffset::east_opt(8 * 3600).unwrap());
        
        active_model.last_indexed_at = sea_orm::Set(Some(now));
        active_model.status = sea_orm::Set(knowledge_base::KnowledgeBaseStatus::Active);
        active_model.updated_at = sea_orm::Set(now);
        
        // 执行更新
        active_model.update(self.db.as_ref()).await.map_err(|e| {
            error!("标记知识库为已索引失败: {}", e);
            AiStudioError::database(format!("标记知识库为已索引失败: {}", e))
        })?;
        
        info!("知识库标记为已索引: id={}", kb_id);
        Ok(())
    }
    
    async fn get_knowledge_bases_needing_reindex(
        &self,
        tenant_id: Option<Uuid>,
        limit: Option<u64>,
    ) -> Result<Vec<knowledge_base::Model>, AiStudioError> {
        debug!("获取需要重新索引的知识库: 租户={:?}, 限制={:?}", tenant_id, limit);
        
        let mut query = KnowledgeBase::find()
            .filter(knowledge_base::Column::Status.eq(knowledge_base::KnowledgeBaseStatus::Active));
        
        if let Some(tenant_id) = tenant_id {
            query = query.filter(knowledge_base::Column::TenantId.eq(tenant_id));
        }
        
        // 查找超过24小时未索引的知识库
        let cutoff_time = Utc::now() - chrono::Duration::hours(24);
        let cutoff_time = cutoff_time.with_timezone(&chrono::FixedOffset::east_opt(8 * 3600).unwrap());
        
        query = query.filter(
            knowledge_base::Column::LastIndexedAt.is_null()
                .or(knowledge_base::Column::LastIndexedAt.lt(cutoff_time))
        );
        
        query = query.order_by_asc(knowledge_base::Column::LastIndexedAt);
        
        if let Some(limit) = limit {
            query = query.limit(limit);
        }
        
        let knowledge_bases = query.all(self.db.as_ref()).await.map_err(|e| {
            error!("查询需要重新索引的知识库失败: {}", e);
            AiStudioError::database(format!("查询需要重新索引的知识库失败: {}", e))
        })?;
        
        debug!("找到 {} 个需要重新索引的知识库", knowledge_bases.len());
        Ok(knowledge_bases)
    }
}

/// 知识库服务工厂
pub struct KnowledgeBaseServiceFactory;

impl KnowledgeBaseServiceFactory {
    /// 创建知识库服务实例
    pub fn create(db: Arc<DatabaseConnection>) -> Arc<dyn KnowledgeBaseService> {
        Arc::new(KnowledgeBaseServiceImpl::new(db))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    // TODO: 添加单元测试
    // - 测试知识库创建
    // - 测试知识库查询
    // - 测试知识库更新
    // - 测试知识库删除
    // - 测试名称可用性检查
    // - 测试统计信息更新
}