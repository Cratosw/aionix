# 设计文档

## 概述

企业级 AI Studio 是一个基于 Rust 技术栈构建的高性能、可扩展的 AI 服务平台。系统采用微服务架构设计，支持多租户、RAG、Agent 编排等企业级功能。核心技术栈包括 Actix Web、SeaORM、PostgreSQL 和 Rig AI 框架。

## 架构设计

### 系统架构图

```mermaid
graph TB
    subgraph "客户端层"
        WEB[Web 界面]
        API_CLIENT[API 客户端]
        PLUGIN[自定义插件]
    end
    
    subgraph "API 网关层"
        GATEWAY[API 网关]
        AUTH[认证服务]
        RATE_LIMIT[限流服务]
    end
    
    subgraph "应用服务层"
        QA_SERVICE[问答服务]
        KB_SERVICE[知识库服务]
        AGENT_SERVICE[Agent 服务]
        WORKFLOW_SERVICE[工作流服务]
        TENANT_SERVICE[租户服务]
    end
    
    subgraph "AI 处理层"
        RAG_ENGINE[RAG 引擎]
        VECTOR_SEARCH[向量检索]
        LLM_SERVICE[大语言模型]
        AGENT_RUNTIME[Agent 运行时]
    end
    
    subgraph "数据存储层"
        POSTGRES[(PostgreSQL)]
        VECTOR_DB[(向量数据库)]
        REDIS[(Redis 缓存)]
        FILE_STORAGE[文件存储]
    end
    
    WEB --> GATEWAY
    API_CLIENT --> GATEWAY
    PLUGIN --> GATEWAY
    
    GATEWAY --> AUTH
    GATEWAY --> RATE_LIMIT
    GATEWAY --> QA_SERVICE
    GATEWAY --> KB_SERVICE
    GATEWAY --> AGENT_SERVICE
    GATEWAY --> WORKFLOW_SERVICE
    GATEWAY --> TENANT_SERVICE
    
    QA_SERVICE --> RAG_ENGINE
    KB_SERVICE --> VECTOR_SEARCH
    AGENT_SERVICE --> AGENT_RUNTIME
    WORKFLOW_SERVICE --> AGENT_RUNTIME
    
    RAG_ENGINE --> VECTOR_SEARCH
    RAG_ENGINE --> LLM_SERVICE
    VECTOR_SEARCH --> VECTOR_DB
    
    QA_SERVICE --> POSTGRES
    KB_SERVICE --> POSTGRES
    AGENT_SERVICE --> POSTGRES
    WORKFLOW_SERVICE --> POSTGRES
    TENANT_SERVICE --> POSTGRES
    
    KB_SERVICE --> FILE_STORAGE
    AGENT_SERVICE --> REDIS
    WORKFLOW_SERVICE --> REDIS
```

### 技术架构

#### 核心框架
- **Web 框架**: Actix Web 4.x - 提供高性能异步 HTTP 服务
- **ORM 框架**: SeaORM 0.12.x - 异步数据库操作和实体管理
- **AI 框架**: Rig - AI 模型集成和推理服务
- **向量数据库**: pgvector (PostgreSQL 扩展) - 向量存储和检索

#### 数据库设计
- **主数据库**: PostgreSQL 15+ - 存储业务数据和元数据
- **向量存储**: pgvector - 文档嵌入向量存储
- **缓存层**: Redis - 会话缓存和临时数据
- **文件存储**: 本地文件系统或对象存储

## 组件和接口设计

### 1. 多租户管理组件

#### 租户实体设计
```rust
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "tenants")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub name: String,
    pub slug: String, // 租户标识符
    pub status: TenantStatus,
    pub config: Json, // 租户配置
    pub quota_limits: Json, // 配额限制
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

#### 租户中间件
```rust
pub struct TenantMiddleware;

impl<S, B> Transform<S, ServiceRequest> for TenantMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
{
    // 从请求头或子域名提取租户信息
    // 验证租户状态和权限
    // 设置数据库连接的租户上下文
}
```

### 2. RAG 检索增强生成组件

#### 文档处理流水线
```rust
pub struct DocumentProcessor {
    pub text_extractor: TextExtractor,
    pub chunker: DocumentChunker,
    pub embedder: EmbeddingGenerator,
    pub vector_store: VectorStore,
}

impl DocumentProcessor {
    pub async fn process_document(&self, doc: Document) -> Result<ProcessedDocument> {
        let text = self.text_extractor.extract(&doc).await?;
        let chunks = self.chunker.chunk_text(&text)?;
        let embeddings = self.embedder.generate_embeddings(&chunks).await?;
        self.vector_store.store_embeddings(&embeddings).await?;
        Ok(ProcessedDocument { chunks, embeddings })
    }
}
```

#### RAG 查询引擎
```rust
pub struct RagEngine {
    pub vector_store: VectorStore,
    pub llm_client: LlmClient,
    pub reranker: Option<Reranker>,
}

impl RagEngine {
    pub async fn query(&self, question: &str, tenant_id: Uuid) -> Result<RagResponse> {
        // 1. 生成问题向量
        let query_embedding = self.generate_query_embedding(question).await?;
        
        // 2. 向量检索
        let retrieved_docs = self.vector_store
            .similarity_search(&query_embedding, tenant_id, 10)
            .await?;
        
        // 3. 重排序（可选）
        let ranked_docs = if let Some(reranker) = &self.reranker {
            reranker.rerank(question, &retrieved_docs).await?
        } else {
            retrieved_docs
        };
        
        // 4. 构建提示词
        let prompt = self.build_rag_prompt(question, &ranked_docs)?;
        
        // 5. LLM 生成答案
        let response = self.llm_client.generate(&prompt).await?;
        
        Ok(RagResponse {
            answer: response.text,
            sources: ranked_docs,
            confidence: response.confidence,
        })
    }
}
```

### 3. 知识库管理组件

#### 知识库实体
```rust
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "knowledge_bases")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub config: Json,
    pub document_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "documents")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub knowledge_base_id: Uuid,
    pub title: String,
    pub content: Text,
    pub file_path: Option<String>,
    pub file_type: String,
    pub metadata: Json,
    pub status: DocumentStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

#### 文档管理服务
```rust
pub struct DocumentService {
    pub db: DatabaseConnection,
    pub processor: DocumentProcessor,
    pub file_storage: FileStorage,
}

impl DocumentService {
    pub async fn create_document(&self, req: CreateDocumentRequest) -> Result<Document> {
        // 1. 保存文件
        let file_path = self.file_storage.save_file(&req.file).await?;
        
        // 2. 创建文档记录
        let document = documents::ActiveModel {
            id: Set(Uuid::new_v4()),
            knowledge_base_id: Set(req.knowledge_base_id),
            title: Set(req.title),
            file_path: Set(Some(file_path)),
            file_type: Set(req.file_type),
            status: Set(DocumentStatus::Processing),
            ..Default::default()
        };
        
        let doc = document.insert(&self.db).await?;
        
        // 3. 异步处理文档
        tokio::spawn(async move {
            self.process_document_async(doc.id).await;
        });
        
        Ok(doc)
    }
    
    pub async fn batch_import(&self, files: Vec<FileUpload>) -> Result<BatchImportResult> {
        // 批量文档导入逻辑
    }
}
```

### 4. Agent 系统组件

#### Agent 实体设计
```rust
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "agents")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub agent_type: AgentType,
    pub config: Json, // Agent 配置
    pub tools: Json,  // 可用工具列表
    pub status: AgentStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "agent_executions")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub agent_id: Uuid,
    pub input: Json,
    pub output: Option<Json>,
    pub status: ExecutionStatus,
    pub error_message: Option<String>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}
```

#### Agent 运行时
```rust
pub struct AgentRuntime {
    pub llm_client: LlmClient,
    pub tool_registry: ToolRegistry,
    pub memory: AgentMemory,
}

impl AgentRuntime {
    pub async fn execute_agent(&self, agent: &Agent, input: AgentInput) -> Result<AgentOutput> {
        let execution_id = Uuid::new_v4();
        
        // 1. 记录执行开始
        self.record_execution_start(execution_id, &agent.id, &input).await?;
        
        // 2. 构建 Agent 提示词
        let prompt = self.build_agent_prompt(agent, &input).await?;
        
        // 3. 执行推理循环
        let mut context = ExecutionContext::new(execution_id, agent.clone());
        let result = self.reasoning_loop(&mut context, prompt).await;
        
        // 4. 记录执行结果
        self.record_execution_result(execution_id, &result).await?;
        
        result
    }
    
    async fn reasoning_loop(&self, context: &mut ExecutionContext, initial_prompt: String) -> Result<AgentOutput> {
        let mut current_prompt = initial_prompt;
        let mut iteration = 0;
        const MAX_ITERATIONS: usize = 10;
        
        while iteration < MAX_ITERATIONS {
            // LLM 推理
            let response = self.llm_client.generate(&current_prompt).await?;
            
            // 解析响应，检查是否需要工具调用
            if let Some(tool_call) = self.parse_tool_call(&response.text)? {
                let tool_result = self.execute_tool(&tool_call).await?;
                current_prompt = self.update_prompt_with_tool_result(&response.text, &tool_result);
                iteration += 1;
            } else {
                // 没有工具调用，返回最终结果
                return Ok(AgentOutput {
                    text: response.text,
                    metadata: response.metadata,
                });
            }
        }
        
        Err(AgentError::MaxIterationsExceeded)
    }
}
```

### 5. Agent 编排组件

#### 工作流实体
```rust
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "workflows")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub definition: Json, // 工作流定义 (DAG)
    pub status: WorkflowStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "workflow_executions")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub workflow_id: Uuid,
    pub input: Json,
    pub output: Option<Json>,
    pub status: ExecutionStatus,
    pub current_step: Option<String>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}
```

#### 工作流引擎
```rust
pub struct WorkflowEngine {
    pub agent_runtime: AgentRuntime,
    pub db: DatabaseConnection,
    pub redis: RedisConnection,
}

impl WorkflowEngine {
    pub async fn execute_workflow(&self, workflow: &Workflow, input: WorkflowInput) -> Result<WorkflowOutput> {
        let execution_id = Uuid::new_v4();
        
        // 1. 创建执行记录
        self.create_execution_record(execution_id, workflow, &input).await?;
        
        // 2. 解析工作流定义
        let dag = self.parse_workflow_definition(&workflow.definition)?;
        
        // 3. 执行工作流
        let mut executor = WorkflowExecutor::new(execution_id, dag, self.agent_runtime.clone());
        let result = executor.execute(input).await;
        
        // 4. 更新执行记录
        self.update_execution_record(execution_id, &result).await?;
        
        result
    }
}

pub struct WorkflowExecutor {
    pub execution_id: Uuid,
    pub dag: WorkflowDAG,
    pub agent_runtime: AgentRuntime,
    pub step_results: HashMap<String, StepResult>,
}

impl WorkflowExecutor {
    pub async fn execute(&mut self, input: WorkflowInput) -> Result<WorkflowOutput> {
        let mut pending_steps = self.dag.get_entry_points();
        let mut completed_steps = HashSet::new();
        
        while !pending_steps.is_empty() {
            let mut next_batch = Vec::new();
            
            for step_id in pending_steps {
                if self.can_execute_step(&step_id, &completed_steps) {
                    next_batch.push(step_id);
                }
            }
            
            if next_batch.is_empty() {
                return Err(WorkflowError::DeadlockDetected);
            }
            
            // 并行执行当前批次的步骤
            let batch_results = self.execute_step_batch(next_batch, &input).await?;
            
            for (step_id, result) in batch_results {
                self.step_results.insert(step_id.clone(), result);
                completed_steps.insert(step_id.clone());
                
                // 添加下一批可执行的步骤
                for next_step in self.dag.get_next_steps(&step_id) {
                    if !completed_steps.contains(&next_step) {
                        pending_steps.insert(next_step);
                    }
                }
                pending_steps.remove(&step_id);
            }
        }
        
        // 构建最终输出
        self.build_workflow_output()
    }
}
```

## 数据模型设计

### 核心实体关系图

```mermaid
erDiagram
    TENANTS ||--o{ KNOWLEDGE_BASES : owns
    TENANTS ||--o{ AGENTS : owns
    TENANTS ||--o{ WORKFLOWS : owns
    TENANTS ||--o{ USERS : contains
    
    KNOWLEDGE_BASES ||--o{ DOCUMENTS : contains
    DOCUMENTS ||--o{ DOCUMENT_CHUNKS : split_into
    DOCUMENT_CHUNKS ||--o{ EMBEDDINGS : has
    
    AGENTS ||--o{ AGENT_EXECUTIONS : executes
    WORKFLOWS ||--o{ WORKFLOW_EXECUTIONS : executes
    WORKFLOW_EXECUTIONS ||--o{ STEP_EXECUTIONS : contains
    
    USERS ||--o{ CONVERSATIONS : initiates
    CONVERSATIONS ||--o{ MESSAGES : contains
    MESSAGES }o--|| RAG_RESPONSES : generates
    
    TENANTS {
        uuid id PK
        string name
        string slug UK
        enum status
        json config
        json quota_limits
        timestamp created_at
        timestamp updated_at
    }
    
    KNOWLEDGE_BASES {
        uuid id PK
        uuid tenant_id FK
        string name
        text description
        json config
        int document_count
        timestamp created_at
        timestamp updated_at
    }
    
    DOCUMENTS {
        uuid id PK
        uuid knowledge_base_id FK
        string title
        text content
        string file_path
        string file_type
        json metadata
        enum status
        timestamp created_at
        timestamp updated_at
    }
    
    AGENTS {
        uuid id PK
        uuid tenant_id FK
        string name
        text description
        enum agent_type
        json config
        json tools
        enum status
        timestamp created_at
        timestamp updated_at
    }
    
    WORKFLOWS {
        uuid id PK
        uuid tenant_id FK
        string name
        text description
        json definition
        enum status
        timestamp created_at
        timestamp updated_at
    }
```

## 错误处理设计

### 错误类型定义
```rust
#[derive(Debug, thiserror::Error)]
pub enum AiStudioError {
    #[error("租户错误: {0}")]
    Tenant(#[from] TenantError),
    
    #[error("知识库错误: {0}")]
    KnowledgeBase(#[from] KnowledgeBaseError),
    
    #[error("RAG 错误: {0}")]
    Rag(#[from] RagError),
    
    #[error("Agent 错误: {0}")]
    Agent(#[from] AgentError),
    
    #[error("工作流错误: {0}")]
    Workflow(#[from] WorkflowError),
    
    #[error("数据库错误: {0}")]
    Database(#[from] sea_orm::DbErr),
    
    #[error("验证错误: {0}")]
    Validation(String),
    
    #[error("权限错误: {0}")]
    Permission(String),
    
    #[error("配额超限: {0}")]
    QuotaExceeded(String),
}
```

### 全局错误处理中间件
```rust
pub struct ErrorHandlerMiddleware;

impl<S, B> Transform<S, ServiceRequest> for ErrorHandlerMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
{
    fn new_transform(&self, service: S) -> Self::Future {
        // 统一错误处理和日志记录
        // 根据错误类型返回适当的 HTTP 状态码
        // 记录错误日志和监控指标
    }
}
```

## 测试策略

### 测试层次结构

#### 1. 单元测试
- **实体模型测试**: 验证数据模型的序列化/反序列化
- **服务层测试**: 测试业务逻辑和数据处理
- **工具函数测试**: 测试辅助函数和工具类

#### 2. 集成测试
- **数据库集成测试**: 使用测试数据库验证 ORM 操作
- **API 集成测试**: 测试 HTTP 接口的完整流程
- **外部服务集成测试**: 测试与 AI 模型和向量数据库的集成

#### 3. 端到端测试
- **用户场景测试**: 模拟完整的用户操作流程
- **多租户测试**: 验证租户隔离和权限控制
- **性能测试**: 测试系统在高负载下的表现

### 测试工具和框架
```rust
// 测试配置
#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::test;
    use sea_orm::{Database, DatabaseConnection};
    use testcontainers::clients::Cli;
    use testcontainers::images::postgres::Postgres;
    
    async fn setup_test_db() -> DatabaseConnection {
        // 使用 testcontainers 创建测试数据库
        let docker = Cli::default();
        let postgres = docker.run(Postgres::default());
        let connection_string = format!(
            "postgres://postgres:postgres@127.0.0.1:{}/postgres",
            postgres.get_host_port_ipv4(5432)
        );
        Database::connect(&connection_string).await.unwrap()
    }
    
    #[actix_web::test]
    async fn test_create_knowledge_base() {
        let db = setup_test_db().await;
        let service = KnowledgeBaseService::new(db);
        
        let request = CreateKnowledgeBaseRequest {
            name: "测试知识库".to_string(),
            description: Some("测试描述".to_string()),
        };
        
        let result = service.create_knowledge_base(request).await;
        assert!(result.is_ok());
    }
}
```

## 部署和运维

### 容器化部署
```dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/aionix /usr/local/bin/aionix
EXPOSE 8080
CMD ["aionix"]
```

### 监控和日志
- **应用监控**: 使用 Prometheus + Grafana 监控系统指标
- **日志聚合**: 使用 ELK Stack 或 Loki 收集和分析日志
- **链路追踪**: 使用 Jaeger 或 Zipkin 进行分布式追踪
- **健康检查**: 实现 /health 端点用于负载均衡器检查

### 配置管理
```rust
#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub ai: AiConfig,
    pub redis: RedisConfig,
    pub storage: StorageConfig,
}

#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub workers: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct AiConfig {
    pub model_endpoint: String,
    pub api_key: String,
    pub max_tokens: u32,
    pub temperature: f32,
}
```

这个设计文档提供了企业级 AI Studio 系统的完整技术架构，涵盖了所有核心功能模块的详细设计，为后续的实现提供了清晰的技术指导。