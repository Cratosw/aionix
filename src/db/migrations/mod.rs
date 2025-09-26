// 数据库迁移模块
// 包含所有迁移脚本和管理功能

use crate::errors::AiStudioError;
use sea_orm::{DatabaseConnection, Statement, ConnectionTrait, TransactionTrait};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{info, warn, instrument};

pub mod migrations;
pub mod seed_data;
pub mod backup;
pub mod tenant_filter;

pub use migrations::*;
pub use seed_data::*;
pub use backup::*;
pub use tenant_filter::*;

/// 迁移信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Migration {
    pub version: String,
    pub name: String,
    pub description: String,
    pub up_sql: String,
    pub down_sql: String,
    pub dependencies: Vec<String>,
}

/// 迁移状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationStatus {
    pub version: String,
    pub name: String,
    pub applied_at: Option<chrono::DateTime<chrono::Utc>>,
    pub is_applied: bool,
    pub checksum: String,
}

/// 迁移管理器
pub struct MigrationManager {
    db: DatabaseConnection,
}

impl MigrationManager {
    /// 创建新的迁移管理器
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// 初始化迁移系统
    #[instrument(skip(self))]
    pub async fn init(&self) -> Result<(), AiStudioError> {
        info!("初始化数据库迁移系统");

        // 创建迁移表
        let create_migrations_table = r#"
            CREATE TABLE IF NOT EXISTS schema_migrations (
                version VARCHAR(255) PRIMARY KEY,
                name VARCHAR(255) NOT NULL,
                description TEXT,
                checksum VARCHAR(64) NOT NULL,
                applied_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
                execution_time_ms INTEGER DEFAULT 0
            );

            CREATE INDEX IF NOT EXISTS idx_schema_migrations_applied_at 
            ON schema_migrations(applied_at);
        "#;

        self.execute_sql(create_migrations_table).await?;

        // 创建 pgvector 扩展（如果需要）
        let create_vector_extension = r#"
            CREATE EXTENSION IF NOT EXISTS vector;
        "#;

        if let Err(e) = self.execute_sql(create_vector_extension).await {
            warn!("创建 pgvector 扩展失败，可能需要手动安装: {}", e);
        }

        info!("迁移系统初始化完成");
        Ok(())
    }

    /// 获取所有可用的迁移
    pub fn get_available_migrations(&self) -> Vec<Migration> {
        migrations::get_all_migrations()
    }

    /// 获取已应用的迁移
    #[instrument(skip(self))]
    pub async fn get_applied_migrations(&self) -> Result<Vec<MigrationStatus>, AiStudioError> {
        let query = r#"
            SELECT version, name, applied_at, checksum 
            FROM schema_migrations 
            ORDER BY applied_at
        "#;

        let results = self.db.query_all(Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            query.to_string(),
        )).await?;

        let mut migrations = Vec::new();
        for row in results {
            let version: String = row.try_get("", "version")?;
            let name: String = row.try_get("", "name")?;
            let applied_at: chrono::DateTime<chrono::Utc> = row.try_get("", "applied_at")?;
            let checksum: String = row.try_get("", "checksum")?;

            migrations.push(MigrationStatus {
                version,
                name,
                applied_at: Some(applied_at),
                is_applied: true,
                checksum,
            });
        }

        Ok(migrations)
    }

    /// 检查迁移状态
    #[instrument(skip(self))]
    pub async fn check_status(&self) -> Result<Vec<MigrationStatus>, AiStudioError> {
        info!("检查数据库迁移状态");

        let applied_migrations = self.get_applied_migrations().await?;
        let applied_versions: HashMap<String, MigrationStatus> = applied_migrations
            .into_iter()
            .map(|m| (m.version.clone(), m))
            .collect();

        let available_migrations = self.get_available_migrations();
        let mut status = Vec::new();

        for migration in available_migrations {
            let checksum = self.calculate_checksum(&migration);
            
            if let Some(applied) = applied_versions.get(&migration.version) {
                // 检查校验和是否匹配
                if applied.checksum != checksum {
                    warn!(
                        version = %migration.version,
                        "迁移校验和不匹配，可能已被修改"
                    );
                }
                status.push(applied.clone());
            } else {
                status.push(MigrationStatus {
                    version: migration.version,
                    name: migration.name,
                    applied_at: None,
                    is_applied: false,
                    checksum,
                });
            }
        }

        Ok(status)
    }

    /// 应用待处理的迁移
    #[instrument(skip(self))]
    pub async fn migrate(&self) -> Result<Vec<String>, AiStudioError> {
        info!("开始应用数据库迁移");

        let status = self.check_status().await?;
        let mut applied_migrations = Vec::new();

        for migration_status in status {
            if !migration_status.is_applied {
                let migration = self.get_available_migrations()
                    .into_iter()
                    .find(|m| m.version == migration_status.version)
                    .ok_or_else(|| AiStudioError::internal(
                        format!("找不到迁移: {}", migration_status.version)
                    ))?;

                self.apply_migration(&migration).await?;
                applied_migrations.push(migration.version);
            }
        }

        if applied_migrations.is_empty() {
            info!("没有待处理的迁移");
        } else {
            info!(count = applied_migrations.len(), "迁移应用完成");
        }

        Ok(applied_migrations)
    }

    /// 应用单个迁移
    #[instrument(skip(self, migration))]
    async fn apply_migration(&self, migration: &Migration) -> Result<(), AiStudioError> {
        info!(
            version = %migration.version,
            name = %migration.name,
            "应用迁移"
        );

        let start_time = std::time::Instant::now();

        // 开始事务
        let txn = self.db.begin().await?;

        // 执行迁移 SQL
        if let Err(e) = self.execute_sql_in_txn(&txn, &migration.up_sql).await {
            txn.rollback().await?;
            return Err(AiStudioError::database(
                format!("迁移 {} 执行失败: {}", migration.version, e)
            ));
        }

        // 记录迁移
        let checksum = self.calculate_checksum(migration);
        let execution_time = start_time.elapsed().as_millis() as i32;

        let record_sql = format!(
            r#"
            INSERT INTO schema_migrations (version, name, description, checksum, execution_time_ms)
            VALUES ('{}', '{}', '{}', '{}', {})
            "#,
            migration.version,
            migration.name,
            migration.description,
            checksum,
            execution_time
        );

        if let Err(e) = self.execute_sql_in_txn(&txn, &record_sql).await {
            txn.rollback().await?;
            return Err(AiStudioError::database(
                format!("记录迁移 {} 失败: {}", migration.version, e)
            ));
        }

        // 提交事务
        txn.commit().await?;

        info!(
            version = %migration.version,
            execution_time_ms = execution_time,
            "迁移应用成功"
        );

        Ok(())
    }

    /// 回滚迁移
    #[instrument(skip(self))]
    pub async fn rollback(&self, version: &str) -> Result<(), AiStudioError> {
        warn!(version = %version, "回滚数据库迁移");

        let migration = self.get_available_migrations()
            .into_iter()
            .find(|m| m.version == version)
            .ok_or_else(|| AiStudioError::not_found("迁移"))?;

        // 开始事务
        let txn = self.db.begin().await?;

        // 执行回滚 SQL
        if let Err(e) = self.execute_sql_in_txn(&txn, &migration.down_sql).await {
            txn.rollback().await?;
            return Err(AiStudioError::database(
                format!("迁移 {} 回滚失败: {}", version, e)
            ));
        }

        // 删除迁移记录
        let delete_sql = format!(
            "DELETE FROM schema_migrations WHERE version = '{}'",
            version
        );

        if let Err(e) = self.execute_sql_in_txn(&txn, &delete_sql).await {
            txn.rollback().await?;
            return Err(AiStudioError::database(
                format!("删除迁移记录 {} 失败: {}", version, e)
            ));
        }

        // 提交事务
        txn.commit().await?;

        info!(version = %version, "迁移回滚完成");
        Ok(())
    }

    /// 验证数据库架构
    #[instrument(skip(self))]
    pub async fn validate_schema(&self) -> Result<SchemaValidation, AiStudioError> {
        info!("验证数据库架构");

        let mut validation = SchemaValidation {
            is_valid: true,
            missing_tables: Vec::new(),
            missing_columns: Vec::new(),
            missing_indexes: Vec::new(),
            errors: Vec::new(),
        };

        // 检查必需的表
        let required_tables = vec![
            "tenants", "users", "sessions",
            "knowledge_bases", "documents", "document_chunks", "embeddings",
            "agents", "agent_executions", "workflows", "workflow_executions", "step_executions"
        ];

        for table_name in required_tables {
            if !self.table_exists(table_name).await? {
                validation.missing_tables.push(table_name.to_string());
                validation.is_valid = false;
            }
        }

        // 检查 pgvector 扩展
        if !self.extension_exists("vector").await? {
            validation.errors.push("pgvector 扩展未安装".to_string());
            validation.is_valid = false;
        }

        if validation.is_valid {
            info!("数据库架构验证通过");
        } else {
            warn!("数据库架构验证失败: {:?}", validation);
        }

        Ok(validation)
    }

    /// 执行 SQL 语句
    async fn execute_sql(&self, sql: &str) -> Result<(), AiStudioError> {
        // 分割 SQL 语句，逐条执行
        let statements: Vec<&str> = sql
            .split(';')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();

        for statement in statements {
            if !statement.is_empty() {
                self.db.execute(Statement::from_string(
                    sea_orm::DatabaseBackend::Postgres,
                    statement.to_string(),
                )).await?;
            }
        }
        Ok(())
    }

    /// 在事务中执行 SQL 语句
    async fn execute_sql_in_txn(
        &self,
        txn: &sea_orm::DatabaseTransaction,
        sql: &str,
    ) -> Result<(), AiStudioError> {
        // 分割 SQL 语句，逐条执行
        let statements: Vec<&str> = sql
            .split(';')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();

        for statement in statements {
            if !statement.is_empty() {
                txn.execute(Statement::from_string(
                    sea_orm::DatabaseBackend::Postgres,
                    statement.to_string(),
                )).await?;
            }
        }
        Ok(())
    }

    /// 检查表是否存在
    async fn table_exists(&self, table_name: &str) -> Result<bool, AiStudioError> {
        let query = format!(
            "SELECT EXISTS (SELECT FROM information_schema.tables WHERE table_name = '{}')",
            table_name
        );

        let result = self.db.query_one(Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            query,
        )).await?;

        if let Some(row) = result {
            Ok(row.try_get("", "exists").unwrap_or(false))
        } else {
            Ok(false)
        }
    }

    /// 检查扩展是否存在
    async fn extension_exists(&self, extension_name: &str) -> Result<bool, AiStudioError> {
        let query = format!(
            "SELECT EXISTS (SELECT FROM pg_extension WHERE extname = '{}')",
            extension_name
        );

        let result = self.db.query_one(Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            query,
        )).await?;

        if let Some(row) = result {
            Ok(row.try_get("", "exists").unwrap_or(false))
        } else {
            Ok(false)
        }
    }

    /// 计算迁移校验和
    fn calculate_checksum(&self, migration: &Migration) -> String {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(migration.up_sql.as_bytes());
        hasher.update(migration.down_sql.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}

/// 架构验证结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaValidation {
    pub is_valid: bool,
    pub missing_tables: Vec<String>,
    pub missing_columns: Vec<String>,
    pub missing_indexes: Vec<String>,
    pub errors: Vec<String>,
}