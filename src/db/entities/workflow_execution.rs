// 工作流执行记录实体定义

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 工作流执行状态枚举
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "workflow_execution_status")]
pub enum WorkflowExecutionStatus {
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
    #[sea_orm(string_value = "paused")]
    Paused,
    #[sea_orm(string_value = "timeout")]
    Timeout,
}

/// 工作流执行记录实体
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "workflow_executions")]
pub struct Model {
    /// 执行记录 ID
    #[sea_orm(primary_key)]
    pub id: Uuid,
    
    /// 工作流 ID
    pub workflow_id: Uuid,
    
    /// 租户 ID（冗余字段，便于查询）
    pub tenant_id: Uuid,
    
    /// 触发用户 ID
    pub triggered_by: Uuid,
    
    /// 执行状态
    pub status: WorkflowExecutionStatus,
    
    /// 输入数据（JSON 格式）
    #[sea_orm(column_type = "Json")]
    pub input: Json,
    
    /// 输出数据（JSON 格式）
    #[sea_orm(column_type = "Json", nullable)]
    pub output: Option<Json>,
    
    /// 执行上下文（JSON 格式）
    #[sea_orm(column_type = "Json")]
    pub context: Json,
    
    /// 当前执行节点 ID
    #[sea_orm(column_type = "String(Some(255))", nullable)]
    pub current_node_id: Option<String>,
    
    /// 执行路径（已完成的节点列表）
    #[sea_orm(column_type = "Json")]
    pub execution_path: Json,
    
    /// 节点执行状态（JSON 格式）
    #[sea_orm(column_type = "Json")]
    pub node_states: Json,
    
    /// 错误信息
    #[sea_orm(column_type = "Text", nullable)]
    pub error_message: Option<String>,
    
    /// 错误详情（JSON 格式）
    #[sea_orm(column_type = "Json", nullable)]
    pub error_details: Option<Json>,
    
    /// 执行指标（JSON 格式）
    #[sea_orm(column_type = "Json")]
    pub metrics: Json,
    
    /// 检查点数据（用于断点续传）
    #[sea_orm(column_type = "Json", nullable)]
    pub checkpoint_data: Option<Json>,
    
    /// 开始时间
    #[sea_orm(nullable)]
    pub started_at: Option<DateTimeWithTimeZone>,
    
    /// 完成时间
    #[sea_orm(nullable)]
    pub completed_at: Option<DateTimeWithTimeZone>,
    
    /// 暂停时间
    #[sea_orm(nullable)]
    pub paused_at: Option<DateTimeWithTimeZone>,
    
    /// 执行耗时（毫秒）
    #[sea_orm(nullable)]
    pub duration_ms: Option<i64>,
    
    /// 重试次数
    pub retry_count: i32,
    
    /// 最大重试次数
    pub max_retries: i32,
    
    /// 父执行 ID（用于子工作流）
    #[sea_orm(nullable)]
    pub parent_execution_id: Option<Uuid>,
    
    /// 创建时间
    pub created_at: DateTimeWithTimeZone,
    
    /// 更新时间
    pub updated_at: DateTimeWithTimeZone,
}

/// 工作流执行记录关联关系
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 多对一：执行记录 -> 工作流
    #[sea_orm(
        belongs_to = "super::workflow::Entity",
        from = "Column::WorkflowId",
        to = "super::workflow::Column::Id"
    )]
    Workflow,
    
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
    
    /// 自关联：父执行记录
    #[sea_orm(
        belongs_to = "Entity",
        from = "Column::ParentExecutionId",
        to = "Column::Id"
    )]
    ParentExecution,
    
    /// 一对多：子执行记录
    #[sea_orm(has_many = "Entity")]
    ChildExecutions,
    
    /// 一对多：步骤执行记录
    #[sea_orm(has_many = "super::step_execution::Entity")]
    StepExecutions,
}

/// 实现与工作流的关联
impl Related<super::workflow::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Workflow.def()
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

/// 实现与步骤执行记录的关联
impl Related<super::step_execution::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::StepExecutions.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

/// 工作流执行输入
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowExecutionInput {
    /// 输入参数
    pub parameters: std::collections::HashMap<String, serde_json::Value>,
    /// 文件附件
    pub attachments: Vec<WorkflowAttachment>,
    /// 触发事件
    pub trigger_event: Option<TriggerEvent>,
    /// 执行选项
    pub execution_options: ExecutionOptions,
}

/// 工作流执行输出
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowExecutionOutput {
    /// 输出结果
    pub results: std::collections::HashMap<String, serde_json::Value>,
    /// 生成的文件
    pub generated_files: Vec<WorkflowAttachment>,
    /// 执行摘要
    pub summary: ExecutionSummary,
}

/// 工作流附件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowAttachment {
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

/// 触发事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerEvent {
    /// 事件类型
    pub event_type: String,
    /// 事件数据
    pub event_data: serde_json::Value,
    /// 事件时间
    pub timestamp: DateTimeWithTimeZone,
    /// 事件来源
    pub source: String,
}

/// 执行选项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionOptions {
    /// 是否异步执行
    pub async_execution: bool,
    /// 优先级
    pub priority: String,
    /// 超时时间（秒）
    pub timeout_seconds: Option<u32>,
    /// 是否启用检查点
    pub enable_checkpoints: bool,
    /// 通知设置
    pub notifications: NotificationSettings,
}

/// 通知设置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationSettings {
    /// 是否在完成时通知
    pub notify_on_completion: bool,
    /// 是否在失败时通知
    pub notify_on_failure: bool,
    /// 通知方式
    pub notification_channels: Vec<String>,
    /// 通知接收者
    pub recipients: Vec<String>,
}

/// 执行上下文
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowExecutionContext {
    /// 全局变量
    pub global_variables: std::collections::HashMap<String, serde_json::Value>,
    /// 节点间共享数据
    pub shared_data: std::collections::HashMap<String, serde_json::Value>,
    /// 执行环境
    pub environment: std::collections::HashMap<String, String>,
    /// 会话信息
    pub session_info: Option<SessionInfo>,
}

/// 会话信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    /// 会话 ID
    pub session_id: String,
    /// 用户 ID
    pub user_id: Uuid,
    /// 会话开始时间
    pub started_at: DateTimeWithTimeZone,
    /// 会话数据
    pub session_data: std::collections::HashMap<String, serde_json::Value>,
}

/// 执行路径
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPath {
    /// 已完成的节点
    pub completed_nodes: Vec<NodeExecution>,
    /// 当前执行的节点
    pub current_nodes: Vec<String>,
    /// 待执行的节点
    pub pending_nodes: Vec<String>,
}

/// 节点执行记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeExecution {
    /// 节点 ID
    pub node_id: String,
    /// 执行状态
    pub status: String,
    /// 开始时间
    pub started_at: DateTimeWithTimeZone,
    /// 完成时间
    pub completed_at: Option<DateTimeWithTimeZone>,
    /// 输入数据
    pub input: serde_json::Value,
    /// 输出数据
    pub output: Option<serde_json::Value>,
    /// 错误信息
    pub error: Option<String>,
    /// 重试次数
    pub retry_count: u32,
}

/// 节点状态映射
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeStates {
    /// 节点状态
    pub states: std::collections::HashMap<String, NodeState>,
}

/// 节点状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeState {
    /// 状态
    pub status: String,
    /// 输入数据
    pub input: Option<serde_json::Value>,
    /// 输出数据
    pub output: Option<serde_json::Value>,
    /// 错误信息
    pub error: Option<String>,
    /// 开始时间
    pub started_at: Option<DateTimeWithTimeZone>,
    /// 完成时间
    pub completed_at: Option<DateTimeWithTimeZone>,
    /// 重试次数
    pub retry_count: u32,
}

/// 执行指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowExecutionMetrics {
    /// 总节点数
    pub total_nodes: u32,
    /// 已完成节点数
    pub completed_nodes: u32,
    /// 失败节点数
    pub failed_nodes: u32,
    /// 跳过节点数
    pub skipped_nodes: u32,
    /// 总执行时间（毫秒）
    pub total_execution_time_ms: i64,
    /// 各节点执行时间
    pub node_execution_times: std::collections::HashMap<String, i64>,
    /// 资源使用情况
    pub resource_usage: ResourceUsage,
}

/// 资源使用情况
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    /// 峰值内存使用（MB）
    pub peak_memory_mb: f64,
    /// CPU 使用时间（毫秒）
    pub cpu_time_ms: i64,
    /// 网络请求次数
    pub network_requests: u32,
    /// API 调用次数
    pub api_calls: u32,
}

/// 执行摘要
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionSummary {
    /// 执行结果
    pub result: String,
    /// 成功节点数
    pub successful_nodes: u32,
    /// 失败节点数
    pub failed_nodes: u32,
    /// 总耗时
    pub total_duration_ms: i64,
    /// 关键指标
    pub key_metrics: std::collections::HashMap<String, serde_json::Value>,
}

/// 检查点数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointData {
    /// 检查点时间
    pub timestamp: DateTimeWithTimeZone,
    /// 当前状态
    pub current_state: WorkflowExecutionContext,
    /// 执行进度
    pub progress: ExecutionPath,
    /// 节点状态
    pub node_states: NodeStates,
    /// 检查点版本
    pub version: u32,
}

impl Default for WorkflowExecutionInput {
    fn default() -> Self {
        Self {
            parameters: std::collections::HashMap::new(),
            attachments: Vec::new(),
            trigger_event: None,
            execution_options: ExecutionOptions::default(),
        }
    }
}

impl Default for ExecutionOptions {
    fn default() -> Self {
        Self {
            async_execution: true,
            priority: "normal".to_string(),
            timeout_seconds: Some(3600), // 1 hour
            enable_checkpoints: true,
            notifications: NotificationSettings::default(),
        }
    }
}

impl Default for NotificationSettings {
    fn default() -> Self {
        Self {
            notify_on_completion: false,
            notify_on_failure: true,
            notification_channels: vec!["email".to_string()],
            recipients: Vec::new(),
        }
    }
}

impl Default for WorkflowExecutionContext {
    fn default() -> Self {
        Self {
            global_variables: std::collections::HashMap::new(),
            shared_data: std::collections::HashMap::new(),
            environment: std::collections::HashMap::new(),
            session_info: None,
        }
    }
}

impl Default for WorkflowExecutionMetrics {
    fn default() -> Self {
        Self {
            total_nodes: 0,
            completed_nodes: 0,
            failed_nodes: 0,
            skipped_nodes: 0,
            total_execution_time_ms: 0,
            node_execution_times: std::collections::HashMap::new(),
            resource_usage: ResourceUsage::default(),
        }
    }
}

impl Default for ResourceUsage {
    fn default() -> Self {
        Self {
            peak_memory_mb: 0.0,
            cpu_time_ms: 0,
            network_requests: 0,
            api_calls: 0,
        }
    }
}

/// 工作流执行记录实用方法
impl Model {
    /// 检查执行是否完成
    pub fn is_completed(&self) -> bool {
        matches!(
            self.status,
            WorkflowExecutionStatus::Completed
                | WorkflowExecutionStatus::Failed
                | WorkflowExecutionStatus::Cancelled
                | WorkflowExecutionStatus::Timeout
        )
    }
    
    /// 检查执行是否成功
    pub fn is_successful(&self) -> bool {
        self.status == WorkflowExecutionStatus::Completed
    }
    
    /// 检查执行是否失败
    pub fn is_failed(&self) -> bool {
        matches!(
            self.status,
            WorkflowExecutionStatus::Failed | WorkflowExecutionStatus::Timeout
        )
    }
    
    /// 检查执行是否正在运行
    pub fn is_running(&self) -> bool {
        self.status == WorkflowExecutionStatus::Running
    }
    
    /// 检查执行是否暂停
    pub fn is_paused(&self) -> bool {
        self.status == WorkflowExecutionStatus::Paused
    }
    
    /// 检查是否可以重试
    pub fn can_retry(&self) -> bool {
        self.is_failed() && self.retry_count < self.max_retries
    }
    
    /// 检查是否可以恢复
    pub fn can_resume(&self) -> bool {
        self.is_paused() && self.checkpoint_data.is_some()
    }
    
    /// 获取执行输入
    pub fn get_input(&self) -> Result<WorkflowExecutionInput, serde_json::Error> {
        serde_json::from_value(self.input.clone())
    }
    
    /// 获取执行输出
    pub fn get_output(&self) -> Result<Option<WorkflowExecutionOutput>, serde_json::Error> {
        if let Some(output) = &self.output {
            Ok(Some(serde_json::from_value(output.clone())?))
        } else {
            Ok(None)
        }
    }
    
    /// 获取执行上下文
    pub fn get_context(&self) -> Result<WorkflowExecutionContext, serde_json::Error> {
        serde_json::from_value(self.context.clone())
    }
    
    /// 获取执行路径
    pub fn get_execution_path(&self) -> Result<ExecutionPath, serde_json::Error> {
        serde_json::from_value(self.execution_path.clone())
    }
    
    /// 获取节点状态
    pub fn get_node_states(&self) -> Result<NodeStates, serde_json::Error> {
        serde_json::from_value(self.node_states.clone())
    }
    
    /// 获取执行指标
    pub fn get_metrics(&self) -> Result<WorkflowExecutionMetrics, serde_json::Error> {
        serde_json::from_value(self.metrics.clone())
    }
    
    /// 获取检查点数据
    pub fn get_checkpoint_data(&self) -> Result<Option<CheckpointData>, serde_json::Error> {
        if let Some(checkpoint) = &self.checkpoint_data {
            Ok(Some(serde_json::from_value(checkpoint.clone())?))
        } else {
            Ok(None)
        }
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
    pub fn progress_percentage(&self) -> Result<f32, serde_json::Error> {
        let metrics = self.get_metrics()?;
        if metrics.total_nodes == 0 {
            Ok(0.0)
        } else {
            Ok((metrics.completed_nodes as f32 / metrics.total_nodes as f32) * 100.0)
        }
    }
    
    /// 获取状态显示名称
    pub fn status_display_name(&self) -> &'static str {
        match self.status {
            WorkflowExecutionStatus::Pending => "等待中",
            WorkflowExecutionStatus::Running => "执行中",
            WorkflowExecutionStatus::Completed => "已完成",
            WorkflowExecutionStatus::Failed => "执行失败",
            WorkflowExecutionStatus::Cancelled => "已取消",
            WorkflowExecutionStatus::Paused => "已暂停",
            WorkflowExecutionStatus::Timeout => "执行超时",
        }
    }
    
    /// 检查是否超时
    pub fn is_timeout(&self) -> bool {
        if let Some(started) = self.started_at {
            if let Ok(input) = self.get_input() {
                if let Some(timeout) = input.execution_options.timeout_seconds {
                    let now = chrono::Utc::now();
                    let started_utc = started.with_timezone(&chrono::Utc);
                    let elapsed = (now - started_utc).num_seconds() as u32;
                    return elapsed > timeout;
                }
            }
        }
        false
    }
}