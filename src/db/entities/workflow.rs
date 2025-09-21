// 工作流实体定义

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 工作流状态枚举
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "workflow_status")]
pub enum WorkflowStatus {
    #[sea_orm(string_value = "active")]
    Active,
    #[sea_orm(string_value = "inactive")]
    Inactive,
    #[sea_orm(string_value = "draft")]
    Draft,
    #[sea_orm(string_value = "archived")]
    Archived,
}

/// 工作流类型枚举
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "workflow_type")]
pub enum WorkflowType {
    #[sea_orm(string_value = "sequential")]
    Sequential,
    #[sea_orm(string_value = "parallel")]
    Parallel,
    #[sea_orm(string_value = "conditional")]
    Conditional,
    #[sea_orm(string_value = "loop")]
    Loop,
    #[sea_orm(string_value = "dag")]
    Dag, // Directed Acyclic Graph
}

/// 工作流实体
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "workflows")]
pub struct Model {
    /// 工作流 ID
    #[sea_orm(primary_key)]
    pub id: Uuid,
    
    /// 租户 ID
    pub tenant_id: Uuid,
    
    /// 工作流名称
    #[sea_orm(column_type = "String(Some(255))")]
    pub name: String,
    
    /// 工作流描述
    #[sea_orm(column_type = "Text", nullable)]
    pub description: Option<String>,
    
    /// 工作流类型
    pub workflow_type: WorkflowType,
    
    /// 工作流状态
    pub status: WorkflowStatus,
    
    /// 工作流版本
    #[sea_orm(column_type = "String(Some(50))")]
    pub version: String,
    
    /// 工作流定义（JSON 格式的 DAG）
    #[sea_orm(column_type = "Json")]
    pub definition: Json,
    
    /// 工作流配置（JSON 格式）
    #[sea_orm(column_type = "Json")]
    pub config: Json,
    
    /// 输入模式定义（JSON Schema）
    #[sea_orm(column_type = "Json")]
    pub input_schema: Json,
    
    /// 输出模式定义（JSON Schema）
    #[sea_orm(column_type = "Json")]
    pub output_schema: Json,
    
    /// 工作流元数据（JSON 格式）
    #[sea_orm(column_type = "Json")]
    pub metadata: Json,
    
    /// 执行统计（JSON 格式）
    #[sea_orm(column_type = "Json")]
    pub execution_stats: Json,
    
    /// 最后执行时间
    #[sea_orm(nullable)]
    pub last_executed_at: Option<DateTimeWithTimeZone>,
    
    /// 创建者用户 ID
    pub created_by: Uuid,
    
    /// 创建时间
    pub created_at: DateTimeWithTimeZone,
    
    /// 更新时间
    pub updated_at: DateTimeWithTimeZone,
}

/// 工作流关联关系
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 多对一：工作流 -> 租户
    #[sea_orm(
        belongs_to = "super::tenant::Entity",
        from = "Column::TenantId",
        to = "super::tenant::Column::Id"
    )]
    Tenant,
    
    /// 多对一：工作流 -> 创建者用户
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::CreatedBy",
        to = "super::user::Column::Id"
    )]
    Creator,
    
    /// 一对多：工作流 -> 工作流执行记录
    #[sea_orm(has_many = "super::workflow_execution::Entity")]
    WorkflowExecutions,
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
        Relation::Creator.def()
    }
}

/// 实现与工作流执行记录的关联
impl Related<super::workflow_execution::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::WorkflowExecutions.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

/// 工作流定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDefinition {
    /// 工作流节点
    pub nodes: Vec<WorkflowNode>,
    /// 工作流边（连接）
    pub edges: Vec<WorkflowEdge>,
    /// 入口节点 ID
    pub entry_node: String,
    /// 出口节点 ID 列表
    pub exit_nodes: Vec<String>,
    /// 全局变量
    pub global_variables: std::collections::HashMap<String, serde_json::Value>,
}

/// 工作流节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowNode {
    /// 节点 ID
    pub id: String,
    /// 节点名称
    pub name: String,
    /// 节点类型
    pub node_type: WorkflowNodeType,
    /// 节点配置
    pub config: WorkflowNodeConfig,
    /// 节点位置（用于 UI 显示）
    pub position: NodePosition,
    /// 节点元数据
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
}

/// 工作流节点类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WorkflowNodeType {
    /// Agent 节点
    Agent { agent_id: Uuid },
    /// 条件节点
    Condition { condition: String },
    /// 循环节点
    Loop { condition: String, max_iterations: u32 },
    /// 并行节点
    Parallel { branches: Vec<String> },
    /// 合并节点
    Merge { merge_strategy: String },
    /// 输入节点
    Input { schema: serde_json::Value },
    /// 输出节点
    Output { schema: serde_json::Value },
    /// 延迟节点
    Delay { duration_ms: u64 },
    /// 自定义节点
    Custom { handler: String },
}

/// 工作流节点配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowNodeConfig {
    /// 输入映射
    pub input_mapping: std::collections::HashMap<String, String>,
    /// 输出映射
    pub output_mapping: std::collections::HashMap<String, String>,
    /// 错误处理策略
    pub error_handling: ErrorHandlingStrategy,
    /// 重试配置
    pub retry_config: RetryConfig,
    /// 超时配置
    pub timeout_ms: Option<u64>,
    /// 条件表达式（用于条件节点）
    pub condition_expr: Option<String>,
}

/// 错误处理策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ErrorHandlingStrategy {
    /// 停止工作流
    Stop,
    /// 继续执行
    Continue,
    /// 重试
    Retry,
    /// 跳转到指定节点
    Goto { node_id: String },
}

/// 重试配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// 最大重试次数
    pub max_retries: u32,
    /// 重试间隔（毫秒）
    pub retry_interval_ms: u64,
    /// 退避策略
    pub backoff_strategy: BackoffStrategy,
}

/// 退避策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BackoffStrategy {
    /// 固定间隔
    Fixed,
    /// 线性增长
    Linear,
    /// 指数增长
    Exponential,
}

/// 节点位置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodePosition {
    pub x: f64,
    pub y: f64,
}

/// 工作流边
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowEdge {
    /// 边 ID
    pub id: String,
    /// 源节点 ID
    pub source: String,
    /// 目标节点 ID
    pub target: String,
    /// 边类型
    pub edge_type: WorkflowEdgeType,
    /// 条件表达式（用于条件边）
    pub condition: Option<String>,
    /// 边标签
    pub label: Option<String>,
}

/// 工作流边类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkflowEdgeType {
    /// 普通边
    Normal,
    /// 条件边
    Conditional,
    /// 错误边
    Error,
    /// 超时边
    Timeout,
}

/// 工作流配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowConfig {
    /// 执行配置
    pub execution_config: WorkflowExecutionConfig,
    /// 监控配置
    pub monitoring_config: MonitoringConfig,
    /// 安全配置
    pub security_config: WorkflowSecurityConfig,
    /// 性能配置
    pub performance_config: WorkflowPerformanceConfig,
}

/// 工作流执行配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowExecutionConfig {
    /// 最大执行时间（秒）
    pub max_execution_time: u32,
    /// 并发限制
    pub concurrency_limit: u32,
    /// 是否启用断点续传
    pub enable_checkpointing: bool,
    /// 检查点间隔（秒）
    pub checkpoint_interval: u32,
}

/// 监控配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    /// 是否启用详细日志
    pub enable_detailed_logging: bool,
    /// 是否启用指标收集
    pub enable_metrics: bool,
    /// 是否启用追踪
    pub enable_tracing: bool,
    /// 告警规则
    pub alert_rules: Vec<AlertRule>,
}

/// 告警规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRule {
    /// 规则名称
    pub name: String,
    /// 条件表达式
    pub condition: String,
    /// 告警级别
    pub severity: String,
    /// 通知方式
    pub notification: Vec<String>,
}

/// 工作流安全配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowSecurityConfig {
    /// 允许的 Agent 列表
    pub allowed_agents: Vec<Uuid>,
    /// 资源限制
    pub resource_limits: WorkflowResourceLimits,
    /// 数据访问控制
    pub data_access_control: DataAccessControl,
}

/// 工作流资源限制
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowResourceLimits {
    /// 最大内存使用（MB）
    pub max_memory_mb: u32,
    /// 最大 CPU 使用率
    pub max_cpu_percent: u32,
    /// 最大网络请求数
    pub max_network_requests: u32,
    /// 最大执行节点数
    pub max_nodes: u32,
}

/// 数据访问控制
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataAccessControl {
    /// 允许访问的知识库
    pub allowed_knowledge_bases: Vec<Uuid>,
    /// 允许的数据操作
    pub allowed_operations: Vec<String>,
    /// 数据脱敏规则
    pub data_masking_rules: Vec<String>,
}

/// 工作流性能配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowPerformanceConfig {
    /// 缓存策略
    pub cache_strategy: String,
    /// 缓存 TTL（秒）
    pub cache_ttl: u32,
    /// 预热策略
    pub warmup_strategy: Option<String>,
    /// 负载均衡策略
    pub load_balancing: String,
}

/// 工作流元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowMetadata {
    /// 标签
    pub tags: Vec<String>,
    /// 分类
    pub category: Option<String>,
    /// 作者信息
    pub author: Option<String>,
    /// 文档链接
    pub documentation_url: Option<String>,
    /// 示例输入
    pub example_inputs: Vec<serde_json::Value>,
    /// 示例输出
    pub example_outputs: Vec<serde_json::Value>,
    /// 自定义字段
    pub custom_fields: std::collections::HashMap<String, serde_json::Value>,
}

/// 工作流执行统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowExecutionStats {
    /// 总执行次数
    pub total_executions: u64,
    /// 成功执行次数
    pub successful_executions: u64,
    /// 失败执行次数
    pub failed_executions: u64,
    /// 平均执行时间（毫秒）
    pub avg_execution_time_ms: f64,
    /// 最后更新时间
    pub last_updated: DateTimeWithTimeZone,
}

impl Default for WorkflowConfig {
    fn default() -> Self {
        Self {
            execution_config: WorkflowExecutionConfig::default(),
            monitoring_config: MonitoringConfig::default(),
            security_config: WorkflowSecurityConfig::default(),
            performance_config: WorkflowPerformanceConfig::default(),
        }
    }
}

impl Default for WorkflowExecutionConfig {
    fn default() -> Self {
        Self {
            max_execution_time: 3600, // 1 hour
            concurrency_limit: 10,
            enable_checkpointing: true,
            checkpoint_interval: 300, // 5 minutes
        }
    }
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            enable_detailed_logging: true,
            enable_metrics: true,
            enable_tracing: true,
            alert_rules: Vec::new(),
        }
    }
}

impl Default for WorkflowSecurityConfig {
    fn default() -> Self {
        Self {
            allowed_agents: Vec::new(),
            resource_limits: WorkflowResourceLimits::default(),
            data_access_control: DataAccessControl::default(),
        }
    }
}

impl Default for WorkflowResourceLimits {
    fn default() -> Self {
        Self {
            max_memory_mb: 1024,
            max_cpu_percent: 80,
            max_network_requests: 1000,
            max_nodes: 100,
        }
    }
}

impl Default for DataAccessControl {
    fn default() -> Self {
        Self {
            allowed_knowledge_bases: Vec::new(),
            allowed_operations: vec!["read".to_string()],
            data_masking_rules: Vec::new(),
        }
    }
}

impl Default for WorkflowPerformanceConfig {
    fn default() -> Self {
        Self {
            cache_strategy: "lru".to_string(),
            cache_ttl: 3600,
            warmup_strategy: None,
            load_balancing: "round_robin".to_string(),
        }
    }
}

impl Default for WorkflowMetadata {
    fn default() -> Self {
        Self {
            tags: Vec::new(),
            category: None,
            author: None,
            documentation_url: None,
            example_inputs: Vec::new(),
            example_outputs: Vec::new(),
            custom_fields: std::collections::HashMap::new(),
        }
    }
}

impl Default for WorkflowExecutionStats {
    fn default() -> Self {
        Self {
            total_executions: 0,
            successful_executions: 0,
            failed_executions: 0,
            avg_execution_time_ms: 0.0,
            last_updated: chrono::Utc::now().into(),
        }
    }
}

/// 工作流实用方法
impl Model {
    /// 检查工作流是否活跃
    pub fn is_active(&self) -> bool {
        self.status == WorkflowStatus::Active
    }
    
    /// 检查工作流是否为草稿状态
    pub fn is_draft(&self) -> bool {
        self.status == WorkflowStatus::Draft
    }
    
    /// 获取工作流定义
    pub fn get_definition(&self) -> Result<WorkflowDefinition, serde_json::Error> {
        serde_json::from_value(self.definition.clone())
    }
    
    /// 获取工作流配置
    pub fn get_config(&self) -> Result<WorkflowConfig, serde_json::Error> {
        serde_json::from_value(self.config.clone())
    }
    
    /// 获取工作流元数据
    pub fn get_metadata(&self) -> Result<WorkflowMetadata, serde_json::Error> {
        serde_json::from_value(self.metadata.clone())
    }
    
    /// 获取执行统计
    pub fn get_execution_stats(&self) -> Result<WorkflowExecutionStats, serde_json::Error> {
        serde_json::from_value(self.execution_stats.clone())
    }
    
    /// 获取成功率
    pub fn success_rate(&self) -> Result<f64, serde_json::Error> {
        let stats = self.get_execution_stats()?;
        if stats.total_executions == 0 {
            Ok(0.0)
        } else {
            Ok(stats.successful_executions as f64 / stats.total_executions as f64)
        }
    }
    
    /// 验证工作流定义
    pub fn validate_definition(&self) -> Result<bool, String> {
        let definition = self.get_definition()
            .map_err(|e| format!("Failed to parse definition: {}", e))?;
        
        // 检查是否有入口节点
        if definition.entry_node.is_empty() {
            return Err("Missing entry node".to_string());
        }
        
        // 检查入口节点是否存在
        if !definition.nodes.iter().any(|n| n.id == definition.entry_node) {
            return Err("Entry node not found in nodes".to_string());
        }
        
        // 检查出口节点是否存在
        for exit_node in &definition.exit_nodes {
            if !definition.nodes.iter().any(|n| n.id == *exit_node) {
                return Err(format!("Exit node '{}' not found in nodes", exit_node));
            }
        }
        
        // 检查边的有效性
        for edge in &definition.edges {
            let source_exists = definition.nodes.iter().any(|n| n.id == edge.source);
            let target_exists = definition.nodes.iter().any(|n| n.id == edge.target);
            
            if !source_exists {
                return Err(format!("Source node '{}' not found", edge.source));
            }
            if !target_exists {
                return Err(format!("Target node '{}' not found", edge.target));
            }
        }
        
        Ok(true)
    }
    
    /// 获取工作流类型的显示名称
    pub fn type_display_name(&self) -> &'static str {
        match self.workflow_type {
            WorkflowType::Sequential => "顺序工作流",
            WorkflowType::Parallel => "并行工作流",
            WorkflowType::Conditional => "条件工作流",
            WorkflowType::Loop => "循环工作流",
            WorkflowType::Dag => "DAG 工作流",
        }
    }
    
    /// 获取状态的显示名称
    pub fn status_display_name(&self) -> &'static str {
        match self.status {
            WorkflowStatus::Active => "活跃",
            WorkflowStatus::Inactive => "非活跃",
            WorkflowStatus::Draft => "草稿",
            WorkflowStatus::Archived => "已归档",
        }
    }
}