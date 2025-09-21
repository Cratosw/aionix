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
                display_nam