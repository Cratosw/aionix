// 工作流引擎
// 实现 DAG 工作流定义、解析和验证

use std::sync::Arc;
use std::collections::{HashMap, HashSet, VecDeque};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{info, warn, error, debug};
use tokio::sync::RwLock;

use crate::errors::AiStudioError;

/// 工作流引擎
pub struct WorkflowEngine {
    /// 已注册的工作流
    workflows: Arc<RwLock<HashMap<Uuid, WorkflowDefinition>>>,
    /// 工作流模板
    templates: Arc<RwLock<HashMap<String, WorkflowTemplate>>>,
    /// 引擎配置
    config: WorkflowEngineConfig,
}

/// 工作流引擎配置
#[derive(Debug, Clone)]
pub struct WorkflowEngineConfig {
    /// 最大工作流步骤数
    pub max_steps: usize,
    /// 最大依赖深度
    pub max_dependency_depth: usize,
    /// 是否启用循环检测
    pub enable_cycle_detection: bool,
    /// 默认超时时间（秒）
    pub default_timeout_seconds: u64,
}

impl Default for WorkflowEngineConfig {
    fn default() -> Self {
        Self {
            max_steps: 1000,
            max_dependency_depth: 50,
            enable_cycle_detection: true,
            default_timeout_seconds: 3600, // 1小时
        }
    }
}

/// 工作流定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDefinition {
    /// 工作流 ID
    pub id: Uuid,
    /// 工作流名称
    pub name: String,
    /// 工作流描述
    pub description: String,
    /// 工作流版本
    pub version: String,
    /// 创建者
    pub created_by: Uuid,
    /// 租户 ID
    pub tenant_id: Uuid,
    /// 工作流步骤
    pub steps: Vec<WorkflowStep>,
    /// 工作流参数
    pub parameters: Vec<WorkflowParameter>,
    /// 工作流输出
    pub outputs: Vec<WorkflowOutput>,
    /// 工作流配置
    pub config: WorkflowConfig,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 更新时间
    pub updated_at: DateTime<Utc>,
    /// 工作流状态
    pub status: WorkflowStatus,
}

/// 工作流步骤
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    /// 步骤 ID
    pub id: String,
    /// 步骤名称
    pub name: String,
    /// 步骤描述
    pub description: String,
    /// 步骤类型
    pub step_type: StepType,
    /// 步骤配置
    pub config: StepConfig,
    /// 依赖步骤
    pub depends_on: Vec<String>,
    /// 条件表达式
    pub condition: Option<String>,
    /// 重试配置
    pub retry_config: Option<RetryConfig>,
    /// 超时配置
    pub timeout_seconds: Option<u64>,
    /// 步骤位置（用于可视化）
    pub position: Option<StepPosition>,
}

/// 步骤类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum StepType {
    /// Agent 任务
    AgentTask,
    /// 工具调用
    ToolCall,
    /// 条件分支
    Condition,
    /// 并行执行
    Parallel,
    /// 循环执行
    Loop,
    /// 等待
    Wait,
    /// 人工审批
    HumanApproval,
    /// 数据转换
    DataTransform,
    /// 外部 API 调用
    ApiCall,
    /// 子工作流
    SubWorkflow,
}

/// 步骤配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StepConfig {
    /// Agent 任务配置
    AgentTask {
        /// Agent ID 或配置
        agent: AgentReference,
        /// 任务描述
        task_description: String,
        /// 任务参数
        parameters: HashMap<String, serde_json::Value>,
    },
    /// 工具调用配置
    ToolCall {
        /// 工具名称
        tool_name: String,
        /// 工具参数
        parameters: HashMap<String, serde_json::Value>,
    },
    /// 条件分支配置
    Condition {
        /// 条件表达式
        expression: String,
        /// 真分支步骤
        true_steps: Vec<String>,
        /// 假分支步骤
        false_steps: Vec<String>,
    },
    /// 并行执行配置
    Parallel {
        /// 并行步骤组
        step_groups: Vec<Vec<String>>,
        /// 是否等待所有完成
        wait_for_all: bool,
    },
    /// 循环执行配置
    Loop {
        /// 循环条件
        condition: String,
        /// 循环步骤
        steps: Vec<String>,
        /// 最大迭代次数
        max_iterations: Option<u32>,
    },
    /// 等待配置
    Wait {
        /// 等待时间（秒）
        duration_seconds: u64,
        /// 等待条件
        condition: Option<String>,
    },
    /// 人工审批配置
    HumanApproval {
        /// 审批者
        approvers: Vec<Uuid>,
        /// 审批描述
        description: String,
        /// 是否需要所有人审批
        require_all: bool,
    },
    /// 数据转换配置
    DataTransform {
        /// 转换脚本
        script: String,
        /// 脚本语言
        language: ScriptLanguage,
        /// 输入映射
        input_mapping: HashMap<String, String>,
        /// 输出映射
        output_mapping: HashMap<String, String>,
    },
    /// API 调用配置
    ApiCall {
        /// API URL
        url: String,
        /// HTTP 方法
        method: String,
        /// 请求头
        headers: HashMap<String, String>,
        /// 请求体
        body: Option<serde_json::Value>,
    },
    /// 子工作流配置
    SubWorkflow {
        /// 子工作流 ID
        workflow_id: Uuid,
        /// 参数映射
        parameter_mapping: HashMap<String, String>,
    },
}

/// Agent 引用
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentReference {
    /// 现有 Agent ID
    ExistingAgent { agent_id: Uuid },
    /// 内联 Agent 配置
    InlineAgent { config: crate::ai::agent_runtime::AgentConfig },
}

/// 脚本语言
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ScriptLanguage {
    JavaScript,
    Python,
    Lua,
    JsonPath,
}

/// 重试配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// 最大重试次数
    pub max_attempts: u32,
    /// 重试间隔（秒）
    pub interval_seconds: u64,
    /// 退避策略
    pub backoff_strategy: BackoffStrategy,
    /// 重试条件
    pub retry_on: Vec<RetryCondition>,
}

/// 退避策略
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum BackoffStrategy {
    /// 固定间隔
    Fixed,
    /// 线性退避
    Linear,
    /// 指数退避
    Exponential,
}

/// 重试条件
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RetryCondition {
    /// 任何错误
    AnyError,
    /// 超时错误
    Timeout,
    /// 网络错误
    NetworkError,
    /// 特定错误码
    ErrorCode(String),
}

/// 步骤位置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepPosition {
    /// X 坐标
    pub x: f32,
    /// Y 坐标
    pub y: f32,
}

/// 工作流参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowParameter {
    /// 参数名称
    pub name: String,
    /// 参数类型
    pub parameter_type: ParameterType,
    /// 参数描述
    pub description: String,
    /// 是否必需
    pub required: bool,
    /// 默认值
    pub default_value: Option<serde_json::Value>,
    /// 参数验证
    pub validation: Option<ParameterValidation>,
}

/// 参数类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ParameterType {
    String,
    Number,
    Boolean,
    Array,
    Object,
    File,
}

/// 参数验证
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterValidation {
    /// 最小值/长度
    pub min: Option<f64>,
    /// 最大值/长度
    pub max: Option<f64>,
    /// 正则表达式
    pub pattern: Option<String>,
    /// 枚举值
    pub enum_values: Option<Vec<serde_json::Value>>,
}

/// 工作流输出
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowOutput {
    /// 输出名称
    pub name: String,
    /// 输出类型
    pub output_type: ParameterType,
    /// 输出描述
    pub description: String,
    /// 输出来源步骤
    pub source_step: String,
    /// 输出路径
    pub source_path: String,
}

/// 工作流配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowConfig {
    /// 并发限制
    pub max_concurrent_steps: Option<u32>,
    /// 总超时时间（秒）
    pub total_timeout_seconds: Option<u64>,
    /// 错误处理策略
    pub error_handling: ErrorHandlingStrategy,
    /// 是否启用日志记录
    pub enable_logging: bool,
    /// 是否启用监控
    pub enable_monitoring: bool,
}

impl Default for WorkflowConfig {
    fn default() -> Self {
        Self {
            max_concurrent_steps: Some(10),
            total_timeout_seconds: Some(3600),
            error_handling: ErrorHandlingStrategy::StopOnError,
            enable_logging: true,
            enable_monitoring: true,
        }
    }
}

/// 错误处理策略
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ErrorHandlingStrategy {
    /// 遇到错误停止
    StopOnError,
    /// 继续执行其他步骤
    ContinueOnError,
    /// 跳过失败步骤
    SkipOnError,
    /// 自定义处理
    Custom(String),
}

/// 工作流状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowStatus {
    /// 草稿
    Draft,
    /// 已发布
    Published,
    /// 已弃用
    Deprecated,
    /// 已删除
    Deleted,
}

/// 工作流模板
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowTemplate {
    /// 模板名称
    pub name: String,
    /// 模板描述
    pub description: String,
    /// 模板类别
    pub category: String,
    /// 模板标签
    pub tags: Vec<String>,
    /// 工作流定义
    pub workflow: WorkflowDefinition,
    /// 创建时间
    pub created_at: DateTime<Utc>,
}

/// 工作流验证结果
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// 是否有效
    pub is_valid: bool,
    /// 错误列表
    pub errors: Vec<ValidationError>,
    /// 警告列表
    pub warnings: Vec<ValidationWarning>,
    /// 依赖图
    pub dependency_graph: DependencyGraph,
}

/// 验证错误
#[derive(Debug, Clone)]
pub struct ValidationError {
    /// 错误类型
    pub error_type: ValidationErrorType,
    /// 错误消息
    pub message: String,
    /// 相关步骤
    pub step_id: Option<String>,
}

/// 验证错误类型
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationErrorType {
    /// 循环依赖
    CircularDependency,
    /// 缺少依赖
    MissingDependency,
    /// 无效步骤配置
    InvalidStepConfig,
    /// 参数验证失败
    ParameterValidation,
    /// 超出限制
    ExceedsLimits,
}

/// 验证警告
#[derive(Debug, Clone)]
pub struct ValidationWarning {
    /// 警告类型
    pub warning_type: ValidationWarningType,
    /// 警告消息
    pub message: String,
    /// 相关步骤
    pub step_id: Option<String>,
}

/// 验证警告类型
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationWarningType {
    /// 未使用的步骤
    UnusedStep,
    /// 性能问题
    PerformanceIssue,
    /// 最佳实践建议
    BestPractice,
}

/// 依赖图
#[derive(Debug, Clone)]
pub struct DependencyGraph {
    /// 节点（步骤）
    pub nodes: HashSet<String>,
    /// 边（依赖关系）
    pub edges: Vec<(String, String)>,
    /// 拓扑排序结果
    pub topological_order: Vec<String>,
}

impl WorkflowEngine {
    /// 创建新的工作流引擎
    pub fn new(config: Option<WorkflowEngineConfig>) -> Self {
        Self {
            workflows: Arc::new(RwLock::new(HashMap::new())),
            templates: Arc::new(RwLock::new(HashMap::new())),
            config: config.unwrap_or_default(),
        }
    }
    
    /// 解析工作流定义
    pub async fn parse_workflow(
        &self,
        workflow_json: &str,
    ) -> Result<WorkflowDefinition, AiStudioError> {
        debug!("解析工作流定义");
        
        // 解析 JSON
        let workflow: WorkflowDefinition = serde_json::from_str(workflow_json)
            .map_err(|e| AiStudioError::validation("workflow_json".to_string(), format!("工作流 JSON 解析失败: {}", e)))?;
        
        // 验证工作流
        let validation_result = self.validate_workflow(&workflow).await?;
        
        if !validation_result.is_valid {
            let error_messages: Vec<String> = validation_result.errors
                .iter()
                .map(|e| e.message.clone())
                .collect();
            return Err(AiStudioError::validation("workflow".to_string(), format!(
                "工作流验证失败: {}",
                error_messages.join(", ")
            )));
        }
        
        info!("工作流解析成功: {}", workflow.name);
        Ok(workflow)
    }
    
    /// 验证工作流定义
    pub async fn validate_workflow(
        &self,
        workflow: &WorkflowDefinition,
    ) -> Result<ValidationResult, AiStudioError> {
        debug!("验证工作流: {}", workflow.name);
        
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        
        // 1. 基本验证
        self.validate_basic_constraints(workflow, &mut errors);
        
        // 2. 构建依赖图
        let dependency_graph = match self.build_dependency_graph(workflow) {
            Ok(graph) => graph,
            Err(e) => {
                errors.push(ValidationError {
                    error_type: ValidationErrorType::InvalidStepConfig,
                    message: format!("构建依赖图失败: {}", e),
                    step_id: None,
                });
                return Ok(ValidationResult {
                    is_valid: false,
                    errors,
                    warnings,
                    dependency_graph: DependencyGraph {
                        nodes: HashSet::new(),
                        edges: Vec::new(),
                        topological_order: Vec::new(),
                    },
                });
            }
        };
        
        // 3. 检查循环依赖
        if self.config.enable_cycle_detection {
            self.detect_cycles(&dependency_graph, &mut errors);
        }
        
        // 4. 验证步骤配置
        self.validate_step_configs(workflow, &mut errors, &mut warnings);
        
        // 5. 验证参数
        self.validate_parameters(workflow, &mut errors);
        
        // 6. 检查性能问题
        self.check_performance_issues(workflow, &mut warnings);
        
        let is_valid = errors.is_empty();
        
        info!("工作流验证完成: {} - 有效: {}, 错误: {}, 警告: {}", 
              workflow.name, is_valid, errors.len(), warnings.len());
        
        Ok(ValidationResult {
            is_valid,
            errors,
            warnings,
            dependency_graph,
        })
    }
    
    /// 注册工作流
    pub async fn register_workflow(
        &self,
        workflow: WorkflowDefinition,
    ) -> Result<(), AiStudioError> {
        info!("注册工作流: {} ({})", workflow.name, workflow.id);
        
        // 验证工作流
        let validation_result = self.validate_workflow(&workflow).await?;
        if !validation_result.is_valid {
            return Err(AiStudioError::validation("workflow".to_string(), "工作流验证失败".to_string()));
        }
        
        // 注册工作流
        let mut workflows = self.workflows.write().await;
        workflows.insert(workflow.id, workflow);
        
        Ok(())
    }
    
    /// 获取工作流定义
    pub async fn get_workflow(&self, workflow_id: Uuid) -> Result<WorkflowDefinition, AiStudioError> {
        let workflows = self.workflows.read().await;
        workflows.get(&workflow_id)
            .cloned()
            .ok_or_else(|| AiStudioError::not_found("工作流不存在"))
    }
    
    /// 列出工作流
    pub async fn list_workflows(&self, tenant_id: Option<Uuid>) -> Result<Vec<WorkflowDefinition>, AiStudioError> {
        let workflows = self.workflows.read().await;
        let mut result = Vec::new();
        
        for workflow in workflows.values() {
            if let Some(tid) = tenant_id {
                if workflow.tenant_id == tid {
                    result.push(workflow.clone());
                }
            } else {
                result.push(workflow.clone());
            }
        }
        
        Ok(result)
    }
    
    /// 注册工作流模板
    pub async fn register_template(&self, template: WorkflowTemplate) -> Result<(), AiStudioError> {
        info!("注册工作流模板: {}", template.name);
        
        let mut templates = self.templates.write().await;
        templates.insert(template.name.clone(), template);
        
        Ok(())
    }
    
    /// 从模板创建工作流
    pub async fn create_from_template(
        &self,
        template_name: &str,
        name: String,
        tenant_id: Uuid,
        created_by: Uuid,
        parameters: Option<HashMap<String, serde_json::Value>>,
    ) -> Result<WorkflowDefinition, AiStudioError> {
        debug!("从模板创建工作流: {} -> {}", template_name, name);
        
        let template = {
            let templates = self.templates.read().await;
            templates.get(template_name)
                .cloned()
                .ok_or_else(|| AiStudioError::not_found("工作流模板不存在"))?
        };
        
        let mut workflow = template.workflow;
        workflow.id = Uuid::new_v4();
        workflow.name = name;
        workflow.tenant_id = tenant_id;
        workflow.created_by = created_by;
        workflow.created_at = Utc::now();
        workflow.updated_at = Utc::now();
        workflow.status = WorkflowStatus::Draft;
        
        // 应用参数
        if let Some(params) = parameters {
            // TODO: 实现参数替换逻辑
        }
        
        Ok(workflow)
    }
    
    /// 基本约束验证
    fn validate_basic_constraints(&self, workflow: &WorkflowDefinition, errors: &mut Vec<ValidationError>) {
        // 检查步骤数量限制
        if workflow.steps.len() > self.config.max_steps {
            errors.push(ValidationError {
                error_type: ValidationErrorType::ExceedsLimits,
                message: format!("步骤数量 {} 超过限制 {}", workflow.steps.len(), self.config.max_steps),
                step_id: None,
            });
        }
        
        // 检查步骤 ID 唯一性
        let mut step_ids = HashSet::new();
        for step in &workflow.steps {
            if !step_ids.insert(&step.id) {
                errors.push(ValidationError {
                    error_type: ValidationErrorType::InvalidStepConfig,
                    message: format!("重复的步骤 ID: {}", step.id),
                    step_id: Some(step.id.clone()),
                });
            }
        }
        
        // 检查工作流名称
        if workflow.name.is_empty() {
            errors.push(ValidationError {
                error_type: ValidationErrorType::InvalidStepConfig,
                message: "工作流名称不能为空".to_string(),
                step_id: None,
            });
        }
    }
    
    /// 构建依赖图
    fn build_dependency_graph(&self, workflow: &WorkflowDefinition) -> Result<DependencyGraph, AiStudioError> {
        let mut nodes = HashSet::new();
        let mut edges = Vec::new();
        
        // 收集所有步骤节点
        for step in &workflow.steps {
            nodes.insert(step.id.clone());
        }
        
        // 构建依赖边
        for step in &workflow.steps {
            for dep in &step.depends_on {
                if !nodes.contains(dep) {
                    return Err(AiStudioError::validation(format!(
                        "步骤 {} 依赖的步骤 {} 不存在", step.id, dep
                    )));
                }
                edges.push((dep.clone(), step.id.clone()));
            }
        }
        
        // 拓扑排序
        let topological_order = self.topological_sort(&nodes, &edges)?;
        
        Ok(DependencyGraph {
            nodes,
            edges,
            topological_order,
        })
    }
    
    /// 拓扑排序
    fn topological_sort(&self, nodes: &HashSet<String>, edges: &[(String, String)]) -> Result<Vec<String>, AiStudioError> {
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        let mut adj_list: HashMap<String, Vec<String>> = HashMap::new();
        
        // 初始化
        for node in nodes {
            in_degree.insert(node.clone(), 0);
            adj_list.insert(node.clone(), Vec::new());
        }
        
        // 构建邻接表和入度
        for (from, to) in edges {
            adj_list.get_mut(from).unwrap().push(to.clone());
            *in_degree.get_mut(to).unwrap() += 1;
        }
        
        // Kahn 算法
        let mut queue = VecDeque::new();
        let mut result = Vec::new();
        
        // 找到所有入度为 0 的节点
        for (node, &degree) in &in_degree {
            if degree == 0 {
                queue.push_back(node.clone());
            }
        }
        
        while let Some(node) = queue.pop_front() {
            result.push(node.clone());
            
            // 更新邻接节点的入度
            for neighbor in &adj_list[&node] {
                let degree = in_degree.get_mut(neighbor).unwrap();
                *degree -= 1;
                if *degree == 0 {
                    queue.push_back(neighbor.clone());
                }
            }
        }
        
        // 检查是否存在循环
        if result.len() != nodes.len() {
            return Err(AiStudioError::validation("检测到循环依赖"));
        }
        
        Ok(result)
    }
    
    /// 检测循环依赖
    fn detect_cycles(&self, graph: &DependencyGraph, errors: &mut Vec<ValidationError>) {
        // 拓扑排序已经检测了循环，这里可以提供更详细的循环信息
        if graph.topological_order.len() != graph.nodes.len() {
            errors.push(ValidationError {
                error_type: ValidationErrorType::CircularDependency,
                message: "检测到循环依赖".to_string(),
                step_id: None,
            });
        }
    }
    
    /// 验证步骤配置
    fn validate_step_configs(
        &self,
        workflow: &WorkflowDefinition,
        errors: &mut Vec<ValidationError>,
        warnings: &mut Vec<ValidationWarning>,
    ) {
        for step in &workflow.steps {
            match &step.step_type {
                StepType::AgentTask => {
                    if let StepConfig::AgentTask { agent, task_description, .. } = &step.config {
                        if task_description.is_empty() {
                            errors.push(ValidationError {
                                error_type: ValidationErrorType::InvalidStepConfig,
                                message: "Agent 任务描述不能为空".to_string(),
                                step_id: Some(step.id.clone()),
                            });
                        }
                    } else {
                        errors.push(ValidationError {
                            error_type: ValidationErrorType::InvalidStepConfig,
                            message: "Agent 任务步骤配置类型不匹配".to_string(),
                            step_id: Some(step.id.clone()),
                        });
                    }
                }
                StepType::ToolCall => {
                    if let StepConfig::ToolCall { tool_name, .. } = &step.config {
                        if tool_name.is_empty() {
                            errors.push(ValidationError {
                                error_type: ValidationErrorType::InvalidStepConfig,
                                message: "工具名称不能为空".to_string(),
                                step_id: Some(step.id.clone()),
                            });
                        }
                    } else {
                        errors.push(ValidationError {
                            error_type: ValidationErrorType::InvalidStepConfig,
                            message: "工具调用步骤配置类型不匹配".to_string(),
                            step_id: Some(step.id.clone()),
                        });
                    }
                }
                _ => {
                    // TODO: 验证其他步骤类型
                }
            }
        }
    }
    
    /// 验证参数
    fn validate_parameters(&self, workflow: &WorkflowDefinition, errors: &mut Vec<ValidationError>) {
        for param in &workflow.parameters {
            if param.name.is_empty() {
                errors.push(ValidationError {
                    error_type: ValidationErrorType::ParameterValidation,
                    message: "参数名称不能为空".to_string(),
                    step_id: None,
                });
            }
            
            // 验证默认值类型
            if let Some(ref default_value) = param.default_value {
                if !self.validate_parameter_type(default_value, &param.parameter_type) {
                    errors.push(ValidationError {
                        error_type: ValidationErrorType::ParameterValidation,
                        message: format!("参数 {} 的默认值类型不匹配", param.name),
                        step_id: None,
                    });
                }
            }
        }
    }
    
    /// 验证参数类型
    fn validate_parameter_type(&self, value: &serde_json::Value, param_type: &ParameterType) -> bool {
        match (value, param_type) {
            (serde_json::Value::String(_), ParameterType::String) => true,
            (serde_json::Value::Number(_), ParameterType::Number) => true,
            (serde_json::Value::Bool(_), ParameterType::Boolean) => true,
            (serde_json::Value::Array(_), ParameterType::Array) => true,
            (serde_json::Value::Object(_), ParameterType::Object) => true,
            _ => false,
        }
    }
    
    /// 检查性能问题
    fn check_performance_issues(&self, workflow: &WorkflowDefinition, warnings: &mut Vec<ValidationWarning>) {
        // 检查步骤数量
        if workflow.steps.len() > 100 {
            warnings.push(ValidationWarning {
                warning_type: ValidationWarningType::PerformanceIssue,
                message: format!("工作流包含 {} 个步骤，可能影响性能", workflow.steps.len()),
                step_id: None,
            });
        }
        
        // 检查深度嵌套
        let max_depth = self.calculate_max_depth(workflow);
        if max_depth > 20 {
            warnings.push(ValidationWarning {
                warning_type: ValidationWarningType::PerformanceIssue,
                message: format!("工作流依赖深度为 {}，可能影响执行效率", max_depth),
                step_id: None,
            });
        }
    }
    
    /// 计算最大依赖深度
    fn calculate_max_depth(&self, workflow: &WorkflowDefinition) -> usize {
        let mut depth_map: HashMap<String, usize> = HashMap::new();
        
        // 使用动态规划计算每个步骤的深度
        for step in &workflow.steps {
            self.calculate_step_depth(&step.id, workflow, &mut depth_map);
        }
        
        depth_map.values().max().copied().unwrap_or(0)
    }
    
    /// 计算步骤深度
    fn calculate_step_depth(
        &self,
        step_id: &str,
        workflow: &WorkflowDefinition,
        depth_map: &mut HashMap<String, usize>,
    ) -> usize {
        if let Some(&depth) = depth_map.get(step_id) {
            return depth;
        }
        
        let step = workflow.steps.iter().find(|s| s.id == step_id);
        if let Some(step) = step {
            let max_dep_depth = step.depends_on
                .iter()
                .map(|dep| self.calculate_step_depth(dep, workflow, depth_map))
                .max()
                .unwrap_or(0);
            
            let depth = max_dep_depth + 1;
            depth_map.insert(step_id.to_string(), depth);
            depth
        } else {
            0
        }
    }
}

/// 工作流引擎工厂
pub struct WorkflowEngineFactory;

impl WorkflowEngineFactory {
    /// 创建工作流引擎实例
    pub fn create(config: Option<WorkflowEngineConfig>) -> Arc<WorkflowEngine> {
        Arc::new(WorkflowEngine::new(config))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_workflow_definition_serialization() {
        let workflow = WorkflowDefinition {
            id: Uuid::new_v4(),
            name: "测试工作流".to_string(),
            description: "用于测试的工作流".to_string(),
            version: "1.0.0".to_string(),
            created_by: Uuid::new_v4(),
            tenant_id: Uuid::new_v4(),
            steps: vec![
                WorkflowStep {
                    id: "step1".to_string(),
                    name: "第一步".to_string(),
                    description: "测试步骤".to_string(),
                    step_type: StepType::AgentTask,
                    config: StepConfig::AgentTask {
                        agent: AgentReference::ExistingAgent { agent_id: Uuid::new_v4() },
                        task_description: "执行测试任务".to_string(),
                        parameters: HashMap::new(),
                    },
                    depends_on: Vec::new(),
                    condition: None,
                    retry_config: None,
                    timeout_seconds: None,
                    position: None,
                }
            ],
            parameters: Vec::new(),
            outputs: Vec::new(),
            config: WorkflowConfig::default(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            status: WorkflowStatus::Draft,
        };
        
        let json = serde_json::to_string(&workflow).unwrap();
        let deserialized: WorkflowDefinition = serde_json::from_str(&json).unwrap();
        
        assert_eq!(workflow.name, deserialized.name);
        assert_eq!(workflow.steps.len(), deserialized.steps.len());
    }
    
    #[tokio::test]
    async fn test_workflow_validation() {
        let engine = WorkflowEngine::new(None);
        
        let workflow = WorkflowDefinition {
            id: Uuid::new_v4(),
            name: "测试工作流".to_string(),
            description: "用于测试的工作流".to_string(),
            version: "1.0.0".to_string(),
            created_by: Uuid::new_v4(),
            tenant_id: Uuid::new_v4(),
            steps: vec![
                WorkflowStep {
                    id: "step1".to_string(),
                    name: "第一步".to_string(),
                    description: "测试步骤".to_string(),
                    step_type: StepType::AgentTask,
                    config: StepConfig::AgentTask {
                        agent: AgentReference::ExistingAgent { agent_id: Uuid::new_v4() },
                        task_description: "执行测试任务".to_string(),
                        parameters: HashMap::new(),
                    },
                    depends_on: Vec::new(),
                    condition: None,
                    retry_config: None,
                    timeout_seconds: None,
                    position: None,
                }
            ],
            parameters: Vec::new(),
            outputs: Vec::new(),
            config: WorkflowConfig::default(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            status: WorkflowStatus::Draft,
        };
        
        let result = engine.validate_workflow(&workflow).await.unwrap();
        assert!(result.is_valid);
    }
}