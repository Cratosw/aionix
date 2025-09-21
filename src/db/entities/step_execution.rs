// 步骤执行记录实体定义

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 步骤执行状态枚举
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "step_execution_status")]
pub enum StepExecutionStatus {
    #[sea_orm(string_value = "pending")]
    Pending,
    #[sea_orm(string_value = "running")]
    Running,
    #[sea_orm(string_value = "completed")]
    Completed,
    #[sea_orm(string_value = "failed")]
    Failed,
    #[sea_orm(string_value = "skipped")]
    Skipped,
    #[sea_orm(string_value = "cancelled")]
    Cancelled,
    #[sea_orm(string_value = "timeout")]
    Timeout,
}

/// 步骤类型枚举
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "step_type")]
pub enum StepType {
    #[sea_orm(string_value = "agent")]
    Agent,
    #[sea_orm(string_value = "condition")]
    Condition,
    #[sea_orm(string_value = "loop")]
    Loop,
    #[sea_orm(string_value = "parallel")]
    Parallel,
    #[sea_orm(string_value = "merge")]
    Merge,
    #[sea_orm(string_value = "input")]
    Input,
    #[sea_orm(string_value = "output")]
    Output,
    #[sea_orm(string_value = "delay")]
    Delay,
    #[sea_orm(string_value = "custom")]
    Custom,
}

/// 步骤执行记录实体
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "step_executions")]
pub struct Model {
    /// 步骤执行 ID
    #[sea_orm(primary_key)]
    pub id: Uuid,
    
    /// 工作流执行 ID
    pub workflow_execution_id: Uuid,
    
    /// 租户 ID（冗余字段，便于查询）
    pub tenant_id: Uuid,
    
    /// 步骤 ID（来自工作流定义）
    #[sea_orm(column_type = "String(Some(255))")]
    pub step_id: String,
    
    /// 步骤名称
    #[sea_orm(column_type = "String(Some(255))")]
    pub step_name: String,
    
    /// 步骤类型
    pub step_type: StepType,
    
    /// 执行状态
    pub status: StepExecutionStatus,
    
    /// 步骤序号（在工作流中的执行顺序）
    pub step_order: i32,
    
    /// 输入数据（JSON 格式）
    #[sea_orm(column_type = "Json")]
    pub input: Json,
    
    /// 输出数据（JSON 格式）
    #[sea_orm(column_type = "Json", nullable)]
    pub output: Option<Json>,
    
    /// 步骤配置（JSON 格式）
    #[sea_orm(column_type = "Json")]
    pub step_config: Json,
    
    /// 执行上下文（JSON 格式）
    #[sea_orm(column_type = "Json")]
    pub context: Json,
    
    /// 错误信息
    #[sea_orm(column_type = "Text", nullable)]
    pub error_message: Option<String>,
    
    /// 错误详情（JSON 格式）
    #[sea_orm(column_type = "Json", nullable)]
    pub error_details: Option<Json>,
    
    /// 执行指标（JSON 格式）
    #[sea_orm(column_type = "Json")]
    pub metrics: Json,
    
    /// 关联的 Agent 执行 ID（如果是 Agent 步骤）
    #[sea_orm(nullable)]
    pub agent_execution_id: Option<Uuid>,
    
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
    
    /// 父步骤 ID（用于嵌套步骤）
    #[sea_orm(nullable)]
    pub parent_step_id: Option<Uuid>,
    
    /// 创建时间
    pub created_at: DateTimeWithTimeZone,
    
    /// 更新时间
    pub updated_at: DateTimeWithTimeZone,
}

/// 步骤执行记录关联关系
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 多对一：步骤执行 -> 工作流执行
    #[sea_orm(
        belongs_to = "super::workflow_execution::Entity",
        from = "Column::WorkflowExecutionId",
        to = "super::workflow_execution::Column::Id"
    )]
    WorkflowExecution,
    
    /// 多对一：步骤执行 -> 租户
    #[sea_orm(
        belongs_to = "super::tenant::Entity",
        from = "Column::TenantId",
        to = "super::tenant::Column::Id"
    )]
    Tenant,
    
    /// 多对一：步骤执行 -> Agent 执行（可选）
    #[sea_orm(
        belongs_to = "super::agent_execution::Entity",
        from = "Column::AgentExecutionId",
        to = "super::agent_execution::Column::Id"
    )]
    AgentExecution,
    
    /// 自关联：父步骤
    #[sea_orm(
        belongs_to = "Entity",
        from = "Column::ParentStepId",
        to = "Column::Id"
    )]
    ParentStep,
    
    /// 一对多：子步骤
    #[sea_orm(has_many = "Entity")]
    ChildSteps,
}

/// 实现与工作流执行的关联
impl Related<super::workflow_execution::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::WorkflowExecution.def()
    }
}

/// 实现与租户的关联
impl Related<super::tenant::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Tenant.def()
    }
}

/// 实现与 Agent 执行的关联
impl Related<super::agent_execution::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::AgentExecution.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

/// 步骤输入数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepInput {
    /// 输入参数
    pub parameters: std::collections::HashMap<String, serde_json::Value>,
    /// 来自前置步骤的数据
    pub previous_outputs: std::collections::HashMap<String, serde_json::Value>,
    /// 全局变量
    pub global_variables: std::collections::HashMap<String, serde_json::Value>,
    /// 文件引用
    pub file_references: Vec<String>,
}

/// 步骤输出数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepOutput {
    /// 输出结果
    pub results: std::collections::HashMap<String, serde_json::Value>,
    /// 生成的文件
    pub generated_files: Vec<String>,
    /// 状态信息
    pub status_info: StatusInfo,
    /// 下一步建议
    pub next_step_suggestions: Vec<String>,
}

/// 状态信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusInfo {
    /// 执行结果
    pub result: String,
    /// 置信度
    pub confidence: Option<f32>,
    /// 质量分数
    pub quality_score: Option<f32>,
    /// 附加信息
    pub additional_info: std::collections::HashMap<String, serde_json::Value>,
}

/// 步骤配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepConfig {
    /// 超时时间（秒）
    pub timeout_seconds: Option<u32>,
    /// 重试配置
    pub retry_config: StepRetryConfig,
    /// 条件表达式（用于条件步骤）
    pub condition_expr: Option<String>,
    /// 循环配置（用于循环步骤）
    pub loop_config: Option<LoopConfig>,
    /// 并行配置（用于并行步骤）
    pub parallel_config: Option<ParallelConfig>,
    /// Agent 配置（用于 Agent 步骤）
    pub agent_config: Option<AgentStepConfig>,
}

/// 步骤重试配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepRetryConfig {
    /// 最大重试次数
    pub max_retries: u32,
    /// 重试间隔（毫秒）
    pub retry_interval_ms: u64,
    /// 退避策略
    pub backoff_strategy: String, // "fixed", "linear", "exponential"
    /// 重试条件
    pub retry_conditions: Vec<String>,
}

/// 循环配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopConfig {
    /// 循环条件
    pub condition: String,
    /// 最大迭代次数
    pub max_iterations: u32,
    /// 循环变量
    pub loop_variable: String,
    /// 迭代数据
    pub iteration_data: Option<serde_json::Value>,
}

/// 并行配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelConfig {
    /// 并行分支
    pub branches: Vec<ParallelBranch>,
    /// 合并策略
    pub merge_strategy: String, // "all", "any", "first", "custom"
    /// 最大并发数
    pub max_concurrency: Option<u32>,
    /// 超时策略
    pub timeout_strategy: String, // "fail_fast", "wait_all", "partial"
}

/// 并行分支
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelBranch {
    /// 分支 ID
    pub branch_id: String,
    /// 分支名称
    pub branch_name: String,
    /// 分支步骤
    pub steps: Vec<String>,
    /// 分支权重
    pub weight: Option<f32>,
}

/// Agent 步骤配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStepConfig {
    /// Agent ID
    pub agent_id: Uuid,
    /// 输入映射
    pub input_mapping: std::collections::HashMap<String, String>,
    /// 输出映射
    pub output_mapping: std::collections::HashMap<String, String>,
    /// 覆盖配置
    pub override_config: Option<serde_json::Value>,
}

/// 步骤执行上下文
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepExecutionContext {
    /// 工作流上下文
    pub workflow_context: std::collections::HashMap<String, serde_json::Value>,
    /// 步骤局部变量
    pub local_variables: std::collections::HashMap<String, serde_json::Value>,
    /// 前置步骤输出
    pub previous_outputs: std::collections::HashMap<String, serde_json::Value>,
    /// 执行环境
    pub environment: std::collections::HashMap<String, String>,
}

/// 步骤执行指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepExecutionMetrics {
    /// 执行时间（毫秒）
    pub execution_time_ms: i64,
    /// 内存使用峰值（MB）
    pub peak_memory_mb: f64,
    /// CPU 使用时间（毫秒）
    pub cpu_time_ms: i64,
    /// API 调用次数
    pub api_calls: u32,
    /// 网络请求次数
    pub network_requests: u32,
    /// 缓存命中次数
    pub cache_hits: u32,
    /// 缓存未命中次数
    pub cache_misses: u32,
    /// 自定义指标
    pub custom_metrics: std::collections::HashMap<String, serde_json::Value>,
}

impl Default for StepInput {
    fn default() -> Self {
        Self {
            parameters: std::collections::HashMap::new(),
            previous_outputs: std::collections::HashMap::new(),
            global_variables: std::collections::HashMap::new(),
            file_references: Vec::new(),
        }
    }
}

impl Default for StepOutput {
    fn default() -> Self {
        Self {
            results: std::collections::HashMap::new(),
            generated_files: Vec::new(),
            status_info: StatusInfo::default(),
            next_step_suggestions: Vec::new(),
        }
    }
}

impl Default for StatusInfo {
    fn default() -> Self {
        Self {
            result: "success".to_string(),
            confidence: None,
            quality_score: None,
            additional_info: std::collections::HashMap::new(),
        }
    }
}

impl Default for StepConfig {
    fn default() -> Self {
        Self {
            timeout_seconds: Some(300), // 5 minutes
            retry_config: StepRetryConfig::default(),
            condition_expr: None,
            loop_config: None,
            parallel_config: None,
            agent_config: None,
        }
    }
}

impl Default for StepRetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            retry_interval_ms: 1000,
            backoff_strategy: "exponential".to_string(),
            retry_conditions: vec!["timeout".to_string(), "error".to_string()],
        }
    }
}

impl Default for StepExecutionContext {
    fn default() -> Self {
        Self {
            workflow_context: std::collections::HashMap::new(),
            local_variables: std::collections::HashMap::new(),
            previous_outputs: std::collections::HashMap::new(),
            environment: std::collections::HashMap::new(),
        }
    }
}

impl Default for StepExecutionMetrics {
    fn default() -> Self {
        Self {
            execution_time_ms: 0,
            peak_memory_mb: 0.0,
            cpu_time_ms: 0,
            api_calls: 0,
            network_requests: 0,
            cache_hits: 0,
            cache_misses: 0,
            custom_metrics: std::collections::HashMap::new(),
        }
    }
}

/// 步骤执行记录实用方法
impl Model {
    /// 检查步骤是否完成
    pub fn is_completed(&self) -> bool {
        matches!(
            self.status,
            StepExecutionStatus::Completed
                | StepExecutionStatus::Failed
                | StepExecutionStatus::Skipped
                | StepExecutionStatus::Cancelled
                | StepExecutionStatus::Timeout
        )
    }
    
    /// 检查步骤是否成功
    pub fn is_successful(&self) -> bool {
        matches!(
            self.status,
            StepExecutionStatus::Completed | StepExecutionStatus::Skipped
        )
    }
    
    /// 检查步骤是否失败
    pub fn is_failed(&self) -> bool {
        matches!(
            self.status,
            StepExecutionStatus::Failed | StepExecutionStatus::Timeout
        )
    }
    
    /// 检查步骤是否正在运行
    pub fn is_running(&self) -> bool {
        self.status == StepExecutionStatus::Running
    }
    
    /// 检查是否可以重试
    pub fn can_retry(&self) -> bool {
        self.is_failed() && self.retry_count < self.max_retries
    }
    
    /// 获取步骤输入
    pub fn get_input(&self) -> Result<StepInput, serde_json::Error> {
        serde_json::from_value(self.input.clone())
    }
    
    /// 获取步骤输出
    pub fn get_output(&self) -> Result<Option<StepOutput>, serde_json::Error> {
        if let Some(output) = &self.output {
            Ok(Some(serde_json::from_value(output.clone())?))
        } else {
            Ok(None)
        }
    }
    
    /// 获取步骤配置
    pub fn get_config(&self) -> Result<StepConfig, serde_json::Error> {
        serde_json::from_value(self.step_config.clone())
    }
    
    /// 获取执行上下文
    pub fn get_context(&self) -> Result<StepExecutionContext, serde_json::Error> {
        serde_json::from_value(self.context.clone())
    }
    
    /// 获取执行指标
    pub fn get_metrics(&self) -> Result<StepExecutionMetrics, serde_json::Error> {
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
    
    /// 获取步骤类型的显示名称
    pub fn type_display_name(&self) -> &'static str {
        match self.step_type {
            StepType::Agent => "Agent 步骤",
            StepType::Condition => "条件步骤",
            StepType::Loop => "循环步骤",
            StepType::Parallel => "并行步骤",
            StepType::Merge => "合并步骤",
            StepType::Input => "输入步骤",
            StepType::Output => "输出步骤",
            StepType::Delay => "延迟步骤",
            StepType::Custom => "自定义步骤",
        }
    }
    
    /// 获取状态的显示名称
    pub fn status_display_name(&self) -> &'static str {
        match self.status {
            StepExecutionStatus::Pending => "等待中",
            StepExecutionStatus::Running => "执行中",
            StepExecutionStatus::Completed => "已完成",
            StepExecutionStatus::Failed => "执行失败",
            StepExecutionStatus::Skipped => "已跳过",
            StepExecutionStatus::Cancelled => "已取消",
            StepExecutionStatus::Timeout => "执行超时",
        }
    }
    
    /// 检查是否超时
    pub fn is_timeout(&self) -> bool {
        if let Some(started) = self.started_at {
            if let Ok(config) = self.get_config() {
                if let Some(timeout) = config.timeout_seconds {
                    let now = chrono::Utc::now();
                    let started_utc = started.with_timezone(&chrono::Utc);
                    let elapsed = (now - started_utc).num_seconds() as u32;
                    return elapsed > timeout;
                }
            }
        }
        false
    }
    
    /// 获取执行进度描述
    pub fn progress_description(&self) -> String {
        match self.status {
            StepExecutionStatus::Pending => "等待执行".to_string(),
            StepExecutionStatus::Running => {
                if let Some(started) = self.started_at {
                    let now = chrono::Utc::now();
                    let started_utc = started.with_timezone(&chrono::Utc);
                    let elapsed = (now - started_utc).num_seconds();
                    format!("执行中 ({}秒)", elapsed)
                } else {
                    "执行中".to_string()
                }
            }
            StepExecutionStatus::Completed => {
                if let Some(duration) = self.duration_ms {
                    format!("已完成 ({}ms)", duration)
                } else {
                    "已完成".to_string()
                }
            }
            StepExecutionStatus::Failed => {
                if let Some(error) = &self.error_message {
                    format!("失败: {}", error)
                } else {
                    "执行失败".to_string()
                }
            }
            StepExecutionStatus::Skipped => "已跳过".to_string(),
            StepExecutionStatus::Cancelled => "已取消".to_string(),
            StepExecutionStatus::Timeout => "执行超时".to_string(),
        }
    }
}