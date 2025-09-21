// 工作流仓储实现

use crate::db::entities::{workflow, prelude::*};
use crate::errors::AiStudioError;
use sea_orm::{prelude::*, *};
use uuid::Uuid;
use tracing::{info, warn, instrument};

/// 工作流仓储
pub struct WorkflowRepository;

impl WorkflowRepository {
    /// 创建新工作流
    #[instrument(skip(db, definition))]
    pub async fn create(
        db: &DatabaseConnection,
        tenant_id: Uuid,
        name: String,
        description: Option<String>,
        workflow_type: workflow::WorkflowType,
        definition: workflow::WorkflowDefinition,
        created_by: Uuid,
    ) -> Result<workflow::Model, AiStudioError> {
        info!(tenant_id = %tenant_id, name = %name, "创建新工作流");

        // 检查工作流名称在租户内是否已存在
        if Self::exists_by_name_in_tenant(db, tenant_id, &name).await? {
            return Err(AiStudioError::conflict(format!("工作流名称 '{}' 在该租户内已存在", name)));
        }

        let workflow = workflow::ActiveModel {
            id: Set(Uuid::new_v4()),
            tenant_id: Set(tenant_id),
            name: Set(name),
            description: Set(description),
            workflow_type: Set(workflow_type),
            status: Set(workflow::WorkflowStatus::Draft),
            version: Set("1.0.0".to_string()),
            definition: Set(serde_json::to_value(definition)?),
            config: Set(serde_json::to_value(workflow::WorkflowConfig::default())?),
            input_schema: Set(serde_json::Value::Object(serde_json::Map::new())),
            output_schema: Set(serde_json::Value::Object(serde_json::Map::new())),
            metadata: Set(serde_json::to_value(workflow::WorkflowMetadata::default())?),
            execution_stats: Set(serde_json::to_value(workflow::WorkflowExecutionStats::default())?),
            last_executed_at: Set(None),
            created_by: Set(created_by),
            created_at: Set(chrono::Utc::now().into()),
            updated_at: Set(chrono::Utc::now().into()),
        };

        let result = workflow.insert(db).await?;
        info!(workflow_id = %result.id, "工作流创建成功");
        Ok(result)
    }

    /// 根据 ID 查找工作流
    #[instrument(skip(db))]
    pub async fn find_by_id(
        db: &DatabaseConnection,
        id: Uuid,
    ) -> Result<Option<workflow::Model>, AiStudioError> {
        let workflow = Workflow::find_by_id(id).one(db).await?;
        Ok(workflow)
    }

    /// 根据名称和租户 ID 查找工作流
    #[instrument(skip(db))]
    pub async fn find_by_name_in_tenant(
        db: &DatabaseConnection,
        tenant_id: Uuid,
        name: &str,
    ) -> Result<Option<workflow::Model>, AiStudioError> {
        let workflow = Workflow::find()
            .filter(workflow::Column::TenantId.eq(tenant_id))
            .filter(workflow::Column::Name.eq(name))
            .one(db)
            .await?;
        Ok(workflow)
    }

    /// 检查工作流名称在租户内是否存在
    #[instrument(skip(db))]
    pub async fn exists_by_name_in_tenant(
        db: &DatabaseConnection,
        tenant_id: Uuid,
        name: &str,
    ) -> Result<bool, AiStudioError> {
        let count = Workflow::find()
            .filter(workflow::Column::TenantId.eq(tenant_id))
            .filter(workflow::Column::Name.eq(name))
            .count(db)
            .await?;
        Ok(count > 0)
    }

    /// 更新工作流信息
    #[instrument(skip(db, workflow))]
    pub async fn update(
        db: &DatabaseConnection,
        workflow: workflow::Model,
    ) -> Result<workflow::Model, AiStudioError> {
        info!(workflow_id = %workflow.id, "更新工作流信息");

        let mut active_model: workflow::ActiveModel = workflow.into();
        active_model.updated_at = Set(chrono::Utc::now().into());

        let result = active_model.update(db).await?;
        info!(workflow_id = %result.id, "工作流信息更新成功");
        Ok(result)
    }

    /// 更新工作流状态
    #[instrument(skip(db))]
    pub async fn update_status(
        db: &DatabaseConnection,
        id: Uuid,
        status: workflow::WorkflowStatus,
    ) -> Result<workflow::Model, AiStudioError> {
        info!(workflow_id = %id, status = ?status, "更新工作流状态");

        let workflow = Self::find_by_id(db, id).await?
            .ok_or_else(|| AiStudioError::not_found("工作流"))?;

        let mut active_model: workflow::ActiveModel = workflow.into();
        active_model.status = Set(status);
        active_model.updated_at = Set(chrono::Utc::now().into());

        let result = active_model.update(db).await?;
        info!(workflow_id = %result.id, "工作流状态更新成功");
        Ok(result)
    }

    /// 更新工作流定义
    #[instrument(skip(db, definition))]
    pub async fn update_definition(
        db: &DatabaseConnection,
        id: Uuid,
        definition: workflow::WorkflowDefinition,
    ) -> Result<workflow::Model, AiStudioError> {
        info!(workflow_id = %id, "更新工作流定义");

        let workflow = Self::find_by_id(db, id).await?
            .ok_or_else(|| AiStudioError::not_found("工作流"))?;

        let mut active_model: workflow::ActiveModel = workflow.into();
        active_model.definition = Set(serde_json::to_value(definition)?);
        active_model.updated_at = Set(chrono::Utc::now().into());

        let result = active_model.update(db).await?;
        info!(workflow_id = %result.id, "工作流定义更新成功");
        Ok(result)
    }

    /// 更新工作流配置
    #[instrument(skip(db, config))]
    pub async fn update_config(
        db: &DatabaseConnection,
        id: Uuid,
        config: workflow::WorkflowConfig,
    ) -> Result<workflow::Model, AiStudioError> {
        info!(workflow_id = %id, "更新工作流配置");

        let workflow = Self::find_by_id(db, id).await?
            .ok_or_else(|| AiStudioError::not_found("工作流"))?;

        let mut active_model: workflow::ActiveModel = workflow.into();
        active_model.config = Set(serde_json::to_value(config)?);
        active_model.updated_at = Set(chrono::Utc::now().into());

        let result = active_model.update(db).await?;
        info!(workflow_id = %result.id, "工作流配置更新成功");
        Ok(result)
    }

    /// 更新最后执行时间
    #[instrument(skip(db))]
    pub async fn update_last_executed(
        db: &DatabaseConnection,
        id: Uuid,
    ) -> Result<(), AiStudioError> {
        Workflow::update_many()
            .col_expr(workflow::Column::LastExecutedAt, Expr::value(chrono::Utc::now()))
            .col_expr(workflow::Column::UpdatedAt, Expr::value(chrono::Utc::now()))
            .filter(workflow::Column::Id.eq(id))
            .exec(db)
            .await?;
        Ok(())
    }

    /// 更新执行统计
    #[instrument(skip(db, stats))]
    pub async fn update_execution_stats(
        db: &DatabaseConnection,
        id: Uuid,
        stats: workflow::WorkflowExecutionStats,
    ) -> Result<(), AiStudioError> {
        Workflow::update_many()
            .col_expr(workflow::Column::ExecutionStats, Expr::value(serde_json::to_value(stats)?))
            .col_expr(workflow::Column::UpdatedAt, Expr::value(chrono::Utc::now()))
            .filter(workflow::Column::Id.eq(id))
            .exec(db)
            .await?;
        Ok(())
    }

    /// 获取租户内的工作流列表
    #[instrument(skip(db))]
    pub async fn find_by_tenant(
        db: &DatabaseConnection,
        tenant_id: Uuid,
        limit: Option<u64>,
        offset: Option<u64>,
    ) -> Result<Vec<workflow::Model>, AiStudioError> {
        let mut query = Workflow::find()
            .filter(workflow::Column::TenantId.eq(tenant_id))
            .order_by_desc(workflow::Column::UpdatedAt);

        if let Some(limit) = limit {
            query = query.limit(limit);
        }

        if let Some(offset) = offset {
            query = query.offset(offset);
        }

        let workflows = query.all(db).await?;
        Ok(workflows)
    }

    /// 获取活跃工作流列表
    #[instrument(skip(db))]
    pub async fn find_active_by_tenant(
        db: &DatabaseConnection,
        tenant_id: Uuid,
        limit: Option<u64>,
        offset: Option<u64>,
    ) -> Result<Vec<workflow::Model>, AiStudioError> {
        let mut query = Workflow::find()
            .filter(workflow::Column::TenantId.eq(tenant_id))
            .filter(workflow::Column::Status.eq(workflow::WorkflowStatus::Active))
            .order_by_desc(workflow::Column::LastExecutedAt);

        if let Some(limit) = limit {
            query = query.limit(limit);
        }

        if let Some(offset) = offset {
            query = query.offset(offset);
        }

        let workflows = query.all(db).await?;
        Ok(workflows)
    }

    /// 按类型查找工作流
    #[instrument(skip(db))]
    pub async fn find_by_type_in_tenant(
        db: &DatabaseConnection,
        tenant_id: Uuid,
        workflow_type: workflow::WorkflowType,
        limit: Option<u64>,
        offset: Option<u64>,
    ) -> Result<Vec<workflow::Model>, AiStudioError> {
        let mut query = Workflow::find()
            .filter(workflow::Column::TenantId.eq(tenant_id))
            .filter(workflow::Column::WorkflowType.eq(workflow_type))
            .order_by_desc(workflow::Column::UpdatedAt);

        if let Some(limit) = limit {
            query = query.limit(limit);
        }

        if let Some(offset) = offset {
            query = query.offset(offset);
        }

        let workflows = query.all(db).await?;
        Ok(workflows)
    }

    /// 搜索工作流
    #[instrument(skip(db))]
    pub async fn search_in_tenant(
        db: &DatabaseConnection,
        tenant_id: Uuid,
        query: &str,
        limit: Option<u64>,
        offset: Option<u64>,
    ) -> Result<Vec<workflow::Model>, AiStudioError> {
        let search_pattern = format!("%{}%", query);
        
        let mut search_query = Workflow::find()
            .filter(workflow::Column::TenantId.eq(tenant_id))
            .filter(
                Condition::any()
                    .add(workflow::Column::Name.like(&search_pattern))
                    .add(workflow::Column::Description.like(&search_pattern))
            )
            .order_by_desc(workflow::Column::UpdatedAt);

        if let Some(limit) = limit {
            search_query = search_query.limit(limit);
        }

        if let Some(offset) = offset {
            search_query = search_query.offset(offset);
        }

        let workflows = search_query.all(db).await?;
        Ok(workflows)
    }

    /// 验证工作流定义
    #[instrument(skip(db))]
    pub async fn validate_workflow(
        db: &DatabaseConnection,
        id: Uuid,
    ) -> Result<bool, AiStudioError> {
        let workflow = Self::find_by_id(db, id).await?
            .ok_or_else(|| AiStudioError::not_found("工作流"))?;

        match workflow.validate_definition() {
            Ok(valid) => Ok(valid),
            Err(error) => Err(AiStudioError::validation("workflow_definition", error)),
        }
    }

    /// 获取租户内工作流总数
    #[instrument(skip(db))]
    pub async fn count_by_tenant(
        db: &DatabaseConnection,
        tenant_id: Uuid,
    ) -> Result<u64, AiStudioError> {
        let count = Workflow::find()
            .filter(workflow::Column::TenantId.eq(tenant_id))
            .count(db)
            .await?;
        Ok(count)
    }

    /// 按状态统计工作流数量
    #[instrument(skip(db))]
    pub async fn count_by_status(
        db: &DatabaseConnection,
        tenant_id: Uuid,
        status: workflow::WorkflowStatus,
    ) -> Result<u64, AiStudioError> {
        let count = Workflow::find()
            .filter(workflow::Column::TenantId.eq(tenant_id))
            .filter(workflow::Column::Status.eq(status))
            .count(db)
            .await?;
        Ok(count)
    }

    /// 软删除工作流
    #[instrument(skip(db))]
    pub async fn soft_delete(
        db: &DatabaseConnection,
        id: Uuid,
    ) -> Result<workflow::Model, AiStudioError> {
        warn!(workflow_id = %id, "软删除工作流");

        let result = Self::update_status(db, id, workflow::WorkflowStatus::Archived).await?;
        warn!(workflow_id = %result.id, "工作流已软删除");
        Ok(result)
    }

    /// 硬删除工作流（谨慎使用）
    #[instrument(skip(db))]
    pub async fn hard_delete(
        db: &DatabaseConnection,
        id: Uuid,
    ) -> Result<(), AiStudioError> {
        warn!(workflow_id = %id, "硬删除工作流");

        let result = Workflow::delete_by_id(id).exec(db).await?;
        if result.rows_affected == 0 {
            return Err(AiStudioError::not_found("工作流"));
        }

        warn!(workflow_id = %id, "工作流已硬删除");
        Ok(())
    }

    /// 获取工作流统计信息
    #[instrument(skip(db))]
    pub async fn get_stats_by_tenant(
        db: &DatabaseConnection,
        tenant_id: Uuid,
    ) -> Result<WorkflowStats, AiStudioError> {
        let workflows = Self::find_by_tenant(db, tenant_id, None, None).await?;
        
        let total_count = workflows.len() as u32;
        let active_count = workflows.iter().filter(|wf| wf.is_active()).count() as u32;
        let draft_count = workflows.iter().filter(|wf| wf.is_draft()).count() as u32;

        // 计算总执行次数
        let mut total_executions = 0u64;
        let mut successful_executions = 0u64;
        
        for workflow in &workflows {
            if let Ok(stats) = workflow.get_execution_stats() {
                total_executions += stats.total_executions;
                successful_executions += stats.successful_executions;
            }
        }

        Ok(WorkflowStats {
            total_count,
            active_count,
            draft_count,
            total_executions,
            successful_executions,
        })
    }
}

/// 工作流统计信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WorkflowStats {
    /// 工作流总数
    pub total_count: u32,
    /// 活跃工作流数
    pub active_count: u32,
    /// 草稿工作流数
    pub draft_count: u32,
    /// 总执行次数
    pub total_executions: u64,
    /// 成功执行次数
    pub successful_executions: u64,
}