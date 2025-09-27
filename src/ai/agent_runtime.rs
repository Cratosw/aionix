// Agent 运行时引擎
// 实现 Agent 执行引擎、推理循环和工具调用机制

use std::sync::Arc;
use std::collections::HashMap;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{info, warn, error, debug};
use tokio::sync::{RwLock, Mutex};
use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait};

use crate::errors::AiStudioError;
use crate::ai::rig_client::RigClient;

/// Agent 运行时引擎
pub struct AgentRuntime {
    /// 数据库连接
    db: Arc<DatabaseConnection>,
    /// Rig AI 客户端
    rig_client: Arc<RigClient>,
    /// 工具注册表
    tool_registry: Arc<RwLock<ToolRegistry>>,
    /// 活跃的 Agent 实例
    active_agents: Arc<RwLock<HashMap<Uuid, AgentInstance>>>,
    /// 运行时配置
    config: AgentRuntimeConfig,
}

/// Agent 运行时配置
#[derive(Debug, Clone)]
pub struct AgentRuntimeConfig {
    /// 最大推理步数
    pub max_reasoning_steps: u32,
    /// 推理超时时间（秒）
    pub reasoning_timeout_seconds: u64,
    /// 最大并发 Agent 数
    pub max_concurrent_agents: usize,
    /// 内存管理配置
    pub memory_config: MemoryConfig,
    /// 工具调用超时时间（秒）
    pub tool_call_timeout_seconds: u64,
}

impl Default for AgentRuntimeConfig {
    fn default() -> Self {
        Self {
            max_reasoning_steps: 50,
            reasoning_timeout_seconds: 300, // 5分钟
            max_concurrent_agents: 100,
            memory_config: MemoryConfig::default(),
            tool_call_timeout_seconds: 30,
        }
    }
}

/// 内存管理配置
#[derive(Debug, Clone)]
pub struct MemoryConfig {
    /// 短期记忆容量
    pub short_term_memory_size: usize,
    /// 长期记忆容量
    pub long_term_memory_size: usize,
    /// 工作记忆容量
    pub working_memory_size: usize,
    /// 记忆压缩阈值
    pub memory_compression_threshold: usize,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            short_term_memory_size: 100,
            long_term_memory_size: 1000,
            working_memory_size: 20,
            memory_compression_threshold: 80,
        }
    }
}

/// Agent 实例
#[derive(Debug, Clone)]
pub struct AgentInstance {
    /// Agent ID
    pub agent_id: Uuid,
    /// Agent 配置
    pub config: AgentConfig,
    /// Agent 状态
    pub state: AgentState,
    /// Agent 内存
    pub memory: AgentMemory,
    /// 执行上下文
    pub execution_context: ExecutionContext,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 最后活跃时间
    pub last_active_at: DateTime<Utc>,
}

/// Agent 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Agent 名称
    pub name: String,
    /// Agent 描述
    pub description: String,
    /// 系统提示词
    pub system_prompt: String,
    /// 可用工具列表
    pub available_tools: Vec<String>,
    /// 推理策略
    pub reasoning_strategy: ReasoningStrategy,
    /// 温度参数
    pub temperature: f32,
    /// 最大令牌数
    pub max_tokens: u32,
    /// 租户 ID
    pub tenant_id: Uuid,
    /// 创建者 ID
    pub created_by: Uuid,
}

/// 推理策略
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ReasoningStrategy {
    /// 反应式推理（ReAct）
    React,
    /// 思维链推理（Chain of Thought）
    ChainOfThought,
    /// 计划与执行
    PlanAndExecute,
    /// 自我反思
    SelfReflection,
}

/// Agent 状态
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AgentState {
    /// 空闲状态
    Idle,
    /// 思考中
    Thinking,
    /// 执行工具
    ExecutingTool,
    /// 等待输入
    WaitingForInput,
    /// 已完成
    Completed,
    /// 错误状态
    Error,
    /// 已暂停
    Paused,
    /// 已停止
    Stopped,
}

/// Agent 内存系统
#[derive(Debug, Clone)]
pub struct AgentMemory {
    /// 短期记忆（当前对话）
    pub short_term: Vec<MemoryItem>,
    /// 长期记忆（持久化存储）
    pub long_term: Vec<MemoryItem>,
    /// 工作记忆（当前任务相关）
    pub working: Vec<MemoryItem>,
    /// 记忆索引
    pub memory_index: HashMap<String, Vec<usize>>,
}

/// 记忆项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryItem {
    /// 记忆 ID
    pub id: Uuid,
    /// 记忆类型
    pub memory_type: MemoryType,
    /// 记忆内容
    pub content: String,
    /// 重要性分数
    pub importance_score: f32,
    /// 访问次数
    pub access_count: u32,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 最后访问时间
    pub last_accessed_at: DateTime<Utc>,
    /// 标签
    pub tags: Vec<String>,
}

/// 记忆类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MemoryType {
    /// 对话记录
    Conversation,
    /// 任务执行记录
    TaskExecution,
    /// 工具使用记录
    ToolUsage,
    /// 学习经验
    LearningExperience,
    /// 错误记录
    ErrorRecord,
    /// 成功案例
    SuccessCase,
}

/// 执行上下文
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    /// 当前任务
    pub current_task: Option<AgentTask>,
    /// 执行历史
    pub execution_history: Vec<ExecutionStep>,
    /// 上下文变量
    pub context_variables: HashMap<String, serde_json::Value>,
    /// 会话 ID
    pub session_id: Option<Uuid>,
    /// 用户 ID
    pub user_id: Option<Uuid>,
}

/// Agent 任务
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTask {
    /// 任务 ID
    pub task_id: Uuid,
    /// 任务描述
    pub description: String,
    /// 任务目标
    pub objective: String,
    /// 任务参数
    pub parameters: HashMap<String, serde_json::Value>,
    /// 任务优先级
    pub priority: TaskPriority,
    /// 任务状态
    pub status: TaskStatus,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 截止时间
    pub deadline: Option<DateTime<Utc>>,
}

/// 任务优先级
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
#[serde(rename_all = "lowercase")]
pub enum TaskPriority {
    Low,
    Normal,
    High,
    Critical,
}

/// 任务状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

/// 执行步骤
#[derive(Debug, Clone, Serialize)]
pub struct ExecutionStep {
    /// 步骤 ID
    pub step_id: Uuid,
    /// 步骤类型
    pub step_type: StepType,
    /// 步骤描述
    pub description: String,
    /// 输入数据
    pub input: serde_json::Value,
    /// 输出数据
    pub output: Option<serde_json::Value>,
    /// 执行状态
    pub status: StepStatus,
    /// 开始时间
    pub started_at: DateTime<Utc>,
    /// 完成时间
    pub completed_at: Option<DateTime<Utc>>,
    /// 错误信息
    pub error: Option<String>,
}

/// 步骤类型
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum StepType {
    Reasoning,
    ToolCall,
    MemoryRetrieval,
    MemoryStorage,
    UserInteraction,
    TaskPlanning,
}

/// 步骤状态
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
    Running,
    Completed,
    Failed,
    Skipped,
}

/// 推理结果
#[derive(Debug, Clone, Serialize)]
pub struct ReasoningResult {
    /// 推理内容
    pub reasoning: String,
    /// 下一步行动
    pub next_action: NextAction,
    /// 置信度
    pub confidence: f32,
    /// 推理步骤
    pub reasoning_steps: Vec<String>,
}

/// 下一步行动
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum NextAction {
    /// 调用工具
    ToolCall {
        tool_name: String,
        parameters: HashMap<String, serde_json::Value>,
    },
    /// 回复用户
    Respond {
        message: String,
    },
    /// 请求更多信息
    RequestInput {
        prompt: String,
    },
    /// 完成任务
    Complete {
        result: serde_json::Value,
    },
    /// 继续思考
    ContinueReasoning {
        focus: String,
    },
}

/// 工具注册表
#[derive(Debug, Default)]
pub struct ToolRegistry {
    /// 注册的工具
    tools: HashMap<String, Box<dyn Tool + Send + Sync>>,
    /// 工具元数据
    tool_metadata: HashMap<String, ToolMetadata>,
}

/// 工具元数据
#[derive(Debug, Clone, Serialize)]
pub struct ToolMetadata {
    /// 工具名称
    pub name: String,
    /// 工具描述
    pub description: String,
    /// 参数模式
    pub parameters_schema: serde_json::Value,
    /// 工具类别
    pub category: String,
    /// 是否需要权限
    pub requires_permission: bool,
    /// 工具版本
    pub version: String,
}

/// 工具接口
pub trait Tool {
    /// 执行工具
    async fn execute(
        &self,
        parameters: HashMap<String, serde_json::Value>,
        context: &ExecutionContext,
    ) -> Result<ToolResult, AiStudioError>;
    
    /// 获取工具元数据
    fn metadata(&self) -> ToolMetadata;
    
    /// 验证参数
    fn validate_parameters(
        &self,
        parameters: &HashMap<String, serde_json::Value>,
    ) -> Result<(), AiStudioError>;
}

/// 工具执行结果
#[derive(Debug, Clone, Serialize)]
pub struct ToolResult {
    /// 执行是否成功
    pub success: bool,
    /// 结果数据
    pub data: serde_json::Value,
    /// 错误信息
    pub error: Option<String>,
    /// 执行时间（毫秒）
    pub execution_time_ms: u64,
    /// 输出消息
    pub message: Option<String>,
}

impl AgentRuntime {
    /// 创建新的 Agent 运行时
    pub fn new(
        db: Arc<DatabaseConnection>,
        rig_client: Arc<RigClient>,
        config: Option<AgentRuntimeConfig>,
    ) -> Self {
        Self {
            db,
            rig_client,
            tool_registry: Arc::new(RwLock::new(ToolRegistry::default())),
            active_agents: Arc::new(RwLock::new(HashMap::new())),
            config: config.unwrap_or_default(),
        }
    }
    
    /// 创建 Agent 实例
    pub async fn create_agent(
        &self,
        config: AgentConfig,
    ) -> Result<Uuid, AiStudioError> {
        let agent_id = Uuid::new_v4();
        let now = Utc::now();
        
        let agent_instance = AgentInstance {
            agent_id,
            config,
            state: AgentState::Idle,
            memory: AgentMemory {
                short_term: Vec::new(),
                long_term: Vec::new(),
                working: Vec::new(),
                memory_index: HashMap::new(),
            },
            execution_context: ExecutionContext {
                current_task: None,
                execution_history: Vec::new(),
                context_variables: HashMap::new(),
                session_id: None,
                user_id: None,
            },
            created_at: now,
            last_active_at: now,
        };
        
        // 检查并发限制
        {
            let active_agents = self.active_agents.read().await;
            if active_agents.len() >= self.config.max_concurrent_agents {
                return Err(AiStudioError::resource_limit("达到最大并发 Agent 数量限制"));
            }
        }
        
        // 添加到活跃 Agent 列表
        {
            let mut active_agents = self.active_agents.write().await;
            active_agents.insert(agent_id, agent_instance);
        }
        
        info!("创建 Agent 实例: agent_id={}", agent_id);
        Ok(agent_id)
    }
    
    /// 执行 Agent 任务
    pub async fn execute_task(
        &self,
        agent_id: Uuid,
        task: AgentTask,
    ) -> Result<serde_json::Value, AiStudioError> {
        debug!("开始执行 Agent 任务: agent_id={}, task_id={}", agent_id, task.task_id);
        
        // 获取 Agent 实例
        let mut agent = {
            let mut active_agents = self.active_agents.write().await;
            active_agents.get_mut(&agent_id)
                .ok_or_else(|| AiStudioError::not_found("Agent 实例不存在"))?
                .clone()
        };
        
        // 设置当前任务
        agent.execution_context.current_task = Some(task.clone());
        agent.state = AgentState::Thinking;
        
        // 执行推理循环
        let result = self.reasoning_loop(&mut agent).await?;
        
        // 更新 Agent 状态
        agent.state = AgentState::Completed;
        agent.last_active_at = Utc::now();
        
        // 保存 Agent 状态
        {
            let mut active_agents = self.active_agents.write().await;
            active_agents.insert(agent_id, agent);
        }
        
        info!("Agent 任务执行完成: agent_id={}, task_id={}", agent_id, task.task_id);
        Ok(result)
    }
    
    /// 推理循环
    async fn reasoning_loop(
        &self,
        agent: &mut AgentInstance,
    ) -> Result<serde_json::Value, AiStudioError> {
        let mut step_count = 0;
        let start_time = Utc::now();
        
        loop {
            // 检查步数限制
            if step_count >= self.config.max_reasoning_steps {
                warn!("Agent 推理步数达到上限: agent_id={}", agent.agent_id);
                break;
            }
            
            // 检查超时
            let elapsed = Utc::now().signed_duration_since(start_time);
            if elapsed.num_seconds() > self.config.reasoning_timeout_seconds as i64 {
                warn!("Agent 推理超时: agent_id={}", agent.agent_id);
                break;
            }
            
            step_count += 1;
            
            // 执行推理步骤
            let reasoning_result = self.perform_reasoning_step(agent).await?;
            
            // 处理下一步行动
            match reasoning_result.next_action {
                NextAction::ToolCall { tool_name, parameters } => {
                    let tool_result = self.execute_tool(&tool_name, parameters, &agent.execution_context).await?;
                    
                    // 将工具结果添加到记忆
                    self.add_memory_item(
                        agent,
                        MemoryType::ToolUsage,
                        format!("工具调用: {} -> {:?}", tool_name, tool_result),
                        0.7,
                    ).await;
                }
                NextAction::Respond { message } => {
                    // 添加响应到记忆
                    self.add_memory_item(
                        agent,
                        MemoryType::Conversation,
                        format!("回复: {}", message),
                        0.8,
                    ).await;
                    
                    return Ok(serde_json::json!({
                        "type": "response",
                        "message": message,
                        "reasoning_steps": step_count
                    }));
                }
                NextAction::Complete { result } => {
                    // 任务完成
                    self.add_memory_item(
                        agent,
                        MemoryType::TaskExecution,
                        format!("任务完成: {:?}", result),
                        0.9,
                    ).await;
                    
                    return Ok(result);
                }
                NextAction::RequestInput { prompt } => {
                    agent.state = AgentState::WaitingForInput;
                    return Ok(serde_json::json!({
                        "type": "input_request",
                        "prompt": prompt,
                        "reasoning_steps": step_count
                    }));
                }
                NextAction::ContinueReasoning { focus: _ } => {
                    // 继续下一轮推理
                    continue;
                }
            }
        }
        
        // 推理循环结束，返回默认结果
        Ok(serde_json::json!({
            "type": "timeout",
            "message": "推理循环超时或达到步数限制",
            "reasoning_steps": step_count
        }))
    }
    
    /// 执行推理步骤
    async fn perform_reasoning_step(
        &self,
        agent: &AgentInstance,
    ) -> Result<ReasoningResult, AiStudioError> {
        debug!("执行推理步骤: agent_id={}", agent.agent_id);
        
        // 构建推理提示
        let prompt = self.build_reasoning_prompt(agent).await?;
        
        // 调用 LLM 进行推理
        let response = self.rig_client.generate_text(&prompt, Some(agent.config.temperature)).await?;
        
        // 解析推理结果
        let reasoning_result = self.parse_reasoning_response(&response, agent).await?;
        
        debug!("推理步骤完成: agent_id={}, 下一步行动={:?}", 
               agent.agent_id, reasoning_result.next_action);
        
        Ok(reasoning_result)
    }
    
    /// 构建推理提示
    async fn build_reasoning_prompt(&self, agent: &AgentInstance) -> Result<String, AiStudioError> {
        let mut prompt = String::new();
        
        // 系统提示
        prompt.push_str(&agent.config.system_prompt);
        prompt.push_str("\n\n");
        
        // 当前任务
        if let Some(ref task) = agent.execution_context.current_task {
            prompt.push_str(&format!("当前任务: {}\n", task.description));
            prompt.push_str(&format!("任务目标: {}\n\n", task.objective));
        }
        
        // 可用工具
        if !agent.config.available_tools.is_empty() {
            prompt.push_str("可用工具:\n");
            for tool_name in &agent.config.available_tools {
                if let Some(metadata) = self.get_tool_metadata(tool_name).await {
                    prompt.push_str(&format!("- {}: {}\n", tool_name, metadata.description));
                }
            }
            prompt.push_str("\n");
        }
        
        // 相关记忆
        let relevant_memories = self.retrieve_relevant_memories(agent, 5).await;
        if !relevant_memories.is_empty() {
            prompt.push_str("相关记忆:\n");
            for memory in relevant_memories {
                prompt.push_str(&format!("- {}\n", memory.content));
            }
            prompt.push_str("\n");
        }
        
        // 推理策略指导
        match agent.config.reasoning_strategy {
            ReasoningStrategy::React => {
                prompt.push_str("请使用 ReAct 推理模式：思考(Thought) -> 行动(Action) -> 观察(Observation)\n");
            }
            ReasoningStrategy::ChainOfThought => {
                prompt.push_str("请使用思维链推理，逐步分析问题并给出推理过程\n");
            }
            ReasoningStrategy::PlanAndExecute => {
                prompt.push_str("请先制定计划，然后逐步执行\n");
            }
            ReasoningStrategy::SelfReflection => {
                prompt.push_str("请进行自我反思，评估当前进展并调整策略\n");
            }
        }
        
        prompt.push_str("\n请提供你的推理过程和下一步行动。");
        
        Ok(prompt)
    }
    
    /// 解析推理响应
    async fn parse_reasoning_response(
        &self,
        response: &str,
        agent: &AgentInstance,
    ) -> Result<ReasoningResult, AiStudioError> {
        // 这里应该实现更复杂的解析逻辑
        // 目前使用简化的解析方式
        
        let reasoning = response.to_string();
        let confidence = 0.8; // 默认置信度
        let reasoning_steps = vec![reasoning.clone()];
        
        // 简单的行动解析逻辑
        let next_action = if response.contains("工具调用") || response.contains("使用工具") {
            // 解析工具调用
            NextAction::ToolCall {
                tool_name: "search".to_string(), // 默认工具
                parameters: HashMap::new(),
            }
        } else if response.contains("完成") || response.contains("结束") {
            NextAction::Complete {
                result: serde_json::json!({"message": "任务完成"}),
            }
        } else if response.contains("需要更多信息") || response.contains("请提供") {
            NextAction::RequestInput {
                prompt: "请提供更多信息".to_string(),
            }
        } else if response.len() > 10 {
            NextAction::Respond {
                message: response.to_string(),
            }
        } else {
            NextAction::ContinueReasoning {
                focus: "继续分析问题".to_string(),
            }
        };
        
        Ok(ReasoningResult {
            reasoning,
            next_action,
            confidence,
            reasoning_steps,
        })
    }
    
    /// 执行工具
    async fn execute_tool(
        &self,
        tool_name: &str,
        parameters: HashMap<String, serde_json::Value>,
        context: &ExecutionContext,
    ) -> Result<ToolResult, AiStudioError> {
        debug!("执行工具: tool_name={}", tool_name);
        
        let tool_registry = self.tool_registry.read().await;
        let tool = tool_registry.tools.get(tool_name)
            .ok_or_else(|| AiStudioError::not_found(&format!("工具不存在: {}", tool_name)))?;
        
        // 验证参数
        tool.validate_parameters(&parameters)?;
        
        // 执行工具
        let start_time = std::time::Instant::now();
        let result = tool.execute(parameters, context).await?;
        let execution_time = start_time.elapsed().as_millis() as u64;
        
        debug!("工具执行完成: tool_name={}, 执行时间={}ms", tool_name, execution_time);
        
        Ok(ToolResult {
            success: result.success,
            data: result.data,
            error: result.error,
            execution_time_ms: execution_time,
            message: result.message,
        })
    }
    
    /// 添加记忆项
    async fn add_memory_item(
        &self,
        agent: &mut AgentInstance,
        memory_type: MemoryType,
        content: String,
        importance_score: f32,
    ) {
        let memory_item = MemoryItem {
            id: Uuid::new_v4(),
            memory_type: memory_type.clone(),
            content: content.clone(),
            importance_score,
            access_count: 0,
            created_at: Utc::now(),
            last_accessed_at: Utc::now(),
            tags: Vec::new(),
        };
        
        // 添加到短期记忆
        agent.memory.short_term.push(memory_item.clone());
        
        // 检查是否需要压缩记忆
        if agent.memory.short_term.len() > self.config.memory_config.memory_compression_threshold {
            self.compress_memory(agent).await;
        }
        
        debug!("添加记忆项: agent_id={}, 类型={:?}, 重要性={}", 
               agent.agent_id, memory_type, importance_score);
    }
    
    /// 压缩记忆
    async fn compress_memory(&self, agent: &mut AgentInstance) {
        // 将重要的短期记忆转移到长期记忆
        let mut important_memories = Vec::new();
        let mut remaining_memories = Vec::new();
        
        for memory in agent.memory.short_term.drain(..) {
            if memory.importance_score > 0.7 {
                important_memories.push(memory);
            } else if remaining_memories.len() < self.config.memory_config.short_term_memory_size / 2 {
                remaining_memories.push(memory);
            }
        }
        
        // 更新记忆
        agent.memory.short_term = remaining_memories;
        agent.memory.long_term.extend(important_memories);
        
        // 限制长期记忆大小
        if agent.memory.long_term.len() > self.config.memory_config.long_term_memory_size {
            agent.memory.long_term.sort_by(|a, b| b.importance_score.partial_cmp(&a.importance_score).unwrap());
            agent.memory.long_term.truncate(self.config.memory_config.long_term_memory_size);
        }
        
        debug!("记忆压缩完成: agent_id={}", agent.agent_id);
    }
    
    /// 检索相关记忆
    async fn retrieve_relevant_memories(
        &self,
        agent: &AgentInstance,
        limit: usize,
    ) -> Vec<MemoryItem> {
        let mut all_memories = Vec::new();
        all_memories.extend(agent.memory.short_term.iter().cloned());
        all_memories.extend(agent.memory.working.iter().cloned());
        all_memories.extend(agent.memory.long_term.iter().take(10).cloned());
        
        // 按重要性和最近访问时间排序
        all_memories.sort_by(|a, b| {
            let score_a = a.importance_score + (a.access_count as f32 * 0.1);
            let score_b = b.importance_score + (b.access_count as f32 * 0.1);
            score_b.partial_cmp(&score_a).unwrap()
        });
        
        all_memories.into_iter().take(limit).collect()
    }
    
    /// 获取工具元数据
    async fn get_tool_metadata(&self, tool_name: &str) -> Option<ToolMetadata> {
        let tool_registry = self.tool_registry.read().await;
        tool_registry.tool_metadata.get(tool_name).cloned()
    }
    
    /// 注册工具
    pub async fn register_tool(
        &self,
        tool: Box<dyn Tool + Send + Sync>,
    ) -> Result<(), AiStudioError> {
        let metadata = tool.metadata();
        let tool_name = metadata.name.clone();
        
        let mut tool_registry = self.tool_registry.write().await;
        tool_registry.tools.insert(tool_name.clone(), tool);
        tool_registry.tool_metadata.insert(tool_name.clone(), metadata);
        
        info!("注册工具: {}", tool_name);
        Ok(())
    }
    
    /// 获取 Agent 状态
    pub async fn get_agent_state(&self, agent_id: Uuid) -> Result<AgentState, AiStudioError> {
        let active_agents = self.active_agents.read().await;
        let agent = active_agents.get(&agent_id)
            .ok_or_else(|| AiStudioError::not_found("Agent 实例不存在"))?;
        
        Ok(agent.state.clone())
    }
    
    /// 停止 Agent
    pub async fn stop_agent(&self, agent_id: Uuid) -> Result<(), AiStudioError> {
        let mut active_agents = self.active_agents.write().await;
        if let Some(agent) = active_agents.get_mut(&agent_id) {
            agent.state = AgentState::Stopped;
            info!("停止 Agent: agent_id={}", agent_id);
        }
        
        Ok(())
    }
    
    /// 清理非活跃 Agent
    pub async fn cleanup_inactive_agents(&self) -> Result<u32, AiStudioError> {
        let mut active_agents = self.active_agents.write().await;
        let now = Utc::now();
        let timeout_duration = chrono::Duration::hours(1); // 1小时超时
        
        let initial_count = active_agents.len();
        active_agents.retain(|_, agent| {
            let inactive_duration = now.signed_duration_since(agent.last_active_at);
            inactive_duration < timeout_duration
        });
        
        let cleaned_count = initial_count - active_agents.len();
        
        if cleaned_count > 0 {
            info!("清理了 {} 个非活跃 Agent", cleaned_count);
        }
        
        Ok(cleaned_count as u32)
    }
}

/// Agent 运行时工厂
pub struct AgentRuntimeFactory;

impl AgentRuntimeFactory {
    /// 创建 Agent 运行时实例
    pub fn create(
        db: Arc<DatabaseConnection>,
        rig_client: Arc<RigClient>,
        config: Option<AgentRuntimeConfig>,
    ) -> Arc<AgentRuntime> {
        Arc::new(AgentRuntime::new(db, rig_client, config))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_agent_config_serialization() {
        let config = AgentConfig {
            name: "测试 Agent".to_string(),
            description: "用于测试的 Agent".to_string(),
            system_prompt: "你是一个有用的助手".to_string(),
            available_tools: vec!["search".to_string(), "calculator".to_string()],
            reasoning_strategy: ReasoningStrategy::React,
            temperature: 0.7,
            max_tokens: 1000,
            tenant_id: Uuid::new_v4(),
            created_by: Uuid::new_v4(),
        };
        
        let serialized = serde_json::to_string(&config).unwrap();
        let deserialized: AgentConfig = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(config.name, deserialized.name);
        assert_eq!(config.reasoning_strategy, deserialized.reasoning_strategy);
    }
    
    #[test]
    fn test_memory_item_creation() {
        let memory_item = MemoryItem {
            id: Uuid::new_v4(),
            memory_type: MemoryType::Conversation,
            content: "测试记忆内容".to_string(),
            importance_score: 0.8,
            access_count: 0,
            created_at: Utc::now(),
            last_accessed_at: Utc::now(),
            tags: vec!["test".to_string()],
        };
        
        assert_eq!(memory_item.memory_type, MemoryType::Conversation);
        assert_eq!(memory_item.importance_score, 0.8);
    }
}