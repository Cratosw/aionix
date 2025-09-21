// Agent 实体定义

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Agent 状态枚举
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "agent_status")]
pub enum AgentStatus {
    #[sea_orm(string_value = "active")]
    Active,
    #[sea_orm(string_value = "inactive")]
    Inactive,
    #[sea_orm(string_value = "draft")]
    Draft,
    #[sea_orm(string_value = "archived")]
    Archived,
}

/// Agent 类型枚举
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "agent_type")]
pub enum AgentType {
    #[sea_orm(string_value = "conversational")]
    Conversational,
    #[sea_orm(string_value = "task_executor")]
    TaskExecutor,
    #[sea_orm(string_value = "data_processor")]
    DataProcessor,
    #[sea_orm(string_value = "code_generator")]
    CodeGenerator,
    #[sea_orm(string_value = "analyst")]
    Analyst,
    #[sea_orm(string_value = "custom")]
    Custom,
}

/// Agent 实体
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "agents")]
pub struct Model {
    /// Agent ID
    #[sea_orm(primary_key)]
    pub id: Uuid,
    
    /// 租户 ID
    pub tenant_id: Uuid,
    
    /// Agent 名称
    #[sea_orm(column_type = "String(Some(255))")]
    pub name: String,
    
    /// Agent 描述
    #[sea_orm(column_type = "Text", nullable)]
    pub description: Option<String>,
    
    /// Agent 类型
    pub agent_type: AgentType,
    
    /// Agent 状态
    pub status: AgentStatus,
    
    /// Agent 版本
    #[sea_orm(column_type = "String(Some(50))")]
    pub version: String,
    
    /// Agent 配置（JSON 格式）
    #[sea_orm(column_type = "Json")]
    pub config: Json,
    
    /// 系统提示词
    #[sea_orm(column_type = "Text")]
    pub system_prompt: String,
    
    /// 可用工具列表（JSON 格式）
    #[sea_orm(column_type = "Json")]
    pub tools: Json,
    
    /// Agent 能力描述（JSON 格式）
    #[sea_orm(column_type = "Json")]
    pub capabilities: Json,
    
    /// Agent 元数据（JSON 格式）
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

/// Agent 关联关系
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 多对一：Agent -> 租户
    #[sea_orm(
        belongs_to = "super::tenant::Entity",
        from = "Column::TenantId",
        to = "super::tenant::Column::Id"
    )]
    Tenant,
    
    /// 多对一：Agent -> 创建者用户
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::CreatedBy",
        to = "super::user::Column::Id"
    )]
    Creator,
    
    /// 一对多：Agent -> Agent 执行记录
    #[sea_orm(has_many = "super::agent_execution::Entity")]
    AgentExecutions,
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

/// 实现与 Agent 执行记录的关联
impl Related<super::agent_execution::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::AgentExecutions.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

/// Agent 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// LLM 配置
    pub llm_config: LlmConfig,
    /// 执行配置
    pub execution_config: ExecutionConfig,
    /// 安全配置
    pub security_config: SecurityConfig,
    /// 性能配置
    pub performance_config: PerformanceConfig,
    /// 自定义配置
    pub custom_config: serde_json::Value,
}

/// LLM 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    /// 模型名称
    pub model_name: String,
    /// 温度参数
    pub temperature: f32,
    /// 最大 token 数
    pub max_tokens: u32,
    /// Top-p 参数
    pub top_p: Option<f32>,
    /// 频率惩罚
    pub frequency_penalty: Option<f32>,
    /// 存在惩罚
    pub presence_penalty: Option<f32>,
    /// 停止词
    pub stop_sequences: Vec<String>,
}

/// 执行配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionConfig {
    /// 最大执行时间（秒）
    pub max_execution_time: u32,
    /// 最大迭代次数
    pub max_iterations: u32,
    /// 是否启用并行执行
    pub enable_parallel: bool,
    /// 重试次数
    pub retry_count: u32,
    /// 超时处理策略
    pub timeout_strategy: String,
}

/// 安全配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// 允许的工具列表
    pub allowed_tools: Vec<String>,
    /// 禁止的工具列表
    pub blocked_tools: Vec<String>,
    /// 资源访问限制
    pub resource_limits: ResourceLimits,
    /// 内容过滤规则
    pub content_filters: Vec<String>,
}

/// 资源限制
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// 最大内存使用（MB）
    pub max_memory_mb: u32,
    /// 最大 CPU 使用率
    pub max_cpu_percent: u32,
    /// 最大网络请求数
    pub max_network_requests: u32,
    /// 最大文件大小（MB）
    pub max_file_size_mb: u32,
}

/// 性能配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// 缓存策略
    pub cache_strategy: String,
    /// 缓存 TTL（秒）
    pub cache_ttl: u32,
    /// 批处理大小
    pub batch_size: u32,
    /// 预热策略
    pub warmup_strategy: Option<String>,
}

/// Agent 工具定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTool {
    /// 工具名称
    pub name: String,
    /// 工具描述
    pub description: String,
    /// 工具类型
    pub tool_type: String,
    /// 工具配置
    pub config: serde_json::Value,
    /// 是否启用
    pub enabled: bool,
    /// 权限要求
    pub permissions: Vec<String>,
}

/// Agent 能力
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCapabilities {
    /// 支持的语言
    pub languages: Vec<String>,
    /// 支持的任务类型
    pub task_types: Vec<String>,
    /// 输入格式
    pub input_formats: Vec<String>,
    /// 输出格式
    pub output_formats: Vec<String>,
    /// 特殊能力
    pub special_abilities: Vec<String>,
}

/// Agent 元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMetadata {
    /// 标签
    pub tags: Vec<String>,
    /// 分类
    pub category: Option<String>,
    /// 作者信息
    pub author: Option<String>,
    /// 许可证
    pub license: Option<String>,
    /// 文档链接
    pub documentation_url: Option<String>,
    /// 示例用法
    pub examples: Vec<AgentExample>,
    /// 自定义字段
    pub custom_fields: std::collections::HashMap<String, serde_json::Value>,
}

/// Agent 示例
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentExample {
    /// 示例名称
    pub name: String,
    /// 示例描述
    pub description: String,
    /// 输入示例
    pub input: serde_json::Value,
    /// 输出示例
    pub output: serde_json::Value,
}

/// Agent 执行统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentExecutionStats {
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

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            llm_config: LlmConfig::default(),
            execution_config: ExecutionConfig::default(),
            security_config: SecurityConfig::default(),
            performance_config: PerformanceConfig::default(),
            custom_config: serde_json::Value::Object(serde_json::Map::new()),
        }
    }
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            model_name: "gpt-3.5-turbo".to_string(),
            temperature: 0.7,
            max_tokens: 2048,
            top_p: Some(1.0),
            frequency_penalty: Some(0.0),
            presence_penalty: Some(0.0),
            stop_sequences: Vec::new(),
        }
    }
}

impl Default for ExecutionConfig {
    fn default() -> Self {
        Self {
            max_execution_time: 300, // 5 minutes
            max_iterations: 10,
            enable_parallel: false,
            retry_count: 3,
            timeout_strategy: "graceful".to_string(),
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            allowed_tools: vec!["search".to_string(), "calculator".to_string()],
            blocked_tools: Vec::new(),
            resource_limits: ResourceLimits::default(),
            content_filters: vec!["profanity".to_string(), "sensitive_data".to_string()],
        }
    }
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_memory_mb: 512,
            max_cpu_percent: 80,
            max_network_requests: 100,
            max_file_size_mb: 10,
        }
    }
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            cache_strategy: "lru".to_string(),
            cache_ttl: 3600, // 1 hour
            batch_size: 10,
            warmup_strategy: None,
        }
    }
}

impl Default for AgentCapabilities {
    fn default() -> Self {
        Self {
            languages: vec!["zh-CN".to_string(), "en-US".to_string()],
            task_types: vec!["conversation".to_string(), "analysis".to_string()],
            input_formats: vec!["text".to_string(), "json".to_string()],
            output_formats: vec!["text".to_string(), "json".to_string()],
            special_abilities: Vec::new(),
        }
    }
}

impl Default for AgentMetadata {
    fn default() -> Self {
        Self {
            tags: Vec::new(),
            category: None,
            author: None,
            license: None,
            documentation_url: None,
            examples: Vec::new(),
            custom_fields: std::collections::HashMap::new(),
        }
    }
}

impl Default for AgentExecutionStats {
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

/// Agent 实用方法
impl Model {
    /// 检查 Agent 是否活跃
    pub fn is_active(&self) -> bool {
        self.status == AgentStatus::Active
    }
    
    /// 检查 Agent 是否为草稿状态
    pub fn is_draft(&self) -> bool {
        self.status == AgentStatus::Draft
    }
    
    /// 检查 Agent 是否已归档
    pub fn is_archived(&self) -> bool {
        self.status == AgentStatus::Archived
    }
    
    /// 获取 Agent 配置
    pub fn get_config(&self) -> Result<AgentConfig, serde_json::Error> {
        serde_json::from_value(self.config.clone())
    }
    
    /// 获取 Agent 工具列表
    pub fn get_tools(&self) -> Result<Vec<AgentTool>, serde_json::Error> {
        serde_json::from_value(self.tools.clone())
    }
    
    /// 获取 Agent 能力
    pub fn get_capabilities(&self) -> Result<AgentCapabilities, serde_json::Error> {
        serde_json::from_value(self.capabilities.clone())
    }
    
    /// 获取 Agent 元数据
    pub fn get_metadata(&self) -> Result<AgentMetadata, serde_json::Error> {
        serde_json::from_value(self.metadata.clone())
    }
    
    /// 获取执行统计
    pub fn get_execution_stats(&self) -> Result<AgentExecutionStats, serde_json::Error> {
        serde_json::from_value(self.execution_stats.clone())
    }
    
    /// 检查是否支持特定工具
    pub fn supports_tool(&self, tool_name: &str) -> Result<bool, serde_json::Error> {
        let tools = self.get_tools()?;
        Ok(tools.iter().any(|tool| tool.name == tool_name && tool.enabled))
    }
    
    /// 检查是否支持特定任务类型
    pub fn supports_task_type(&self, task_type: &str) -> Result<bool, serde_json::Error> {
        let capabilities = self.get_capabilities()?;
        Ok(capabilities.task_types.contains(&task_type.to_string()))
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
    
    /// 检查是否需要更新
    pub fn needs_update(&self) -> bool {
        if let Some(last_executed) = self.last_executed_at {
            let now = chrono::Utc::now();
            let last_executed_utc = last_executed.with_timezone(&chrono::Utc);
            // 如果超过7天未执行，可能需要更新
            (now - last_executed_utc).num_days() > 7
        } else {
            false
        }
    }
    
    /// 获取 Agent 类型的显示名称
    pub fn type_display_name(&self) -> &'static str {
        match self.agent_type {
            AgentType::Conversational => "对话型 Agent",
            AgentType::TaskExecutor => "任务执行 Agent",
            AgentType::DataProcessor => "数据处理 Agent",
            AgentType::CodeGenerator => "代码生成 Agent",
            AgentType::Analyst => "分析型 Agent",
            AgentType::Custom => "自定义 Agent",
        }
    }
    
    /// 获取状态的显示名称
    pub fn status_display_name(&self) -> &'static str {
        match self.status {
            AgentStatus::Active => "活跃",
            AgentStatus::Inactive => "非活跃",
            AgentStatus::Draft => "草稿",
            AgentStatus::Archived => "已归档",
        }
    }
}