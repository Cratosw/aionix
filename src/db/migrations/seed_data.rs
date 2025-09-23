// 种子数据管理
// 用于开发和测试环境的初始数据

use crate::errors::AiStudioError;
use sea_orm::{DatabaseConnection, Statement, ConnectionTrait};
use tracing::{info, warn, instrument};
use uuid::Uuid;

/// 种子数据管理器
pub struct SeedDataManager {
    db: DatabaseConnection,
}

impl SeedDataManager {
    /// 创建新的种子数据管理器
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// 初始化所有种子数据
    #[instrument(skip(self))]
    pub async fn seed_all(&self) -> Result<(), AiStudioError> {
        info!("开始初始化种子数据");

        // 检查是否已经有数据
        if self.has_existing_data().await? {
            info!("检测到现有数据，跳过种子数据初始化");
            return Ok(());
        }

        // 创建默认租户
        let tenant_id = self.create_default_tenant().await?;
        
        // 创建管理员用户
        let admin_user_id = self.create_admin_user(tenant_id).await?;
        
        // 创建示例知识库
        let kb_id = self.create_sample_knowledge_base(tenant_id, admin_user_id).await?;
        
        // 创建示例文档
        self.create_sample_documents(kb_id, admin_user_id).await?;
        
        // 创建示例 Agent
        self.create_sample_agents(tenant_id, admin_user_id).await?;
        
        // 创建示例工作流
        self.create_sample_workflows(tenant_id, admin_user_id).await?;

        info!("种子数据初始化完成");
        Ok(())
    }

    /// 检查是否已有数据
    async fn has_existing_data(&self) -> Result<bool, AiStudioError> {
        let query = "SELECT COUNT(*) as count FROM tenants";
        let result = self.db.query_one(Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            query.to_string(),
        )).await?;

        if let Some(row) = result {
            let count: i64 = row.try_get("", "count").unwrap_or(0);
            Ok(count > 0)
        } else {
            Ok(false)
        }
    }

    /// 创建默认租户
    #[instrument(skip(self))]
    async fn create_default_tenant(&self) -> Result<Uuid, AiStudioError> {
        info!("创建默认租户");

        let tenant_id = Uuid::new_v4();
        let sql = format!(
            r#"
            INSERT INTO tenants (
                id, name, slug, display_name, description, status, 
                config, quota_limits, contact_email
            ) VALUES (
                '{}', 'default', 'default', '默认租户', 
                '系统默认租户，用于开发和测试', 'active',
                '{{"environment": "development"}}',
                '{{"max_users": 100, "max_knowledge_bases": 10, "max_documents": 1000, "max_agents": 20, "max_workflows": 10}}',
                'admin@example.com'
            )
            "#,
            tenant_id
        );

        self.db.execute(Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            sql,
        )).await?;

        info!(tenant_id = %tenant_id, "默认租户创建成功");
        Ok(tenant_id)
    }

    /// 创建管理员用户
    #[instrument(skip(self))]
    async fn create_admin_user(&self, tenant_id: Uuid) -> Result<Uuid, AiStudioError> {
        info!("创建管理员用户");

        let user_id = Uuid::new_v4();
        // 使用简单的密码哈希（生产环境应使用 bcrypt 等）
        let password_hash = "admin123"; // 实际应该是哈希值
        
        let sql = format!(
            r#"
            INSERT INTO users (
                id, tenant_id, username, email, password_hash, 
                display_name, status, role, permissions
            ) VALUES (
                '{}', '{}', 'admin', 'admin@example.com', '{}',
                '系统管理员', 'active', 'admin',
                '["*"]'
            )
            "#,
            user_id, tenant_id, password_hash
        );

        self.db.execute(Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            sql,
        )).await?;

        info!(user_id = %user_id, "管理员用户创建成功");
        Ok(user_id)
    }

    /// 创建示例知识库
    #[instrument(skip(self))]
    async fn create_sample_knowledge_base(&self, tenant_id: Uuid, user_id: Uuid) -> Result<Uuid, AiStudioError> {
        info!("创建示例知识库");

        let kb_id = Uuid::new_v4();
        let sql = format!(
            r#"
            INSERT INTO knowledge_bases (
                id, tenant_id, name, description, status, 
                config, embedding_model, chunk_size, chunk_overlap, created_by
            ) VALUES (
                '{}', '{}', '示例知识库', '用于演示和测试的示例知识库',
                'active', '{{"auto_update": true, "public": false}}',
                'text-embedding-ada-002', 1000, 200, '{}'
            )
            "#,
            kb_id, tenant_id, user_id
        );

        self.db.execute(Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            sql,
        )).await?;

        info!(kb_id = %kb_id, "示例知识库创建成功");
        Ok(kb_id)
    }

    /// 创建示例文档
    #[instrument(skip(self))]
    async fn create_sample_documents(&self, kb_id: Uuid, user_id: Uuid) -> Result<(), AiStudioError> {
        info!("创建示例文档");

        let documents = vec![
            SampleDocument {
                title: "AI Studio 用户指南".to_string(),
                content: r#"
# AI Studio 用户指南

## 简介
AI Studio 是一个企业级的人工智能问答和知识管理平台。

## 主要功能
1. 智能问答：基于知识库的智能问答系统
2. 知识管理：文档上传、管理和检索
3. Agent 系统：可定制的智能助手
4. 工作流编排：复杂任务的自动化处理

## 快速开始
1. 创建知识库
2. 上传文档
3. 开始提问

## 常见问题
Q: 如何上传文档？
A: 在知识库页面点击"上传文档"按钮，选择文件即可。

Q: 支持哪些文档格式？
A: 支持 PDF、Word、Markdown、TXT 等格式。
                "#.to_string(),
                file_type: "markdown".to_string(),
                tags: vec!["用户指南".to_string(), "帮助".to_string()],
            },
            SampleDocument {
                title: "API 开发文档".to_string(),
                content: r#"
# API 开发文档

## 认证
所有 API 请求都需要在请求头中包含认证信息：
```
Authorization: Bearer <your-api-key>
X-Tenant-ID: <tenant-id>
```

## 基础 API

### 获取知识库列表
```
GET /api/v1/knowledge-bases
```

### 创建知识库
```
POST /api/v1/knowledge-bases
Content-Type: application/json

{
  "name": "我的知识库",
  "description": "知识库描述"
}
```

### 上传文档
```
POST /api/v1/knowledge-bases/{id}/documents
Content-Type: multipart/form-data

file: <文件>
title: <标题>
```

### 智能问答
```
POST /api/v1/qa/ask
Content-Type: application/json

{
  "question": "你的问题",
  "knowledge_base_id": "知识库ID"
}
```

## 错误处理
API 使用标准的 HTTP 状态码：
- 200: 成功
- 400: 请求错误
- 401: 未授权
- 404: 资源不存在
- 500: 服务器错误
                "#.to_string(),
                file_type: "markdown".to_string(),
                tags: vec!["API".to_string(), "开发".to_string()],
            },
            SampleDocument {
                title: "系统配置说明".to_string(),
                content: r#"
# 系统配置说明

## 数据库配置
```toml
[database]
url = "postgresql://user:password@localhost/aionix"
max_connections = 10
min_connections = 1
```

## AI 模型配置
```toml
[ai]
model_endpoint = "https://api.openai.com/v1"
api_key = "your-api-key"
model_name = "gpt-3.5-turbo"
max_tokens = 2048
temperature = 0.7
```

## 向量数据库配置
```toml
[vector_db]
embedding_model = "text-embedding-ada-002"
dimension = 1536
similarity_threshold = 0.8
```

## 缓存配置
```toml
[redis]
url = "redis://localhost:6379"
pool_size = 10
timeout = 5000
```

## 日志配置
```toml
[logging]
level = "info"
format = "json"
output = "stdout"
```
                "#.to_string(),
                file_type: "markdown".to_string(),
                tags: vec!["配置".to_string(), "系统".to_string()],
            },
        ];

        for (index, doc) in documents.iter().enumerate() {
            let doc_id = Uuid::new_v4();
            let sql = format!(
                r#"
                INSERT INTO documents (
                    id, knowledge_base_id, title, content, file_type,
                    status, tags, language, created_by
                ) VALUES (
                    '{}', '{}', '{}', '{}', '{}',
                    'completed', ARRAY[{}], 'zh', '{}'
                )
                "#,
                doc_id,
                kb_id,
                doc.title.replace("'", "''"),
                doc.content.replace("'", "''"),
                doc.file_type,
                doc.tags.iter().map(|t| format!("'{}'", t)).collect::<Vec<_>>().join(","),
                user_id
            );

            self.db.execute(Statement::from_string(
                sea_orm::DatabaseBackend::Postgres,
                sql,
            )).await?;

            // 创建文档块（简化版本）
            self.create_sample_chunks(doc_id, &doc.content).await?;
        }

        info!("示例文档创建成功");
        Ok(())
    }

    /// 创建示例文档块
    async fn create_sample_chunks(&self, doc_id: Uuid, content: &str) -> Result<(), AiStudioError> {
        // 简单的文档分块逻辑
        let chunks: Vec<&str> = content
            .split("\n\n")
            .filter(|chunk| !chunk.trim().is_empty())
            .collect();

        for (index, chunk) in chunks.iter().enumerate() {
            let chunk_id = Uuid::new_v4();
            let content_hash = format!("{:x}", md5::compute(chunk.as_bytes()));
            
            let sql = format!(
                r#"
                INSERT INTO document_chunks (
                    id, document_id, chunk_index, content, content_hash,
                    token_count, start_position, end_position
                ) VALUES (
                    '{}', '{}', {}, '{}', '{}',
                    {}, 0, {}
                )
                "#,
                chunk_id,
                doc_id,
                index,
                chunk.replace("'", "''"),
                content_hash,
                chunk.len() / 4, // 粗略估算 token 数量
                chunk.len()
            );

            self.db.execute(Statement::from_string(
                sea_orm::DatabaseBackend::Postgres,
                sql,
            )).await?;
        }

        Ok(())
    }

    /// 创建示例 Agent
    #[instrument(skip(self))]
    async fn create_sample_agents(&self, tenant_id: Uuid, user_id: Uuid) -> Result<(), AiStudioError> {
        info!("创建示例 Agent");

        let agents = vec![
            SampleAgent {
                name: "智能客服".to_string(),
                description: "专业的客服助手，能够回答常见问题".to_string(),
                agent_type: "conversational".to_string(),
                system_prompt: "你是一个专业的客服助手。请礼貌、准确地回答用户的问题。如果不确定答案，请诚实告知并建议用户联系人工客服。".to_string(),
                tools: vec!["knowledge_search".to_string(), "ticket_create".to_string()],
            },
            SampleAgent {
                name: "文档分析师".to_string(),
                description: "专门用于分析和总结文档内容".to_string(),
                agent_type: "task".to_string(),
                system_prompt: "你是一个文档分析专家。请仔细阅读文档内容，提供准确的分析和总结。".to_string(),
                tools: vec!["document_read".to_string(), "text_analysis".to_string()],
            },
            SampleAgent {
                name: "工作流协调器".to_string(),
                description: "负责协调和管理复杂的工作流程".to_string(),
                agent_type: "workflow".to_string(),
                system_prompt: "你是一个工作流协调专家。请根据任务要求，合理安排和执行工作流程。".to_string(),
                tools: vec!["workflow_execute".to_string(), "task_schedule".to_string()],
            },
        ];

        for agent in agents {
            let agent_id = Uuid::new_v4();
            let tools_json = serde_json::to_string(&agent.tools).unwrap();
            
            let sql = format!(
                r#"
                INSERT INTO agents (
                    id, tenant_id, name, description, agent_type,
                    status, system_prompt, tools, created_by
                ) VALUES (
                    '{}', '{}', '{}', '{}', '{}',
                    'active', '{}', '{}', '{}'
                )
                "#,
                agent_id,
                tenant_id,
                agent.name.replace("'", "''"),
                agent.description.replace("'", "''"),
                agent.agent_type,
                agent.system_prompt.replace("'", "''"),
                tools_json.replace("'", "''"),
                user_id
            );

            self.db.execute(Statement::from_string(
                sea_orm::DatabaseBackend::Postgres,
                sql,
            )).await?;
        }

        info!("示例 Agent 创建成功");
        Ok(())
    }

    /// 创建示例工作流
    #[instrument(skip(self))]
    async fn create_sample_workflows(&self, tenant_id: Uuid, user_id: Uuid) -> Result<(), AiStudioError> {
        info!("创建示例工作流");

        let workflow_id = Uuid::new_v4();
        let definition = serde_json::json!({
            "name": "文档处理流程",
            "description": "自动处理上传的文档",
            "steps": [
                {
                    "id": "extract_text",
                    "name": "提取文本",
                    "type": "document_processor",
                    "config": {
                        "extract_images": true,
                        "extract_tables": true
                    }
                },
                {
                    "id": "analyze_content",
                    "name": "分析内容",
                    "type": "agent",
                    "agent_name": "文档分析师",
                    "depends_on": ["extract_text"]
                },
                {
                    "id": "generate_summary",
                    "name": "生成摘要",
                    "type": "agent",
                    "agent_name": "文档分析师",
                    "depends_on": ["analyze_content"]
                },
                {
                    "id": "create_chunks",
                    "name": "创建文档块",
                    "type": "chunker",
                    "config": {
                        "chunk_size": 1000,
                        "overlap": 200
                    },
                    "depends_on": ["extract_text"]
                },
                {
                    "id": "generate_embeddings",
                    "name": "生成向量嵌入",
                    "type": "embedder",
                    "depends_on": ["create_chunks"]
                }
            ]
        });

        let sql = format!(
            r#"
            INSERT INTO workflows (
                id, tenant_id, name, description, status,
                definition, tags, created_by
            ) VALUES (
                '{}', '{}', '文档处理流程', '自动处理上传文档的完整流程',
                'active', '{}', ARRAY['文档', '自动化'], '{}'
            )
            "#,
            workflow_id,
            tenant_id,
            definition.to_string().replace("'", "''"),
            user_id
        );

        self.db.execute(Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            sql,
        )).await?;

        info!("示例工作流创建成功");
        Ok(())
    }

    /// 清理种子数据
    #[instrument(skip(self))]
    pub async fn clean_seed_data(&self) -> Result<(), AiStudioError> {
        warn!("清理种子数据");

        let tables = vec![
            "step_executions",
            "workflow_executions", 
            "workflows",
            "agent_executions",
            "agents",
            "embeddings",
            "document_chunks",
            "documents",
            "knowledge_bases",
            "sessions",
            "users",
            "tenants",
        ];

        for table in tables {
            let sql = format!("TRUNCATE TABLE {} CASCADE", table);
            self.db.execute(Statement::from_string(
                sea_orm::DatabaseBackend::Postgres,
                sql,
            )).await?;
        }

        info!("种子数据清理完成");
        Ok(())
    }

    /// 重新初始化种子数据
    #[instrument(skip(self))]
    pub async fn reseed(&self) -> Result<(), AiStudioError> {
        info!("重新初始化种子数据");
        
        self.clean_seed_data().await?;
        self.seed_all().await?;
        
        info!("种子数据重新初始化完成");
        Ok(())
    }
}

/// 示例文档结构
#[derive(Debug, Clone)]
struct SampleDocument {
    title: String,
    content: String,
    file_type: String,
    tags: Vec<String>,
}

/// 示例 Agent 结构
#[derive(Debug, Clone)]
struct SampleAgent {
    name: String,
    description: String,
    agent_type: String,
    system_prompt: String,
    tools: Vec<String>,
}