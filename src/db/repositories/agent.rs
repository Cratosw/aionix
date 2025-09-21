// Agent 仓储实现

use crate::db::entities::{agent, prelude::*};
use crate::errors::AiStudioError;
use sea_orm::{prelude::*, *};
use uuid::Uuid;
use tracing::{info, warn, instrument};

/// Agent 仓储
pub struct AgentRepository;

impl AgentRepository {
    /// 创建新 Agent
    #[instrument(skip(db, system_prompt))]
    pub async fn create(
        db: &DatabaseConnection,
        tenant_id: Uuid,
        name: String,
        description: Option<String>,
        agent_type: agent::AgentType,
        system_prompt: String,
        created_by: Uuid,
    ) -> Result<agent::Model, AiStudioError> {
        info!(tenant_id = %tenant_id, name = %name, "创建新 Agent");

        // 检查 Agent 名称在租户内是否已存在
        if Self::exists_by_name_in_tenant(db, tenant_id, &name).await? {
            return Err(AiStudioError::conflict(format!("Agent 名称 '{}' 在该租户内已存在", name)));
        }

        let agent = agent::ActiveModel {
            id: Set(Uuid::new_v4()),
            tenant_id: Set(tenant_id),
            name: Set(name),
            description: Set(description),
            agent_type: Set(agent_type),
            status: Set(agent::AgentStatus::Draft),
            version: Set("1.0.0".to_string()),
            config: Set(serde_json::to_value(agent::AgentConfig::default())?),
            system_prompt: Set(system_prompt),
            tools: Set(serde_json::to_value(Vec::<agent::AgentTool>::new())?),
            capabilities: Set(serde_json::to_value(agent::AgentCapabilities::default())?),
            metadata: Set(serde_json::to_value(agent::AgentMetadata::default())?),
            execution_stats: Set(serde_json::to_value(agent::AgentExecutionStats::default())?),
            last_executed_at: Set(None),
            created_by: Set(created_by),
            created_at: Set(chrono::Utc::now().into()),
            updated_at: Set(chrono::Utc::now().into()),
        };

        let result = agent.insert(db).await?;
        info!(agent_id = %result.id, "Agent 创建成功");
        Ok(result)
    }

    /// 根据 ID 查找 Agent
    #[instrument(skip(db))]
    pub async fn find_by_id(
        db: &DatabaseConnection,
        id: Uuid,
    ) -> Result<Option<agent::Model>, AiStudioError> {
        let agent = Agent::find_by_id(id).one(db).await?;
        Ok(agent)
    }

    /// 根据名称和租户 ID 查找 Agent
    #[instrument(skip(db))]
    pub async fn find_by_name_in_tenant(
        db: &DatabaseConnection,
        tenant_id: Uuid,
        name: &str,
    ) -> Result<Option<agent::Model>, AiStudioError> {
        let agent = Agent::find()
            .filter(agent::Column::TenantId.eq(tenant_id))
            .filter(agent::Column::Name.eq(name))
            .one(db)
            .await?;
        Ok(agent)
    }

    /// 检查 Agent 名称在租户内是否存在
    #[instrument(skip(db))]
    pub async fn exists_by_name_in_tenant(
        db: &DatabaseConnection,
        tenant_id: Uuid,
        name: &str,
    ) -> Result<bool, AiStudioError> {
        let count = Agent::find()
            .filter(agent::Column::TenantId.eq(tenant_id))
            .filter(agent::Column::Name.eq(name))
            .count(db)
            .await?;
        Ok(count > 0)
    }

    /// 更新 Agent 信息
    #[instrument(skip(db, agent))]
    pub async fn update(
        db: &DatabaseConnection,
        agent: agent::Model,
    ) -> Result<agent::Model, AiStudioError> {
        info!(agent_id = %agent.id, "更新 Agent 信息");

        let mut active_model: agent::ActiveModel = agent.into();
        active_model.updated_at = Set(chrono::Utc::now().into());

        let result = active_model.update(db).await?;
        info!(agent_id = %result.id, "Agent 信息更新成功");
        Ok(result)
    }

    /// 更新 Agent 状态
    #[instrument(skip(db))]
    pub async fn update_status(
        db: &DatabaseConnection,
        id: Uuid,
        status: agent::AgentStatus,
    ) -> Result<agent::Model, AiStudioError> {
        info!(agent_id = %id, status = ?status, "更新 Agent 状态");

        let agent = Self::find_by_id(db, id).await?
            .ok_or_else(|| AiStudioError::not_found("Agent"))?;

        let mut active_model: agent::ActiveModel = agent.into();
        active_model.status = Set(status);
        active_model.updated_at = Set(chrono::Utc::now().into());

        let result = active_model.update(db).await?;
        info!(agent_id = %result.id, "Agent 状态更新成功");
        Ok(result)
    }

    /// 更新 Agent 配置
    #[instrument(skip(db, config))]
    pub async fn update_config(
        db: &DatabaseConnection,
        id: Uuid,
        config: agent::AgentConfig,
    ) -> Result<agent::Model, AiStudioError> {
        info!(agent_id = %id, "更新 Agent 配置");

        let agent = Self::find_by_id(db, id).await?
            .ok_or_else(|| AiStudioError::not_found("Agent"))?;

        let mut active_model: agent::ActiveModel = agent.into();
        active_model.config = Set(serde_json::to_value(config)?);
        active_model.updated_at = Set(chrono::Utc::now().into());

        let result = active_model.update(db).await?;
        info!(agent_id = %result.id, "Agent 配置更新成功");
        Ok(result)
    }

    /// 更新 Agent 工具列表
    #[instrument(skip(db, tools))]
    pub async fn update_tools(
        db: &DatabaseConnection,
        id: Uuid,
        tools: Vec<agent::AgentTool>,
    ) -> Result<agent::Model, AiStudioError> {
        info!(agent_id = %id, tool_count = tools.len(), "更新 Agent 工具");

        let agent = Self::find_by_id(db, id).await?
            .ok_or_else(|| AiStudioError::not_found("Agent"))?;

        let mut active_model: agent::ActiveModel = agent.into();
        active_model.tools = Set(serde_json::to_value(tools)?);
        active_model.updated_at = Set(chrono::Utc::now().into());

        let result = active_model.update(db).await?;
        info!(agent_id = %result.id, "Agent 工具更新成功");
        Ok(result)
    }

    /// 更新最后执行时间
    #[instrument(skip(db))]
    pub async fn update_last_executed(
        db: &DatabaseConnection,
        id: Uuid,
    ) -> Result<(), AiStudioError> {
        Agent::update_many()
            .col_expr(agent::Column::LastExecutedAt, Expr::value(chrono::Utc::now()))
            .col_expr(agent::Column::UpdatedAt, Expr::value(chrono::Utc::now()))
            .filter(agent::Column::Id.eq(id))
            .exec(db)
            .await?;
        Ok(())
    }

    /// 更新执行统计
    #[instrument(skip(db, stats))]
    pub async fn update_execution_stats(
        db: &DatabaseConnection,
        id: Uuid,
        stats: agent::AgentExecutionStats,
    ) -> Result<(), AiStudioError> {
        Agent::update_many()
            .col_expr(agent::Column::ExecutionStats, Expr::value(serde_json::to_value(stats)?))
            .col_expr(agent::Column::UpdatedAt, Expr::value(chrono::Utc::now()))
            .filter(agent::Column::Id.eq(id))
            .exec(db)
            .await?;
        Ok(())
    }

    /// 获取租户内的 Agent 列表
    #[instrument(skip(db))]
    pub async fn find_by_tenant(
        db: &DatabaseConnection,
        tenant_id: Uuid,
        limit: Option<u64>,
        offset: Option<u64>,
    ) -> Result<Vec<agent::Model>, AiStudioError> {
        let mut query = Agent::find()
            .filter(agent::Column::TenantId.eq(tenant_id))
            .order_by_desc(agent::Column::UpdatedAt);

        if let Some(limit) = limit {
            query = query.limit(limit);
        }

        if let Some(offset) = offset {
            query = query.offset(offset);
        }

        let agents = query.all(db).await?;
        Ok(agents)
    }

    /// 获取活跃 Agent 列表
    #[instrument(skip(db))]
    pub async fn find_active_by_tenant(
        db: &DatabaseConnection,
        tenant_id: Uuid,
        limit: Option<u64>,
        offset: Option<u64>,
    ) -> Result<Vec<agent::Model>, AiStudioError> {
        let mut query = Agent::find()
            .filter(agent::Column::TenantId.eq(tenant_id))
            .filter(agent::Column::Status.eq(agent::AgentStatus::Active))
            .order_by_desc(agent::Column::LastExecutedAt);

        if let Some(limit) = limit {
            query = query.limit(limit);
        }

        if let Some(offset) = offset {
            query = query.offset(offset);
        }

        let agents = query.all(db).await?;
        Ok(agents)
    }

    /// 按类型查找 Agent
    #[instrument(skip(db))]
    pub async fn find_by_type_in_tenant(
        db: &DatabaseConnection,
        tenant_id: Uuid,
        agent_type: agent::AgentType,
        limit: Option<u64>,
        offset: Option<u64>,
    ) -> Result<Vec<agent::Model>, AiStudioError> {
        let mut query = Agent::find()
            .filter(agent::Column::TenantId.eq(tenant_id))
            .filter(agent::Column::AgentType.eq(agent_type))
            .order_by_desc(agent::Column::UpdatedAt);

        if let Some(limit) = limit {
            query = query.limit(limit);
        }

        if let Some(offset) = offset {
            query = query.offset(offset);
        }

        let agents = query.all(db).await?;
        Ok(agents)
    }

    /// 搜索 Agent
    #[instrument(skip(db))]
    pub async fn search_in_tenant(
        db: &DatabaseConnection,
        tenant_id: Uuid,
        query: &str,
        limit: Option<u64>,
        offset: Option<u64>,
    ) -> Result<Vec<agent::Model>, AiStudioError> {
        let search_pattern = format!("%{}%", query);
        
        let mut search_query = Agent::find()
            .filter(agent::Column::TenantId.eq(tenant_id))
            .filter(
                Condition::any()
                    .add(agent::Column::Name.like(&search_pattern))
                    .add(agent::Column::Description.like(&search_pattern))
            )
            .order_by_desc(agent::Column::UpdatedAt);

        if let Some(limit) = limit {
            search_query = search_query.limit(limit);
        }

        if let Some(offset) = offset {
            search_query = search_query.offset(offset);
        }

        let agents = search_query.all(db).await?;
        Ok(agents)
    }

    /// 获取租户内 Agent 总数
    #[instrument(skip(db))]
    pub async fn count_by_tenant(
        db: &DatabaseConnection,
        tenant_id: Uuid,
    ) -> Result<u64, AiStudioError> {
        let count = Agent::find()
            .filter(agent::Column::TenantId.eq(tenant_id))
            .count(db)
            .await?;
        Ok(count)
    }

    /// 按状态统计 Agent 数量
    #[instrument(skip(db))]
    pub async fn count_by_status(
        db: &DatabaseConnection,
        tenant_id: Uuid,
        status: agent::AgentStatus,
    ) -> Result<u64, AiStudioError> {
        let count = Agent::find()
            .filter(agent::Column::TenantId.eq(tenant_id))
            .filter(agent::Column::Status.eq(status))
            .count(db)
            .await?;
        Ok(count)
    }

    /// 软删除 Agent
    #[instrument(skip(db))]
    pub async fn soft_delete(
        db: &DatabaseConnection,
        id: Uuid,
    ) -> Result<agent::Model, AiStudioError> {
        warn!(agent_id = %id, "软删除 Agent");

        let result = Self::update_status(db, id, agent::AgentStatus::Archived).await?;
        warn!(agent_id = %result.id, "Agent 已软删除");
        Ok(result)
    }

    /// 硬删除 Agent（谨慎使用）
    #[instrument(skip(db))]
    pub async fn hard_delete(
        db: &DatabaseConnection,
        id: Uuid,
    ) -> Result<(), AiStudioError> {
        warn!(agent_id = %id, "硬删除 Agent");

        let result = Agent::delete_by_id(id).exec(db).await?;
        if result.rows_affected == 0 {
            return Err(AiStudioError::not_found("Agent"));
        }

        warn!(agent_id = %id, "Agent 已硬删除");
        Ok(())
    }

    /// 获取 Agent 统计信息
    #[instrument(skip(db))]
    pub async fn get_stats_by_tenant(
        db: &DatabaseConnection,
        tenant_id: Uuid,
    ) -> Result<AgentStats, AiStudioError> {
        let agents = Self::find_by_tenant(db, tenant_id, None, None).await?;
        
        let total_count = agents.len() as u32;
        let active_count = agents.iter().filter(|agent| agent.is_active()).count() as u32;
        let draft_count = agents.iter().filter(|agent| agent.is_draft()).count() as u32;
        let archived_count = agents.iter().filter(|agent| agent.is_archived()).count() as u32;

        // 计算总执行次数
        let mut total_executions = 0u64;
        let mut successful_executions = 0u64;
        
        for agent in &agents {
            if let Ok(stats) = agent.get_execution_stats() {
                total_executions += stats.total_executions;
                successful_executions += stats.successful_executions;
            }
        }

        Ok(AgentStats {
            total_count,
            active_count,
            draft_count,
            archived_count,
            total_executions,
            successful_executions,
        })
    }
}

/// Agent 统计信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AgentStats {
    /// Agent 总数
    pub total_count: u32,
    /// 活跃 Agent 数
    pub active_count: u32,
    /// 草稿 Agent 数
    pub draft_count: u32,
    /// 已归档 Agent 数
    pub archived_count: u32,
    /// 总执行次数
    pub total_executions: u64,
    /// 成功执行次数
    pub successful_executions: u64,
}