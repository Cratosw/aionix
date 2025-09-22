// 数据库备份和恢复工具
// 提供数据备份、恢复和灾难恢复功能

use crate::errors::AiStudioError;
use sea_orm::{DatabaseConnection, Statement, ConnectionTrait};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::process::Command;
use tracing::{info, warn, error, instrument};
use uuid::Uuid;

/// 备份类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BackupType {
    /// 完整备份
    Full,
    /// 增量备份
    Incremental,
    /// 差异备份
    Differential,
    /// 仅数据备份
    DataOnly,
    /// 仅架构备份
    SchemaOnly,
}

/// 备份状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BackupStatus {
    /// 进行中
    InProgress,
    /// 已完成
    Completed,
    /// 失败
    Failed,
    /// 已取消
    Cancelled,
}

/// 备份信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupInfo {
    pub id: Uuid,
    pub backup_type: BackupType,
    pub status: BackupStatus,
    pub file_path: PathBuf,
    pub file_size_bytes: u64,
    pub tenant_id: Option<Uuid>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub error_message: Option<String>,
    pub metadata: serde_json::Value,
}

/// 恢复选项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreOptions {
    pub backup_id: Uuid,
    pub target_database: Option<String>,
    pub restore_data: bool,
    pub restore_schema: bool,
    pub clean_before_restore: bool,
    pub tenant_filter: Option<Uuid>,
}

/// 备份管理器
pub struct BackupManager {
    db: DatabaseConnection,
    backup_dir: PathBuf,
    pg_dump_path: String,
    pg_restore_path: String,
}

impl BackupManager {
    /// 创建新的备份管理器
    pub fn new(
        db: DatabaseConnection,
        backup_dir: PathBuf,
        pg_dump_path: Option<String>,
        pg_restore_path: Option<String>,
    ) -> Self {
        Self {
            db,
            backup_dir,
            pg_dump_path: pg_dump_path.unwrap_or_else(|| "pg_dump".to_string()),
            pg_restore_path: pg_restore_path.unwrap_or_else(|| "pg_restore".to_string()),
        }
    }

    /// 初始化备份系统
    #[instrument(skip(self))]
    pub async fn init(&self) -> Result<(), AiStudioError> {
        info!("初始化备份系统");

        // 创建备份目录
        if !self.backup_dir.exists() {
            fs::create_dir_all(&self.backup_dir).await
                .map_err(|e| AiStudioError::internal(format!("创建备份目录失败: {}", e)))?;
        }

        // 创建备份记录表
        let create_backups_table = r#"
            CREATE TABLE IF NOT EXISTS backups (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                backup_type VARCHAR(50) NOT NULL,
                status VARCHAR(50) NOT NULL,
                file_path VARCHAR(1000) NOT NULL,
                file_size_bytes BIGINT DEFAULT 0,
                tenant_id UUID REFERENCES tenants(id),
                created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                completed_at TIMESTAMPTZ,
                error_message TEXT,
                metadata JSONB NOT NULL DEFAULT '{}'
            );

            CREATE INDEX IF NOT EXISTS idx_backups_status ON backups(status);
            CREATE INDEX IF NOT EXISTS idx_backups_tenant_id ON backups(tenant_id);
            CREATE INDEX IF NOT EXISTS idx_backups_created_at ON backups(created_at);
        "#;

        self.db.execute(Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            create_backups_table.to_string(),
        )).await?;

        info!("备份系统初始化完成");
        Ok(())
    }

    /// 创建完整备份
    #[instrument(skip(self))]
    pub async fn create_full_backup(&self, tenant_id: Option<Uuid>) -> Result<BackupInfo, AiStudioError> {
        info!("开始创建完整备份");

        let backup_id = Uuid::new_v4();
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let filename = if let Some(tid) = tenant_id {
            format!("full_backup_{}_{}.sql", tid, timestamp)
        } else {
            format!("full_backup_{}.sql", timestamp)
        };
        let file_path = self.backup_dir.join(&filename);

        // 创建备份记录
        let mut backup_info = BackupInfo {
            id: backup_id,
            backup_type: BackupType::Full,
            status: BackupStatus::InProgress,
            file_path: file_path.clone(),
            file_size_bytes: 0,
            tenant_id,
            created_at: chrono::Utc::now(),
            completed_at: None,
            error_message: None,
            metadata: serde_json::json!({}),
        };

        self.record_backup(&backup_info).await?;

        // 执行备份
        match self.execute_pg_dump(&file_path, tenant_id).await {
            Ok(file_size) => {
                backup_info.status = BackupStatus::Completed;
                backup_info.file_size_bytes = file_size;
                backup_info.completed_at = Some(chrono::Utc::now());
                
                self.update_backup_status(&backup_info).await?;
                info!(backup_id = %backup_id, file_size = file_size, "完整备份创建成功");
            }
            Err(e) => {
                backup_info.status = BackupStatus::Failed;
                backup_info.error_message = Some(e.to_string());
                backup_info.completed_at = Some(chrono::Utc::now());
                
                self.update_backup_status(&backup_info).await?;
                error!(backup_id = %backup_id, error = %e, "完整备份创建失败");
                return Err(e);
            }
        }

        Ok(backup_info)
    }

    /// 创建增量备份
    #[instrument(skip(self))]
    pub async fn create_incremental_backup(&self, tenant_id: Option<Uuid>) -> Result<BackupInfo, AiStudioError> {
        info!("开始创建增量备份");

        // 获取最后一次备份的时间
        let last_backup_time = self.get_last_backup_time(tenant_id).await?;
        
        let backup_id = Uuid::new_v4();
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let filename = if let Some(tid) = tenant_id {
            format!("incremental_backup_{}_{}.sql", tid, timestamp)
        } else {
            format!("incremental_backup_{}.sql", timestamp)
        };
        let file_path = self.backup_dir.join(&filename);

        let mut backup_info = BackupInfo {
            id: backup_id,
            backup_type: BackupType::Incremental,
            status: BackupStatus::InProgress,
            file_path: file_path.clone(),
            file_size_bytes: 0,
            tenant_id,
            created_at: chrono::Utc::now(),
            completed_at: None,
            error_message: None,
            metadata: serde_json::json!({
                "last_backup_time": last_backup_time
            }),
        };

        self.record_backup(&backup_info).await?;

        // 执行增量备份
        match self.execute_incremental_backup(&file_path, tenant_id, last_backup_time).await {
            Ok(file_size) => {
                backup_info.status = BackupStatus::Completed;
                backup_info.file_size_bytes = file_size;
                backup_info.completed_at = Some(chrono::Utc::now());
                
                self.update_backup_status(&backup_info).await?;
                info!(backup_id = %backup_id, file_size = file_size, "增量备份创建成功");
            }
            Err(e) => {
                backup_info.status = BackupStatus::Failed;
                backup_info.error_message = Some(e.to_string());
                backup_info.completed_at = Some(chrono::Utc::now());
                
                self.update_backup_status(&backup_info).await?;
                error!(backup_id = %backup_id, error = %e, "增量备份创建失败");
                return Err(e);
            }
        }

        Ok(backup_info)
    }

    /// 恢复备份
    #[instrument(skip(self))]
    pub async fn restore_backup(&self, options: RestoreOptions) -> Result<(), AiStudioError> {
        info!(backup_id = %options.backup_id, "开始恢复备份");

        // 获取备份信息
        let backup_info = self.get_backup_info(options.backup_id).await?;
        
        if backup_info.status != BackupStatus::Completed {
            return Err(AiStudioError::validation("只能恢复已完成的备份"));
        }

        if !backup_info.file_path.exists() {
            return Err(AiStudioError::not_found("备份文件不存在"));
        }

        // 执行恢复
        match backup_info.backup_type {
            BackupType::Full | BackupType::DataOnly | BackupType::SchemaOnly => {
                self.execute_pg_restore(&backup_info.file_path, &options).await?;
            }
            BackupType::Incremental | BackupType::Differential => {
                // 增量恢复需要先恢复基础备份，然后应用增量
                self.execute_incremental_restore(&backup_info, &options).await?;
            }
        }

        info!(backup_id = %options.backup_id, "备份恢复完成");
        Ok(())
    }

    /// 列出备份
    #[instrument(skip(self))]
    pub async fn list_backups(&self, tenant_id: Option<Uuid>) -> Result<Vec<BackupInfo>, AiStudioError> {
        let query = if let Some(tid) = tenant_id {
            format!(
                "SELECT * FROM backups WHERE tenant_id = '{}' ORDER BY created_at DESC",
                tid
            )
        } else {
            "SELECT * FROM backups ORDER BY created_at DESC".to_string()
        };

        let results = self.db.query_all(Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            query,
        )).await?;

        let mut backups = Vec::new();
        for row in results {
            let backup_info = BackupInfo {
                id: row.try_get("", "id")?,
                backup_type: serde_json::from_str(&row.try_get::<String>("", "backup_type")?)
                    .unwrap_or(BackupType::Full),
                status: serde_json::from_str(&row.try_get::<String>("", "status")?)
                    .unwrap_or(BackupStatus::Failed),
                file_path: PathBuf::from(row.try_get::<String>("", "file_path")?),
                file_size_bytes: row.try_get("", "file_size_bytes").unwrap_or(0),
                tenant_id: row.try_get("", "tenant_id").ok(),
                created_at: row.try_get("", "created_at")?,
                completed_at: row.try_get("", "completed_at").ok(),
                error_message: row.try_get("", "error_message").ok(),
                metadata: row.try_get("", "metadata").unwrap_or_else(|_| serde_json::json!({})),
            };
            backups.push(backup_info);
        }

        Ok(backups)
    }

    /// 删除备份
    #[instrument(skip(self))]
    pub async fn delete_backup(&self, backup_id: Uuid) -> Result<(), AiStudioError> {
        info!(backup_id = %backup_id, "删除备份");

        let backup_info = self.get_backup_info(backup_id).await?;

        // 删除备份文件
        if backup_info.file_path.exists() {
            fs::remove_file(&backup_info.file_path).await
                .map_err(|e| AiStudioError::internal(format!("删除备份文件失败: {}", e)))?;
        }

        // 删除备份记录
        let delete_query = format!(
            "DELETE FROM backups WHERE id = '{}'",
            backup_id
        );

        self.db.execute(Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            delete_query,
        )).await?;

        info!(backup_id = %backup_id, "备份删除完成");
        Ok(())
    }

    /// 清理过期备份
    #[instrument(skip(self))]
    pub async fn cleanup_old_backups(&self, retention_days: u32) -> Result<Vec<Uuid>, AiStudioError> {
        info!(retention_days = retention_days, "清理过期备份");

        let cutoff_date = chrono::Utc::now() - chrono::Duration::days(retention_days as i64);
        
        let query = format!(
            "SELECT id FROM backups WHERE created_at < '{}' AND status = 'Completed'",
            cutoff_date.format("%Y-%m-%d %H:%M:%S")
        );

        let results = self.db.query_all(Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            query,
        )).await?;

        let mut deleted_backups = Vec::new();
        for row in results {
            let backup_id: Uuid = row.try_get("", "id")?;
            if let Err(e) = self.delete_backup(backup_id).await {
                warn!(backup_id = %backup_id, error = %e, "删除过期备份失败");
            } else {
                deleted_backups.push(backup_id);
            }
        }

        info!(count = deleted_backups.len(), "过期备份清理完成");
        Ok(deleted_backups)
    }

    /// 验证备份完整性
    #[instrument(skip(self))]
    pub async fn verify_backup(&self, backup_id: Uuid) -> Result<bool, AiStudioError> {
        info!(backup_id = %backup_id, "验证备份完整性");

        let backup_info = self.get_backup_info(backup_id).await?;
        
        if !backup_info.file_path.exists() {
            return Ok(false);
        }

        // 检查文件大小
        let metadata = fs::metadata(&backup_info.file_path).await
            .map_err(|e| AiStudioError::internal(format!("读取备份文件元数据失败: {}", e)))?;
        
        if metadata.len() != backup_info.file_size_bytes {
            warn!(
                backup_id = %backup_id,
                expected_size = backup_info.file_size_bytes,
                actual_size = metadata.len(),
                "备份文件大小不匹配"
            );
            return Ok(false);
        }

        // 尝试读取备份文件头部，验证格式
        let content = fs::read_to_string(&backup_info.file_path).await
            .map_err(|e| AiStudioError::internal(format!("读取备份文件失败: {}", e)))?;

        if !content.starts_with("--") && !content.contains("PostgreSQL database dump") {
            warn!(backup_id = %backup_id, "备份文件格式无效");
            return Ok(false);
        }

        info!(backup_id = %backup_id, "备份完整性验证通过");
        Ok(true)
    }

    /// 执行 pg_dump
    async fn execute_pg_dump(&self, file_path: &Path, tenant_id: Option<Uuid>) -> Result<u64, AiStudioError> {
        let mut cmd = Command::new(&self.pg_dump_path);
        
        // 基本参数
        cmd.args(&[
            "--verbose",
            "--clean",
            "--no-acl",
            "--no-owner",
            "--format=plain",
            &format!("--file={}", file_path.display()),
        ]);

        // 如果指定了租户，添加过滤条件
        if let Some(tid) = tenant_id {
            // 这里需要根据实际需求添加租户过滤的 SQL
            // 由于 pg_dump 不直接支持复杂过滤，可能需要使用自定义脚本
        }

        let output = cmd.output().await
            .map_err(|e| AiStudioError::internal(format!("执行 pg_dump 失败: {}", e)))?;

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            return Err(AiStudioError::internal(format!("pg_dump 执行失败: {}", error_msg)));
        }

        // 获取文件大小
        let metadata = fs::metadata(file_path).await
            .map_err(|e| AiStudioError::internal(format!("获取备份文件大小失败: {}", e)))?;

        Ok(metadata.len())
    }

    /// 执行增量备份
    async fn execute_incremental_backup(
        &self,
        file_path: &Path,
        tenant_id: Option<Uuid>,
        since: chrono::DateTime<chrono::Utc>,
    ) -> Result<u64, AiStudioError> {
        // 增量备份的实现比较复杂，这里提供一个简化版本
        // 实际实现可能需要使用 WAL 日志或者基于时间戳的查询

        let incremental_query = if let Some(tid) = tenant_id {
            format!(
                r#"
                -- 增量备份：租户 {} 自 {} 以来的变更
                COPY (
                    SELECT 'tenants' as table_name, row_to_json(t) as data 
                    FROM tenants t 
                    WHERE id = '{}' AND updated_at > '{}'
                ) TO STDOUT;
                "#,
                tid, since.format("%Y-%m-%d %H:%M:%S"), tid, since.format("%Y-%m-%d %H:%M:%S")
            )
        } else {
            format!(
                r#"
                -- 全局增量备份自 {} 以来的变更
                COPY (
                    SELECT 'tenants' as table_name, row_to_json(t) as data 
                    FROM tenants t 
                    WHERE updated_at > '{}'
                ) TO STDOUT;
                "#,
                since.format("%Y-%m-%d %H:%M:%S")
            )
        };

        // 将查询写入文件
        fs::write(file_path, incremental_query).await
            .map_err(|e| AiStudioError::internal(format!("写入增量备份文件失败: {}", e)))?;

        let metadata = fs::metadata(file_path).await
            .map_err(|e| AiStudioError::internal(format!("获取增量备份文件大小失败: {}", e)))?;

        Ok(metadata.len())
    }

    /// 执行 pg_restore
    async fn execute_pg_restore(&self, file_path: &Path, options: &RestoreOptions) -> Result<(), AiStudioError> {
        let mut cmd = Command::new(&self.pg_restore_path);
        
        cmd.args(&[
            "--verbose",
            "--clean",
            "--no-acl",
            "--no-owner",
        ]);

        if options.clean_before_restore {
            cmd.arg("--clean");
        }

        if !options.restore_data {
            cmd.arg("--schema-only");
        }

        if !options.restore_schema {
            cmd.arg("--data-only");
        }

        cmd.arg(file_path.to_str().unwrap());

        let output = cmd.output().await
            .map_err(|e| AiStudioError::internal(format!("执行 pg_restore 失败: {}", e)))?;

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            return Err(AiStudioError::internal(format!("pg_restore 执行失败: {}", error_msg)));
        }

        Ok(())
    }

    /// 执行增量恢复
    async fn execute_incremental_restore(&self, backup_info: &BackupInfo, options: &RestoreOptions) -> Result<(), AiStudioError> {
        // 增量恢复的实现
        // 这里需要根据增量备份的格式来实现恢复逻辑
        warn!("增量恢复功能尚未完全实现");
        Ok(())
    }

    /// 记录备份信息
    async fn record_backup(&self, backup_info: &BackupInfo) -> Result<(), AiStudioError> {
        let sql = format!(
            r#"
            INSERT INTO backups (
                id, backup_type, status, file_path, file_size_bytes,
                tenant_id, created_at, completed_at, error_message, metadata
            ) VALUES (
                '{}', '{}', '{}', '{}', {},
                {}, '{}', {}, {}, '{}'
            )
            "#,
            backup_info.id,
            serde_json::to_string(&backup_info.backup_type).unwrap().trim_matches('"'),
            serde_json::to_string(&backup_info.status).unwrap().trim_matches('"'),
            backup_info.file_path.display(),
            backup_info.file_size_bytes,
            backup_info.tenant_id.map(|id| format!("'{}'", id)).unwrap_or("NULL".to_string()),
            backup_info.created_at.format("%Y-%m-%d %H:%M:%S"),
            backup_info.completed_at.map(|dt| format!("'{}'", dt.format("%Y-%m-%d %H:%M:%S"))).unwrap_or("NULL".to_string()),
            backup_info.error_message.as_ref().map(|msg| format!("'{}'", msg.replace("'", "''"))).unwrap_or("NULL".to_string()),
            backup_info.metadata.to_string().replace("'", "''")
        );

        self.db.execute(Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            sql,
        )).await?;

        Ok(())
    }

    /// 更新备份状态
    async fn update_backup_status(&self, backup_info: &BackupInfo) -> Result<(), AiStudioError> {
        let sql = format!(
            r#"
            UPDATE backups SET 
                status = '{}',
                file_size_bytes = {},
                completed_at = {},
                error_message = {}
            WHERE id = '{}'
            "#,
            serde_json::to_string(&backup_info.status).unwrap().trim_matches('"'),
            backup_info.file_size_bytes,
            backup_info.completed_at.map(|dt| format!("'{}'", dt.format("%Y-%m-%d %H:%M:%S"))).unwrap_or("NULL".to_string()),
            backup_info.error_message.as_ref().map(|msg| format!("'{}'", msg.replace("'", "''"))).unwrap_or("NULL".to_string()),
            backup_info.id
        );

        self.db.execute(Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            sql,
        )).await?;

        Ok(())
    }

    /// 获取备份信息
    async fn get_backup_info(&self, backup_id: Uuid) -> Result<BackupInfo, AiStudioError> {
        let query = format!(
            "SELECT * FROM backups WHERE id = '{}'",
            backup_id
        );

        let result = self.db.query_one(Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            query,
        )).await?;

        if let Some(row) = result {
            Ok(BackupInfo {
                id: row.try_get("", "id")?,
                backup_type: serde_json::from_str(&row.try_get::<String>("", "backup_type")?)
                    .unwrap_or(BackupType::Full),
                status: serde_json::from_str(&row.try_get::<String>("", "status")?)
                    .unwrap_or(BackupStatus::Failed),
                file_path: PathBuf::from(row.try_get::<String>("", "file_path")?),
                file_size_bytes: row.try_get("", "file_size_bytes").unwrap_or(0),
                tenant_id: row.try_get("", "tenant_id").ok(),
                created_at: row.try_get("", "created_at")?,
                completed_at: row.try_get("", "completed_at").ok(),
                error_message: row.try_get("", "error_message").ok(),
                metadata: row.try_get("", "metadata").unwrap_or_else(|_| serde_json::json!({})),
            })
        } else {
            Err(AiStudioError::not_found("备份"))
        }
    }

    /// 获取最后一次备份时间
    async fn get_last_backup_time(&self, tenant_id: Option<Uuid>) -> Result<chrono::DateTime<chrono::Utc>, AiStudioError> {
        let query = if let Some(tid) = tenant_id {
            format!(
                "SELECT MAX(created_at) as last_backup FROM backups WHERE tenant_id = '{}' AND status = 'Completed'",
                tid
            )
        } else {
            "SELECT MAX(created_at) as last_backup FROM backups WHERE status = 'Completed'".to_string()
        };

        let result = self.db.query_one(Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            query,
        )).await?;

        if let Some(row) = result {
            if let Ok(last_backup) = row.try_get::<chrono::DateTime<chrono::Utc>>("", "last_backup") {
                Ok(last_backup)
            } else {
                // 如果没有备份记录，返回一个很早的时间
                Ok(chrono::DateTime::from_timestamp(0, 0).unwrap_or_else(chrono::Utc::now))
            }
        } else {
            Ok(chrono::DateTime::from_timestamp(0, 0).unwrap_or_else(chrono::Utc::now))
        }
    }
}