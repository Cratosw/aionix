// Agent 执行记录实体定义

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Agent 执行状态枚举
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "agent_execution_status")]
pub enum AgentExecutionStatus {
    #[sea_orm(string_value = "pending")]
    Pending,
    #[sea_orm(string_value = "running")]
    Running,
    #[sea_orm(string_value = "completed")]
    Completed,
    #[sea_orm(string_value = "failed")]
    Failed,
    #[sea_orm(string_value = "cancelled")]
    Cancelled,
    #[sea_orm(string_value = "timeout")]
    Timeout,
}

/// Agent 执行优先级枚举
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "execution_priority")]
pub enum ExecutionPriority {
    #[sea_orm(string_value = "low")]
    Low,
    #[sea_orm(string_value = "normal")]
    Normal,
    #[sea_orm(string_value = "high")]
    High,
    #[sea_orm(string_value = "urgent")]
    Urgent,
}

/// Agent 执行记录实体
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "agent_executions")]
pub struct Model {
    /// 执行记录 ID
    #[sea_orm(primary_key)]
    pub id: Uuid,
    
    /// Agent ID
    pub agent_id: Uuid,
    
    /// 租户 ID（冗余字段，便于查询）
    pub tenant_id: Uuid,
    
    /// 触发用户 ID
    pub triggered_by: Uuid,
    
    /// 执行状态
    pub status: AgentExecutionStatus,
    
    /// 执行优先级
    pub priority: ExecutionPriority,
    
    /// 输入数据（JSON 格式）
    #[sea_orm(column_type = "Json")]
    pub input: Json,
    
    /// 输出数据（JSON 格式）
    #[sea_orm(column_type = "Json", nullable)]
    pub output: Option<Json>,
    
    /// 执行上下文（JSON 格式）
    #[sea_orm(column_type = "Json")]
    pub context: Json,
    
    /// 执行配置（JSON 格式）
    #[sea_orm(column_type = "Json")]
    pub execution_config: Json,
    
    /// 执行步骤记录（JSON 格式）
    #[sea_orm(column_type = "Json")]
    pub steps: Json,
    
    /// 错误信息
    #[sea_orm(column_type = "Text", nullable)]
    pub error_message: Option<String>,
    
    /// 错误详情（JSON 格式）
    #[sea_orm(column_type = "Json", nullable)]
    pub error_details: Option<Json>,
    
    /// 执行指标（JSON 格式）
    #[sea_orm(column_type = "Json")]
    pub metrics: Json,
    
    /// 开始时间
    #[sea_orm(nullable)]
    pub started_at: Option<DateTimeWithTimeZone>,
    
    /// 完成时间
    #[sea_orm(nullable)]
    pub completed_at: Option<DateTimeWithTimeZone>,
    
    /// 执行耗时（毫秒）
    #[sea_orm(nullable)]
    pub duration_ms: Option<i64>,
    
    /// 重试次数
    pub retry_count: i32,
    
    /// 最大重试次数
    pub max_retries: i32,
    
    /// 父执行 ID（用于重试或子任务）
    #[sea_orm(nullable)]
    pub parent_execution_id: Option<Uuid>,
    
    /// 工作流执行 ID（如果是工作流的一部分）
    #[sea_orm(nullable)]
    pub workflow_execution_id: Option<Uuid>,
    
    /// 创建时间
    pub created_at: DateTimeWithTimeZone,
    
    /// 更新时间
    pub updated_at: DateTimeWithTimeZone,
}

/// Agent 执行记录关联关系
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 多对一：执行记录 -> Agent
    #[sea_orm(
        belongs_to = "super::agent::Entity",
        from = "Column::AgentId",
        to = "super::agent::Column::Id"
    )]
    Agent,
    
    /// 多对一：执行记录 -> 租户
    #[sea_orm(
        belongs_to = "super::tenant::Entity",
        from = "Column::TenantId",
        to = "super::tenant::Column::Id"
    )]
    Tenant,
    
    /// 多对一：执行记录 -> 触发用户
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::TriggeredBy",
        to = "super::user::Column::Id"
    )]
    TriggeredBy,
}

/// 实现与 Agent 的关联
impl Related<super::agent::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Agent.def()
    }
}

/// 实现与租户的关联
impl Related<super::tenant::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Tenant.def()
    }
}

/// 实现与用户的关联
impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::TriggeredBy.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

/// 执行输入数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionInput {
    /// 用户消息或任务描述
    pub message: String,
    /// 附加参数
    pub parameters: std::collections::HashMap<String, serde_json::Value>,
    /// 文件附件
    pub attachments: Vec<ExecutionAttachment>,
    /// 上下文引用
    pub context_refs: Vec<String>,
}

/// 执行输出数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionOutput {
    /// 响应消息
    pub message: String,
    /// 结构化数据
    pub data: serde_json::Value,
    /// 生成的文件
    pub generated_files: Vec<ExecutionAttachment>,
    /// 工具调用结果
    pub tool_results: Vec<ToolCallResult>,
}

/// 执行附件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionAttachment {
    /// 文件名
    pub filename: String,
    /// 文件类型
    pub content_type: String,
    /// 文件大小
    pub size: u64,
    /// 文件路径或 URL
    pub url: String,
    /// 文件描述
    pub description: Option<String>,
}

/// 执行上下文
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionContext {
    /// 会话 ID
    pub session_id: Option<String>,
    /// 对话历史
    pub conversation_history: Vec<ConversationMessage>,
    /// 环境变量
    pub environment: std::collections::HashMap<String, String>,
    /// 临时数据
    pub temp_data: std::collections::HashMap<String, serde_json::Value>,
}

/// 对话消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMessage {
    /// 消息角色
    pub role: String, // "user", "assistant", "system"
    /// 消息内容
    pub content: String,
    /// 时间戳
    pub timestamp: DateTimeWithTimeZone,
}

/// 执行步骤
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStep {
    /// 步骤 ID
    pub step_id: String,
    /// 步骤名称
    pub name: String,
    /// 步骤类型
    pub step_type: String,
    /// 步骤状态
    pub status: String,
    /// 输入数据
    pub input: serde_json::Value,
    /// 输出数据
    pub output: Option<serde_json::Value>,
    /// 错误信息
    pub error: Option<String>,
    /// 开始时间
    pub started_at: DateTimeWithTimeZone,
    /// 完成时间
    pub completed_at: Option<DateTimeWithTimeZone>,
    /// 耗时（毫秒）
    pub duration_ms: Option<i64>,
}

/// 工具调用结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallResult {
    /// 工具名称
    pub tool_name: String,
    /// 调用参数
    pub parameters: serde_json::Value,
    /// 调用结果
    pub result: serde_json::Value,
    /// 是否成功
    pub success: bool,
    /// 错误信息
    pub error: Option<String>,
    /// 耗时（毫秒）
    pub duration_ms: i64,
}

/// 执行指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionMetrics {
    /// 总 token 使用量
    pub total_tokens: u32,
    /// 输入 token 数
    pub input_tokens: u32,
    /// 输出 token 数
    pub output_tokens: u32,
    /// API 调用次数
    pub api_calls: u32,
    /// 工具调用次数
    pub tool_calls: u32,
    /// 内存使用峰值（MB）
    pub peak_memory_mb: f64,
    /// CPU 使用时间（毫秒）
    pub cpu_time_ms: i64,
    /// 网络请求次数
    pub network_requests: u32,
    /// 缓存命中次数
    pub cache_hits: u32,
    /// 缓存未命中次数
    pub cache_misses: u32,
}

impl Default for ExecutionInput {
    fn default() -> Self {
        Self {
            message: String::new(),
            parameters: std::collections::HashMap::new(),
            attachments: Vec::new(),
            context_refs: Vec::new(),
        }
    }
}

impl Default for ExecutionOutput {
    fn default() -> Self {
        Self {
            message: String::new(),
            data: serde_json::Value::Null,
            generated_files: Vec::new(),
            tool_results: Vec::new(),
        }
    }
}

impl Default for ExecutionContext {
    fn default() -> Self {
        Self {
            session_id: None,
            conversation_history: Vec::new(),
            environment: std::collections::HashMap::new(),
            temp_data: std::collections::HashMap::new(),
        }
    }
}

impl Default for ExecutionMetrics {
    fn default() -> Self {
        Self {
            total_tokens: 0,
            input_tokens: 0,
            output_tokens: 0,
            api_calls: 0,
            tool_calls: 0,
            peak_memory_mb: 0.0,
            cpu_time_ms: 0,
            network_requests: 0,
            cache_hits: 0,
            cache_misses: 0,
        }
    }
}

/// Agent 执行记录实用方法
impl Model {
    /// 检查执行是否完成
    pub fn is_completed(&self) -> bool {
        matches!(
            self.status,
            AgentExecutionStatus::Completed
                | AgentExecutionStatus::Failed
                | AgentExecutionStatus::Cancelled
                | AgentExecutionStatus::Timeout
        )
    }
    
    /// 检查执行是否成功
    pub fn is_successful(&self) -> bool {
        self.status == AgentExecutionStatus::Completed
    }
    
    /// 检查执行是否失败
    pub fn is_failed(&self) -> bool {
        matches!(
            self.status,
            AgentExecutionStatus::Failed | AgentExecutionStatus::Timeout
        )
    }
    
    /// 检查执行是否正在运行
    pub fn is_running(&self) -> bool {
        self.status == AgentExecutionStatus::Running
    }
    
    /// 检查是否可以重试
    pub fn can_retry(&self) -> bool {
        self.is_failed() && self.retry_count < self.max_retries
    }
    
    /// 获取执行输入
    pub fn get_input(&self) -> Result<ExecutionInput, serde_json::Error> {
        serde_json::from_value(self.input.clone())
    }
    
    /// 获取执行输出
    pub fn get_output(&self) -> Result<Option<ExecutionOutput>, serde_json::Error> {
        if let Some(output) = &self.output {
            Ok(Some(serde_json::from_value(output.clone())?))
        } else {
            Ok(None)
        }
    }
    
    /// 获取执行上下文
    pub fn get_context(&self) -> Result<ExecutionContext, serde_json::Error> {
        serde_json::from_value(self.context.clone())
    }
    
    /// 获取执行步骤
    pub fn get_steps(&self) -> Result<Vec<ExecutionStep>, serde_json::Error> {
        serde_json::from_value(self.steps.clone())
    }
    
    /// 获取执行指标
    pub fn get_metrics(&self) -> Result<ExecutionMetrics, serde_json::Error> {
        serde_json::from_value(self.metrics.clone())
    }
    
    /// 计算执行耗时
    pub fn calculate_duration(&self) -> Option<chrono::Duration> {
        if let (Some(start), Some(end)) = (self.started_at, self.completed_at) {
            let start_utc = start.with_timezone(&chrono::Utc);
            let end_utc = end.with_timezone(&chrono::Utc);
            Some(end_utc - start_utc)
        } else {
            None
        }
    }
    
    /// 获取执行进度百分比
    pub fn progress_percentage(&self) -> f32 {
        match self.status {
            AgentExecutionStatus::Pending => 0.0,
            AgentExecutionStatus::Running => {
                // 可以根据步骤完成情况计算更精确的进度
                if let Ok(steps) = self.get_steps() {
                    let completed_steps = steps.iter().filter(|s| s.status == "completed").count();
                    if steps.is_empty() {
                        50.0 // 默认进度
                    } else {
                        (completed_steps as f32 / steps.len() as f32) * 100.0
                    }
                } else {
                    50.0
                }
            }
            AgentExecutionStatus::Completed => 100.0,
            AgentExecutionStatus::Failed
            | AgentExecutionStatus::Cancelled
            | AgentExecutionStatus::Timeout => 0.0,
        }
    }
    
    /// 获取状态显示名称
    pub fn status_display_name(&self) -> &'static str {
        match self.status {
            AgentExecutionStatus::Pending => "等待中",
            AgentExecutionStatus::Running => "执行中",
            AgentExecutionStatus::Completed => "已完成",
            AgentExecutionStatus::Failed => "执行失败",
            AgentExecutionStatus::Cancelled => "已取消",
            AgentExecutionStatus::Timeout => "执行超时",
        }
    }
    
    /// 获取优先级显示名称
    pub fn priority_display_name(&self) -> &'static str {
        match self.priority {
            ExecutionPriority::Low => "低",
            ExecutionPriority::Normal => "普通",
            ExecutionPriority::High => "高",
            ExecutionPriority::Urgent => "紧急",
        }
    }
    
    /// 检查是否超时
    pub fn is_timeout(&self) -> bool {
        if let Some(started) = self.started_at {
            if let Ok(config) = serde_json::from_value::<serde_json::Value>(self.execution_config.clone()) {
                if let Some(timeout) = config.get("max_execution_time").and_then(|v| v.as_u64()) {
                    let now = chrono::Utc::now();
                    let started_utc = started.with_timezone(&chrono::Utc);
                    let elapsed = (now - started_utc).num_seconds() as u64;
                    return elapsed > timeout;
                }
            }
        }
        false
    }
}