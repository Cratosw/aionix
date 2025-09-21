// 数据库迁移管理
// 处理数据库架构迁移和版本控制

use crate::db::DatabaseManager;
use crate::errors::AiStudioError;
use sea_orm::{Statement, ConnectionTrait};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{info, warn, instrument};

/// 迁移状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationStatus {
    pub version: String,
    pub name: String,
    pub applied_at: Option<chrono::DateTime<chrono::Utc>>,
    pub is_applied: bool,
}

/// 迁移管理器
pub struct MigrationManager;

impl MigrationManager {
    /// 初始化迁移系统
    #[instrument]
    pub async fn init() -> Result<(), AiStudioError> {
        info!("初始化数据库迁移系统");
        
        let db_manager = DatabaseManager::get()?;
        let connection = db_manager.get_connection();

        // 创建迁移表
        let create_migrations_table = "
            CREATE TABLE IF NOT EXISTS seaql_migrations (
                version VARCHAR(255) PRIMARY KEY,
                name VARCHAR(255) NOT NULL,
                applied_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
            )
        ";

        connection.execute(Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            create_migrations_table.to_string(),
        )).await.map_err(|e| AiStudioError::database(format!("创建迁移表失败: {}", e)))?;

        info!("迁移系统初始化完成");
        Ok(())
    }

    /// 获取已应用的迁移
    #[instrument]
    pub async fn get_applied_migrations() -> Result<Vec<MigrationStatus>, AiStudioError> {
        let db_manager = DatabaseManager::get()?;
        let connection = db_manager.get_connection();

        let query = "SELECT version, name, applied_at FROM seaql_migrations ORDER BY applied_at";
        let results = connection.query_all(Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            query.to_string(),
        )).await.map_err(|e| AiStudioError::database(format!("查询迁移记录失败: {}", e)))?;

        let mut migrations = Vec::new();
        for row in results {
            let version: String = row.try_get("", "version")
                .map_err(|e| AiStudioError::database(format!("解析迁移版本失败: {}", e)))?;
            let name: String = row.try_get("", "name")
                .map_err(|e| AiStudioError::database(format!("解析迁移名称失败: {}", e)))?;
            let applied_at: chrono::DateTime<chrono::Utc> = row.try_get("", "applied_at")
                .map_err(|e| AiStudioError::database(format!("解析迁移时间失败: {}", e)))?;

            migrations.push(MigrationStatus {
                version,
                name,
                applied_at: Some(applied_at),
                is_applied: true,
            });
        }

        Ok(migrations)
    }

    /// 检查迁移状态
    #[instrument]
    pub async fn check_migration_status() -> Result<Vec<MigrationStatus>, AiStudioError> {
        info!("检查数据库迁移状态");

        // 获取已应用的迁移
        let applied_migrations = Self::get_applied_migrations().await?;
        let applied_versions: HashMap<String, MigrationStatus> = applied_migrations
            .into_iter()
            .map(|m| (m.version.clone(), m))
            .collect();

        // 获取所有可用的迁移（这里需要根据实际的迁移文件来实现）
        let available_migrations = Self::get_available_migrations();
        
        let mut status = Vec::new();
        for migration in available_migrations {
            if let Some(applied) = applied_versions.get(&migration.version) {
                status.push(applied.clone());
            } else {
                status.push(MigrationStatus {
                    version: migration.version,
                    name: migration.name,
                    applied_at: None,
                    is_applied: false,
                });
            }
        }

        Ok(status)
    }

    /// 应用待处理的迁移
    #[instrument]
    pub async fn apply_pending_migrations() -> Result<Vec<String>, AiStudioError> {
        info!("应用待处理的数据库迁移");

        let migration_status = Self::check_migration_status().await?;
        let mut applied_migrations = Vec::new();

        for migration in migration_status {
            if !migration.is_applied {
                info!(version = %migration.version, name = %migration.name, "应用迁移");
                
                // 这里需要根据实际的迁移内容来执行
                // 目前只是记录到迁移表中
                Self::record_migration(&migration.version, &migration.name).await?;
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

    /// 记录迁移到数据库
    #[instrument(skip(version, name))]
    async fn record_migration(version: &str, name: &str) -> Result<(), AiStudioError> {
        let db_manager = DatabaseManager::get()?;
        let connection = db_manager.get_connection();

        let insert_query = format!(
            "INSERT INTO seaql_migrations (version, name) VALUES ('{}', '{}')",
            version, name
        );

        connection.execute(Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            insert_query,
        )).await.map_err(|e| AiStudioError::database(format!("记录迁移失败: {}", e)))?;

        Ok(())
    }

    /// 获取可用的迁移列表（硬编码，实际应该从文件系统读取）
    fn get_available_migrations() -> Vec<MigrationInfo> {
        vec![
            MigrationInfo {
                version: "20240101_000001".to_string(),
                name: "create_tenants_table".to_string(),
            },
            MigrationInfo {
                version: "20240101_000002".to_string(),
                name: "create_users_table".to_string(),
            },
            MigrationInfo {
                version: "20240101_000003".to_string(),
                name: "create_sessions_table".to_string(),
            },
            MigrationInfo {
                version: "20240101_000004".to_string(),
                name: "create_knowledge_bases_table".to_string(),
            },
            MigrationInfo {
                version: "20240101_000005".to_string(),
                name: "create_documents_table".to_string(),
            },
            MigrationInfo {
                version: "20240101_000006".to_string(),
                name: "create_document_chunks_table".to_string(),
            },
            MigrationInfo {
                version: "20240101_000007".to_string(),
                name: "create_embeddings_table".to_string(),
            },
        ]
    }

    /// 回滚迁移
    #[instrument(skip(version))]
    pub async fn rollback_migration(version: &str) -> Result<(), AiStudioError> {
        warn!(version = %version, "回滚数据库迁移");

        let db_manager = DatabaseManager::get()?;
        let connection = db_manager.get_connection();

        // 删除迁移记录
        let delete_query = format!(
            "DELETE FROM seaql_migrations WHERE version = '{}'",
            version
        );

        connection.execute(Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            delete_query,
        )).await.map_err(|e| AiStudioError::database(format!("删除迁移记录失败: {}", e)))?;

        // 这里需要执行实际的回滚 SQL
        // 目前只是删除记录

        info!(version = %version, "迁移回滚完成");
        Ok(())
    }

    /// 重置所有迁移
    #[instrument]
    pub async fn reset_migrations() -> Result<(), AiStudioError> {
        warn!("重置所有数据库迁移");

        let db_manager = DatabaseManager::get()?;
        let connection = db_manager.get_connection();

        // 清空迁移表
        let truncate_query = "TRUNCATE TABLE seaql_migrations";
        connection.execute(Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            truncate_query.to_string(),
        )).await.map_err(|e| AiStudioError::database(format!("清空迁移表失败: {}", e)))?;

        info!("迁移重置完成");
        Ok(())
    }

    /// 验证数据库架构
    #[instrument]
    pub async fn validate_schema() -> Result<SchemaValidation, AiStudioError> {
        info!("验证数据库架构");

        let db_manager = DatabaseManager::get()?;
        let connection = db_manager.get_connection();

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
            "knowledge_bases", "documents", "document_chunks", "embeddings"
        ];

        for table_name in required_tables {
            let table_exists_query = format!(
                "SELECT EXISTS (SELECT FROM information_schema.tables WHERE table_name = '{}')",
                table_name
            );

            match connection.query_one(Statement::from_string(
                sea_orm::DatabaseBackend::Postgres,
                table_exists_query,
            )).await {
                Ok(Some(row)) => {
                    let exists: bool = row.try_get("", "exists").unwrap_or(false);
                    if !exists {
                        validation.missing_tables.push(table_name.to_string());
                        validation.is_valid = false;
                    }
                }
                Ok(None) => {
                    validation.errors.push(format!("检查表 {} 时返回空结果", table_name));
                    validation.is_valid = false;
                }
                Err(e) => {
                    validation.errors.push(format!("检查表 {} 时出错: {}", table_name, e));
                    validation.is_valid = false;
                }
            }
        }

        if validation.is_valid {
            info!("数据库架构验证通过");
        } else {
            warn!(
                missing_tables = ?validation.missing_tables,
                errors = ?validation.errors,
                "数据库架构验证失败"
            );
        }

        Ok(validation)
    }
}

/// 迁移信息
#[derive(Debug, Clone)]
struct MigrationInfo {
    version: String,
    name: String,
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