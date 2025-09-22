// 数据库迁移脚本定义

use super::Migration;

/// 获取所有迁移
pub fn get_all_migrations() -> Vec<Migration> {
    vec![
        create_tenants_table(),
        create_users_table(),
        create_sessions_table(),
        create_knowledge_bases_table(),
        create_documents_table(),
        create_document_chunks_table(),
        create_embeddings_table(),
        create_agents_table(),
        create_agent_executions_table(),
        create_workflows_table(),
        create_workflow_executions_table(),
        create_step_executions_table(),
        add_indexes(),
        add_constraints(),
    ]
}

/// 创建租户表
fn create_tenants_table() -> Migration {
    Migration {
        version: "20240101_000001".to_string(),
        name: "create_tenants_table".to_string(),
        description: "创建租户表".to_string(),
        up_sql: r#"
            CREATE TYPE tenant_status AS ENUM ('active', 'suspended', 'inactive');

            CREATE TABLE tenants (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                name VARCHAR(255) NOT NULL UNIQUE,
                slug VARCHAR(100) NOT NULL UNIQUE,
                display_name VARCHAR(255) NOT NULL,
                description TEXT,
                status tenant_status NOT NULL DEFAULT 'active',
                config JSONB NOT NULL DEFAULT '{}',
                quota_limits JSONB NOT NULL DEFAULT '{}',
                usage_stats JSONB NOT NULL DEFAULT '{}',
                contact_email VARCHAR(255),
                contact_phone VARCHAR(50),
                created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                last_active_at TIMESTAMPTZ
            );

            CREATE INDEX idx_tenants_status ON tenants(status);
            CREATE INDEX idx_tenants_slug ON tenants(slug);
            CREATE INDEX idx_tenants_last_active ON tenants(last_active_at);
        "#.to_string(),
        down_sql: r#"
            DROP TABLE IF EXISTS tenants;
            DROP TYPE IF EXISTS tenant_status;
        "#.to_string(),
        dependencies: vec![],
    }
}

/// 创建用户表
fn create_users_table() -> Migration {
    Migration {
        version: "20240101_000002".to_string(),
        name: "create_users_table".to_string(),
        description: "创建用户表".to_string(),
        up_sql: r#"
            CREATE TYPE user_status AS ENUM ('active', 'inactive', 'suspended', 'pending');
            CREATE TYPE user_role AS ENUM ('admin', 'manager', 'user', 'viewer');

            CREATE TABLE users (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
                username VARCHAR(100) NOT NULL,
                email VARCHAR(255) NOT NULL UNIQUE,
                password_hash VARCHAR(255) NOT NULL,
                display_name VARCHAR(255) NOT NULL,
                avatar_url VARCHAR(500),
                status user_status NOT NULL DEFAULT 'pending',
                role user_role NOT NULL DEFAULT 'user',
                permissions JSONB NOT NULL DEFAULT '[]',
                preferences JSONB NOT NULL DEFAULT '{}',
                last_login_at TIMESTAMPTZ,
                email_verified_at TIMESTAMPTZ,
                created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
            );

            CREATE UNIQUE INDEX idx_users_tenant_username ON users(tenant_id, username);
            CREATE INDEX idx_users_tenant_id ON users(tenant_id);
            CREATE INDEX idx_users_email ON users(email);
            CREATE INDEX idx_users_status ON users(status);
            CREATE INDEX idx_users_role ON users(role);       
 "#.to_string(),
        down_sql: r#"
            DROP TABLE IF EXISTS users;
            DROP TYPE IF EXISTS user_status;
            DROP TYPE IF EXISTS user_role;
        "#.to_string(),
        dependencies: vec!["20240101_000001".to_string()],
    }
}

/// 创建会话表
fn create_sessions_table() -> Migration {
    Migration {
        version: "20240101_000003".to_string(),
        name: "create_sessions_table".to_string(),
        description: "创建用户会话表".to_string(),
        up_sql: r#"
            CREATE TYPE session_status AS ENUM ('active', 'expired', 'revoked');

            CREATE TABLE sessions (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
                session_token VARCHAR(255) NOT NULL UNIQUE,
                refresh_token VARCHAR(255),
                status session_status NOT NULL DEFAULT 'active',
                ip_address INET,
                user_agent TEXT,
                expires_at TIMESTAMPTZ NOT NULL,
                last_accessed_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
            );

            CREATE INDEX idx_sessions_user_id ON sessions(user_id);
            CREATE INDEX idx_sessions_tenant_id ON sessions(tenant_id);
            CREATE INDEX idx_sessions_token ON sessions(session_token);
            CREATE INDEX idx_sessions_status ON sessions(status);
            CREATE INDEX idx_sessions_expires_at ON sessions(expires_at);
        "#.to_string(),
        down_sql: r#"
            DROP TABLE IF EXISTS sessions;
            DROP TYPE IF EXISTS session_status;
        "#.to_string(),
        dependencies: vec!["20240101_000002".to_string()],
    }
}

/// 创建知识库表
fn create_knowledge_bases_table() -> Migration {
    Migration {
        version: "20240101_000004".to_string(),
        name: "create_knowledge_bases_table".to_string(),
        description: "创建知识库表".to_string(),
        up_sql: r#"
            CREATE TYPE knowledge_base_status AS ENUM ('active', 'inactive', 'processing', 'error');

            CREATE TABLE knowledge_bases (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
                name VARCHAR(255) NOT NULL,
                description TEXT,
                status knowledge_base_status NOT NULL DEFAULT 'active',
                config JSONB NOT NULL DEFAULT '{}',
                document_count INTEGER NOT NULL DEFAULT 0,
                total_size_bytes BIGINT NOT NULL DEFAULT 0,
                embedding_model VARCHAR(100),
                chunk_size INTEGER DEFAULT 1000,
                chunk_overlap INTEGER DEFAULT 200,
                created_by UUID REFERENCES users(id),
                created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
            );

            CREATE UNIQUE INDEX idx_knowledge_bases_tenant_name ON knowledge_bases(tenant_id, name);
            CREATE INDEX idx_knowledge_bases_tenant_id ON knowledge_bases(tenant_id);
            CREATE INDEX idx_knowledge_bases_status ON knowledge_bases(status);
            CREATE INDEX idx_knowledge_bases_created_by ON knowledge_bases(created_by);
        "#.to_string(),
        down_sql: r#"
            DROP TABLE IF EXISTS knowledge_bases;
            DROP TYPE IF EXISTS knowledge_base_status;
        "#.to_string(),
        dependencies: vec!["20240101_000002".to_string()],
    }
}

/// 创建文档表
fn create_documents_table() -> Migration {
    Migration {
        version: "20240101_000005".to_string(),
        name: "create_documents_table".to_string(),
        description: "创建文档表".to_string(),
        up_sql: r#"
            CREATE TYPE document_status AS ENUM ('pending', 'processing', 'completed', 'failed', 'archived');

            CREATE TABLE documents (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                knowledge_base_id UUID NOT NULL REFERENCES knowledge_bases(id) ON DELETE CASCADE,
                title VARCHAR(500) NOT NULL,
                content TEXT,
                file_path VARCHAR(1000),
                file_name VARCHAR(255),
                file_type VARCHAR(50),
                file_size_bytes BIGINT,
                mime_type VARCHAR(100),
                status document_status NOT NULL DEFAULT 'pending',
                metadata JSONB NOT NULL DEFAULT '{}',
                tags TEXT[],
                language VARCHAR(10) DEFAULT 'zh',
                processing_error TEXT,
                chunk_count INTEGER DEFAULT 0,
                created_by UUID REFERENCES users(id),
                created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                processed_at TIMESTAMPTZ
            );

            CREATE INDEX idx_documents_knowledge_base_id ON documents(knowledge_base_id);
            CREATE INDEX idx_documents_status ON documents(status);
            CREATE INDEX idx_documents_file_type ON documents(file_type);
            CREATE INDEX idx_documents_created_by ON documents(created_by);
            CREATE INDEX idx_documents_tags ON documents USING GIN(tags);
            CREATE INDEX idx_documents_title_search ON documents USING GIN(to_tsvector('chinese', title));
            CREATE INDEX idx_documents_content_search ON documents USING GIN(to_tsvector('chinese', content));
        "#.to_string(),
        down_sql: r#"
            DROP TABLE IF EXISTS documents;
            DROP TYPE IF EXISTS document_status;
        "#.to_string(),
        dependencies: vec!["20240101_000004".to_string()],
    }
}

/// 创建文档块表
fn create_document_chunks_table() -> Migration {
    Migration {
        version: "20240101_000006".to_string(),
        name: "create_document_chunks_table".to_string(),
        description: "创建文档块表".to_string(),
        up_sql: r#"
            CREATE TABLE document_chunks (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                document_id UUID NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
                chunk_index INTEGER NOT NULL,
                content TEXT NOT NULL,
                content_hash VARCHAR(64) NOT NULL,
                token_count INTEGER,
                start_position INTEGER,
                end_position INTEGER,
                metadata JSONB NOT NULL DEFAULT '{}',
                created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
            );

            CREATE UNIQUE INDEX idx_document_chunks_doc_index ON document_chunks(document_id, chunk_index);
            CREATE INDEX idx_document_chunks_document_id ON document_chunks(document_id);
            CREATE INDEX idx_document_chunks_hash ON document_chunks(content_hash);
            CREATE INDEX idx_document_chunks_content_search ON document_chunks USING GIN(to_tsvector('chinese', content));
        "#.to_string(),
        down_sql: r#"
            DROP TABLE IF EXISTS document_chunks;
        "#.to_string(),
        dependencies: vec!["20240101_000005".to_string()],
    }
}

/// 创建向量嵌入表
fn create_embeddings_table() -> Migration {
    Migration {
        version: "20240101_000007".to_string(),
        name: "create_embeddings_table".to_string(),
        description: "创建向量嵌入表".to_string(),
        up_sql: r#"
            CREATE TABLE embeddings (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                chunk_id UUID NOT NULL REFERENCES document_chunks(id) ON DELETE CASCADE,
                model_name VARCHAR(100) NOT NULL,
                vector vector(1536) NOT NULL,
                vector_norm FLOAT,
                created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
            );

            CREATE UNIQUE INDEX idx_embeddings_chunk_model ON embeddings(chunk_id, model_name);
            CREATE INDEX idx_embeddings_chunk_id ON embeddings(chunk_id);
            CREATE INDEX idx_embeddings_model ON embeddings(model_name);
            CREATE INDEX idx_embeddings_vector_cosine ON embeddings USING ivfflat (vector vector_cosine_ops) WITH (lists = 100);
            CREATE INDEX idx_embeddings_vector_l2 ON embeddings USING ivfflat (vector vector_l2_ops) WITH (lists = 100);
        "#.to_string(),
        down_sql: r#"
            DROP TABLE IF EXISTS embeddings;
        "#.to_string(),
        dependencies: vec!["20240101_000006".to_string()],
    }
}

/// 创建 Agent 表
fn create_agents_table() -> Migration {
    Migration {
        version: "20240101_000008".to_string(),
        name: "create_agents_table".to_string(),
        description: "创建 Agent 表".to_string(),
        up_sql: r#"
            CREATE TYPE agent_type AS ENUM ('conversational', 'task', 'workflow', 'tool');
            CREATE TYPE agent_status AS ENUM ('active', 'inactive', 'training', 'error');

            CREATE TABLE agents (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
                name VARCHAR(255) NOT NULL,
                description TEXT,
                agent_type agent_type NOT NULL DEFAULT 'conversational',
                status agent_status NOT NULL DEFAULT 'active',
                config JSONB NOT NULL DEFAULT '{}',
                tools JSONB NOT NULL DEFAULT '[]',
                system_prompt TEXT,
                model_config JSONB NOT NULL DEFAULT '{}',
                memory_config JSONB NOT NULL DEFAULT '{}',
                execution_stats JSONB NOT NULL DEFAULT '{}',
                created_by UUID REFERENCES users(id),
                created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                last_used_at TIMESTAMPTZ
            );

            CREATE UNIQUE INDEX idx_agents_tenant_name ON agents(tenant_id, name);
            CREATE INDEX idx_agents_tenant_id ON agents(tenant_id);
            CREATE INDEX idx_agents_type ON agents(agent_type);
            CREATE INDEX idx_agents_status ON agents(status);
            CREATE INDEX idx_agents_created_by ON agents(created_by);
        "#.to_string(),
        down_sql: r#"
            DROP TABLE IF EXISTS agents;
            DROP TYPE IF EXISTS agent_type;
            DROP TYPE IF EXISTS agent_status;
        "#.to_string(),
        dependencies: vec!["20240101_000002".to_string()],
    }
}

/// 创建 Agent 执行记录表
fn create_agent_executions_table() -> Migration {
    Migration {
        version: "20240101_000009".to_string(),
        name: "create_agent_executions_table".to_string(),
        description: "创建 Agent 执行记录表".to_string(),
        up_sql: r#"
            CREATE TYPE execution_status AS ENUM ('pending', 'running', 'completed', 'failed', 'cancelled', 'timeout');

            CREATE TABLE agent_executions (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                agent_id UUID NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
                user_id UUID REFERENCES users(id),
                session_id UUID REFERENCES sessions(id),
                input JSONB NOT NULL,
                output JSONB,
                status execution_status NOT NULL DEFAULT 'pending',
                error_message TEXT,
                error_code VARCHAR(50),
                execution_trace JSONB,
                tool_calls JSONB,
                token_usage JSONB,
                execution_time_ms INTEGER,
                started_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                completed_at TIMESTAMPTZ,
                created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
            );

            CREATE INDEX idx_agent_executions_agent_id ON agent_executions(agent_id);
            CREATE INDEX idx_agent_executions_user_id ON agent_executions(user_id);
            CREATE INDEX idx_agent_executions_session_id ON agent_executions(session_id);
            CREATE INDEX idx_agent_executions_status ON agent_executions(status);
            CREATE INDEX idx_agent_executions_started_at ON agent_executions(started_at);
        "#.to_string(),
        down_sql: r#"
            DROP TABLE IF EXISTS agent_executions;
            DROP TYPE IF EXISTS execution_status;
        "#.to_string(),
        dependencies: vec!["20240101_000008".to_string()],
    }
}

/// 创建工作流表
fn create_workflows_table() -> Migration {
    Migration {
        version: "20240101_000010".to_string(),
        name: "create_workflows_table".to_string(),
        description: "创建工作流表".to_string(),
        up_sql: r#"
            CREATE TYPE workflow_status AS ENUM ('draft', 'active', 'inactive', 'archived');

            CREATE TABLE workflows (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
                name VARCHAR(255) NOT NULL,
                description TEXT,
                status workflow_status NOT NULL DEFAULT 'draft',
                definition JSONB NOT NULL,
                version INTEGER NOT NULL DEFAULT 1,
                tags TEXT[],
                execution_stats JSONB NOT NULL DEFAULT '{}',
                created_by UUID REFERENCES users(id),
                created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                published_at TIMESTAMPTZ
            );

            CREATE UNIQUE INDEX idx_workflows_tenant_name ON workflows(tenant_id, name);
            CREATE INDEX idx_workflows_tenant_id ON workflows(tenant_id);
            CREATE INDEX idx_workflows_status ON workflows(status);
            CREATE INDEX idx_workflows_created_by ON workflows(created_by);
            CREATE INDEX idx_workflows_tags ON workflows USING GIN(tags);
        "#.to_string(),
        down_sql: r#"
            DROP TABLE IF EXISTS workflows;
            DROP TYPE IF EXISTS workflow_status;
        "#.to_string(),
        dependencies: vec!["20240101_000002".to_string()],
    }
}

/// 创建工作流执行记录表
fn create_workflow_executions_table() -> Migration {
    Migration {
        version: "20240101_000011".to_string(),
        name: "create_workflow_executions_table".to_string(),
        description: "创建工作流执行记录表".to_string(),
        up_sql: r#"
            CREATE TABLE workflow_executions (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                workflow_id UUID NOT NULL REFERENCES workflows(id) ON DELETE CASCADE,
                user_id UUID REFERENCES users(id),
                session_id UUID REFERENCES sessions(id),
                input JSONB NOT NULL,
                output JSONB,
                status execution_status NOT NULL DEFAULT 'pending',
                current_step VARCHAR(255),
                step_count INTEGER DEFAULT 0,
                completed_steps INTEGER DEFAULT 0,
                error_message TEXT,
                error_code VARCHAR(50),
                execution_trace JSONB,
                execution_time_ms INTEGER,
                started_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                completed_at TIMESTAMPTZ,
                created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
            );

            CREATE INDEX idx_workflow_executions_workflow_id ON workflow_executions(workflow_id);
            CREATE INDEX idx_workflow_executions_user_id ON workflow_executions(user_id);
            CREATE INDEX idx_workflow_executions_session_id ON workflow_executions(session_id);
            CREATE INDEX idx_workflow_executions_status ON workflow_executions(status);
            CREATE INDEX idx_workflow_executions_started_at ON workflow_executions(started_at);
        "#.to_string(),
        down_sql: r#"
            DROP TABLE IF EXISTS workflow_executions;
        "#.to_string(),
        dependencies: vec!["20240101_000010".to_string()],
    }
}

/// 创建步骤执行记录表
fn create_step_executions_table() -> Migration {
    Migration {
        version: "20240101_000012".to_string(),
        name: "create_step_executions_table".to_string(),
        description: "创建步骤执行记录表".to_string(),
        up_sql: r#"
            CREATE TABLE step_executions (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                workflow_execution_id UUID NOT NULL REFERENCES workflow_executions(id) ON DELETE CASCADE,
                step_name VARCHAR(255) NOT NULL,
                step_type VARCHAR(100) NOT NULL,
                agent_id UUID REFERENCES agents(id),
                input JSONB NOT NULL,
                output JSONB,
                status execution_status NOT NULL DEFAULT 'pending',
                error_message TEXT,
                error_code VARCHAR(50),
                retry_count INTEGER DEFAULT 0,
                execution_time_ms INTEGER,
                started_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                completed_at TIMESTAMPTZ,
                created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
            );

            CREATE INDEX idx_step_executions_workflow_execution_id ON step_executions(workflow_execution_id);
            CREATE INDEX idx_step_executions_agent_id ON step_executions(agent_id);
            CREATE INDEX idx_step_executions_status ON step_executions(status);
            CREATE INDEX idx_step_executions_step_name ON step_executions(step_name);
            CREATE INDEX idx_step_executions_started_at ON step_executions(started_at);
        "#.to_string(),
        down_sql: r#"
            DROP TABLE IF EXISTS step_executions;
        "#.to_string(),
        dependencies: vec!["20240101_000011".to_string()],
    }
}

/// 添加索引
fn add_indexes() -> Migration {
    Migration {
        version: "20240101_000013".to_string(),
        name: "add_indexes".to_string(),
        description: "添加性能优化索引".to_string(),
        up_sql: r#"
            -- 复合索引用于常见查询
            CREATE INDEX idx_documents_kb_status ON documents(knowledge_base_id, status);
            CREATE INDEX idx_agent_executions_agent_status ON agent_executions(agent_id, status);
            CREATE INDEX idx_workflow_executions_workflow_status ON workflow_executions(workflow_id, status);
            
            -- 时间范围查询索引
            CREATE INDEX idx_users_tenant_created ON users(tenant_id, created_at);
            CREATE INDEX idx_documents_kb_created ON documents(knowledge_base_id, created_at);
            CREATE INDEX idx_agent_executions_agent_started ON agent_executions(agent_id, started_at);
            
            -- 全文搜索索引
            CREATE INDEX idx_tenants_name_search ON tenants USING GIN(to_tsvector('chinese', name));
            CREATE INDEX idx_knowledge_bases_name_search ON knowledge_bases USING GIN(to_tsvector('chinese', name));
            CREATE INDEX idx_agents_name_search ON agents USING GIN(to_tsvector('chinese', name));
            CREATE INDEX idx_workflows_name_search ON workflows USING GIN(to_tsvector('chinese', name));
        "#.to_string(),
        down_sql: r#"
            DROP INDEX IF EXISTS idx_documents_kb_status;
            DROP INDEX IF EXISTS idx_agent_executions_agent_status;
            DROP INDEX IF EXISTS idx_workflow_executions_workflow_status;
            DROP INDEX IF EXISTS idx_users_tenant_created;
            DROP INDEX IF EXISTS idx_documents_kb_created;
            DROP INDEX IF EXISTS idx_agent_executions_agent_started;
            DROP INDEX IF EXISTS idx_tenants_name_search;
            DROP INDEX IF EXISTS idx_knowledge_bases_name_search;
            DROP INDEX IF EXISTS idx_agents_name_search;
            DROP INDEX IF EXISTS idx_workflows_name_search;
        "#.to_string(),
        dependencies: vec!["20240101_000012".to_string()],
    }
}

/// 添加约束
fn add_constraints() -> Migration {
    Migration {
        version: "20240101_000014".to_string(),
        name: "add_constraints".to_string(),
        description: "添加数据完整性约束".to_string(),
        up_sql: r#"
            -- 检查约束
            ALTER TABLE tenants ADD CONSTRAINT chk_tenants_slug_format 
                CHECK (slug ~ '^[a-z0-9][a-z0-9-]*[a-z0-9]$');
            
            ALTER TABLE users ADD CONSTRAINT chk_users_email_format 
                CHECK (email ~ '^[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}$');
            
            ALTER TABLE documents ADD CONSTRAINT chk_documents_file_size 
                CHECK (file_size_bytes >= 0);
            
            ALTER TABLE document_chunks ADD CONSTRAINT chk_chunks_positions 
                CHECK (start_position >= 0 AND end_position > start_position);
            
            ALTER TABLE embeddings ADD CONSTRAINT chk_embeddings_vector_norm 
                CHECK (vector_norm >= 0);
            
            -- 触发器：自动更新 updated_at 字段
            CREATE OR REPLACE FUNCTION update_updated_at_column()
            RETURNS TRIGGER AS $$
            BEGIN
                NEW.updated_at = CURRENT_TIMESTAMP;
                RETURN NEW;
            END;
            $$ language 'plpgsql';

            CREATE TRIGGER update_tenants_updated_at BEFORE UPDATE ON tenants
                FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
            
            CREATE TRIGGER update_users_updated_at BEFORE UPDATE ON users
                FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
            
            CREATE TRIGGER update_knowledge_bases_updated_at BEFORE UPDATE ON knowledge_bases
                FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
            
            CREATE TRIGGER update_documents_updated_at BEFORE UPDATE ON documents
                FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
            
            CREATE TRIGGER update_agents_updated_at BEFORE UPDATE ON agents
                FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
            
            CREATE TRIGGER update_workflows_updated_at BEFORE UPDATE ON workflows
                FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
        "#.to_string(),
        down_sql: r#"
            -- 删除触发器
            DROP TRIGGER IF EXISTS update_tenants_updated_at ON tenants;
            DROP TRIGGER IF EXISTS update_users_updated_at ON users;
            DROP TRIGGER IF EXISTS update_knowledge_bases_updated_at ON knowledge_bases;
            DROP TRIGGER IF EXISTS update_documents_updated_at ON documents;
            DROP TRIGGER IF EXISTS update_agents_updated_at ON agents;
            DROP TRIGGER IF EXISTS update_workflows_updated_at ON workflows;
            
            DROP FUNCTION IF EXISTS update_updated_at_column();
            
            -- 删除约束
            ALTER TABLE tenants DROP CONSTRAINT IF EXISTS chk_tenants_slug_format;
            ALTER TABLE users DROP CONSTRAINT IF EXISTS chk_users_email_format;
            ALTER TABLE documents DROP CONSTRAINT IF EXISTS chk_documents_file_size;
            ALTER TABLE document_chunks DROP CONSTRAINT IF EXISTS chk_chunks_positions;
            ALTER TABLE embeddings DROP CONSTRAINT IF EXISTS chk_embeddings_vector_norm;
        "#.to_string(),
        dependencies: vec!["20240101_000013".to_string()],
    }
}