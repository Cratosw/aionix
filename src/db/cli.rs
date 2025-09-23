// 数据库管理 CLI 工具
// 提供迁移、备份、恢复等命令行功能

use crate::config::AppConfig;
use crate::db::migrations::{MigrationManager, SeedDataManager, BackupManager, BackupType, RestoreOptions};
use crate::errors::AiStudioError;
use sea_orm::{Database, DatabaseConnection};
use std::path::PathBuf;
use tracing::info;
use uuid::Uuid;

/// CLI 命令
#[derive(Debug, Clone)]
pub enum CliCommand {
    /// 迁移相关命令
    Migration(MigrationCommand),
    /// 种子数据相关命令
    Seed(SeedCommand),
    /// 备份相关命令
    Backup(BackupCommand),
}

/// 迁移命令
#[derive(Debug, Clone)]
pub enum MigrationCommand {
    /// 初始化迁移系统
    Init,
    /// 检查迁移状态
    Status,
    /// 应用迁移
    Migrate,
    /// 回滚迁移
    Rollback { version: String },
    /// 重置迁移
    Reset,
    /// 验证数据库架构
    Validate,
}

/// 种子数据命令
#[derive(Debug, Clone)]
pub enum SeedCommand {
    /// 初始化种子数据
    Init,
    /// 清理种子数据
    Clean,
    /// 重新初始化种子数据
    Reseed,
}

/// 备份命令
#[derive(Debug, Clone)]
pub enum BackupCommand {
    /// 创建备份
    Create {
        backup_type: BackupType,
        tenant_id: Option<Uuid>,
    },
    /// 列出备份
    List {
        tenant_id: Option<Uuid>,
    },
    /// 恢复备份
    Restore {
        backup_id: Uuid,
        clean: bool,
        data_only: bool,
        schema_only: bool,
    },
    /// 删除备份
    Delete {
        backup_id: Uuid,
    },
    /// 验证备份
    Verify {
        backup_id: Uuid,
    },
    /// 清理过期备份
    Cleanup {
        retention_days: u32,
    },
}

/// CLI 执行器
pub struct CliExecutor {
    db: DatabaseConnection,
    config: AppConfig,
}

impl CliExecutor {
    /// 创建新的 CLI 执行器
    pub async fn new(config: AppConfig) -> Result<Self, AiStudioError> {
        let db = Database::connect(&config.database.url).await?;
        
        Ok(Self { db, config })
    }

    /// 执行 CLI 命令
    pub async fn execute(&self, command: CliCommand) -> Result<(), AiStudioError> {
        match command {
            CliCommand::Migration(cmd) => self.execute_migration_command(cmd).await,
            CliCommand::Seed(cmd) => self.execute_seed_command(cmd).await,
            CliCommand::Backup(cmd) => self.execute_backup_command(cmd).await,
        }
    }

    /// 执行迁移命令
    async fn execute_migration_command(&self, command: MigrationCommand) -> Result<(), AiStudioError> {
        let manager = MigrationManager::new(self.db.clone());

        match command {
            MigrationCommand::Init => {
                info!("初始化迁移系统...");
                manager.init().await?;
                println!("✅ 迁移系统初始化完成");
            }
            MigrationCommand::Status => {
                info!("检查迁移状态...");
                let status = manager.check_status().await?;
                
                println!("📊 迁移状态:");
                println!("{:<20} {:<30} {:<15} {:<20}", "版本", "名称", "状态", "应用时间");
                println!("{}", "-".repeat(85));
                
                for migration in status {
                    let status_str = if migration.is_applied { "✅ 已应用" } else { "⏳ 待应用" };
                    let applied_at = migration.applied_at
                        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                        .unwrap_or_else(|| "-".to_string());
                    
                    println!(
                        "{:<20} {:<30} {:<15} {:<20}",
                        migration.version,
                        migration.name,
                        status_str,
                        applied_at
                    );
                }
            }
            MigrationCommand::Migrate => {
                info!("应用迁移...");
                let applied = manager.migrate().await?;
                
                if applied.is_empty() {
                    println!("✅ 没有待应用的迁移");
                } else {
                    println!("✅ 成功应用 {} 个迁移:", applied.len());
                    for version in applied {
                        println!("  - {}", version);
                    }
                }
            }
            MigrationCommand::Rollback { version } => {
                info!("回滚迁移: {}", version);
                manager.rollback(&version).await?;
                println!("✅ 迁移 {} 回滚完成", version);
            }
            MigrationCommand::Reset => {
                info!("重置迁移...");
                // 这里需要实现重置逻辑
                println!("⚠️  重置功能尚未实现");
            }
            MigrationCommand::Validate => {
                info!("验证数据库架构...");
                let validation = manager.validate_schema().await?;
                
                if validation.is_valid {
                    println!("✅ 数据库架构验证通过");
                } else {
                    println!("❌ 数据库架构验证失败:");
                    
                    if !validation.missing_tables.is_empty() {
                        println!("  缺失的表:");
                        for table in validation.missing_tables {
                            println!("    - {}", table);
                        }
                    }
                    
                    if !validation.missing_columns.is_empty() {
                        println!("  缺失的列:");
                        for column in validation.missing_columns {
                            println!("    - {}", column);
                        }
                    }
                    
                    if !validation.errors.is_empty() {
                        println!("  错误:");
                        for error in validation.errors {
                            println!("    - {}", error);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// 执行种子数据命令
    async fn execute_seed_command(&self, command: SeedCommand) -> Result<(), AiStudioError> {
        let manager = SeedDataManager::new(self.db.clone());

        match command {
            SeedCommand::Init => {
                info!("初始化种子数据...");
                manager.seed_all().await?;
                println!("✅ 种子数据初始化完成");
            }
            SeedCommand::Clean => {
                info!("清理种子数据...");
                manager.clean_seed_data().await?;
                println!("✅ 种子数据清理完成");
            }
            SeedCommand::Reseed => {
                info!("重新初始化种子数据...");
                manager.reseed().await?;
                println!("✅ 种子数据重新初始化完成");
            }
        }

        Ok(())
    }

    /// 执行备份命令
    async fn execute_backup_command(&self, command: BackupCommand) -> Result<(), AiStudioError> {
        let backup_dir = PathBuf::from("./backups");
        let manager = BackupManager::new(
            self.db.clone(),
            backup_dir,
            None, // 使用默认的 pg_dump 路径
            None, // 使用默认的 pg_restore 路径
        );

        // 初始化备份系统
        manager.init().await?;

        match command {
            BackupCommand::Create { backup_type, tenant_id } => {
                info!("创建备份...");
                
                let backup_info = match backup_type {
                    BackupType::Full => manager.create_full_backup(tenant_id).await?,
                    BackupType::Incremental => manager.create_incremental_backup(tenant_id).await?,
                    _ => {
                        println!("❌ 暂不支持该备份类型");
                        return Ok(());
                    }
                };

                println!("✅ 备份创建完成:");
                println!("  ID: {}", backup_info.id);
                println!("  类型: {:?}", backup_info.backup_type);
                println!("  文件: {}", backup_info.file_path.display());
                println!("  大小: {} bytes", backup_info.file_size_bytes);
            }
            BackupCommand::List { tenant_id } => {
                info!("列出备份...");
                let backups = manager.list_backups(tenant_id).await?;
                
                if backups.is_empty() {
                    println!("📝 没有找到备份");
                } else {
                    println!("📊 备份列表:");
                    println!("{:<36} {:<15} {:<15} {:<20} {:<15}", "ID", "类型", "状态", "创建时间", "大小");
                    println!("{}", "-".repeat(100));
                    
                    for backup in backups {
                        let status_str = match backup.status {
                            crate::db::migrations::backup::BackupStatus::Completed => "✅ 完成",
                            crate::db::migrations::backup::BackupStatus::InProgress => "⏳ 进行中",
                            crate::db::migrations::backup::BackupStatus::Failed => "❌ 失败",
                            crate::db::migrations::backup::BackupStatus::Cancelled => "⏹️ 取消",
                        };
                        
                        println!(
                            "{:<36} {:<15} {:<15} {:<20} {:<15}",
                            backup.id,
                            format!("{:?}", backup.backup_type),
                            status_str,
                            backup.created_at.format("%Y-%m-%d %H:%M:%S"),
                            format!("{} bytes", backup.file_size_bytes)
                        );
                    }
                }
            }
            BackupCommand::Restore { backup_id, clean, data_only, schema_only } => {
                info!("恢复备份: {}", backup_id);
                
                let options = RestoreOptions {
                    backup_id,
                    target_database: None,
                    restore_data: !schema_only,
                    restore_schema: !data_only,
                    clean_before_restore: clean,
                    tenant_filter: None,
                };

                manager.restore_backup(options).await?;
                println!("✅ 备份恢复完成");
            }
            BackupCommand::Delete { backup_id } => {
                info!("删除备份: {}", backup_id);
                manager.delete_backup(backup_id).await?;
                println!("✅ 备份删除完成");
            }
            BackupCommand::Verify { backup_id } => {
                info!("验证备份: {}", backup_id);
                let is_valid = manager.verify_backup(backup_id).await?;
                
                if is_valid {
                    println!("✅ 备份验证通过");
                } else {
                    println!("❌ 备份验证失败");
                }
            }
            BackupCommand::Cleanup { retention_days } => {
                info!("清理过期备份...");
                let deleted = manager.cleanup_old_backups(retention_days).await?;
                
                if deleted.is_empty() {
                    println!("✅ 没有过期备份需要清理");
                } else {
                    println!("✅ 清理了 {} 个过期备份:", deleted.len());
                    for backup_id in deleted {
                        println!("  - {}", backup_id);
                    }
                }
            }
        }

        Ok(())
    }
}

/// 解析命令行参数
pub fn parse_args(args: Vec<String>) -> Result<CliCommand, AiStudioError> {
    if args.len() < 2 {
        return Err(AiStudioError::validation("args", "请提供命令"));
    }

    match args[1].as_str() {
        "migration" | "migrate" => {
            if args.len() < 3 {
                return Err(AiStudioError::validation("migration", "请提供迁移子命令"));
            }

            let subcommand = match args[2].as_str() {
                "init" => MigrationCommand::Init,
                "status" => MigrationCommand::Status,
                "migrate" | "up" => MigrationCommand::Migrate,
                "rollback" | "down" => {
                    if args.len() < 4 {
                        return Err(AiStudioError::validation("version", "请提供要回滚的版本"));
                    }
                    MigrationCommand::Rollback { version: args[3].clone() }
                }
                "reset" => MigrationCommand::Reset,
                "validate" => MigrationCommand::Validate,
                _ => return Err(AiStudioError::validation("migration", "未知的迁移子命令")),
            };

            Ok(CliCommand::Migration(subcommand))
        }
        "seed" => {
            if args.len() < 3 {
                return Err(AiStudioError::validation("seed", "请提供种子数据子命令"));
            }

            let subcommand = match args[2].as_str() {
                "init" => SeedCommand::Init,
                "clean" => SeedCommand::Clean,
                "reseed" => SeedCommand::Reseed,
                _ => return Err(AiStudioError::validation("seed", "未知的种子数据子命令")),
            };

            Ok(CliCommand::Seed(subcommand))
        }
        "backup" => {
            if args.len() < 3 {
                return Err(AiStudioError::validation("backup", "请提供备份子命令"));
            }

            let subcommand = match args[2].as_str() {
                "create" => {
                    let backup_type = if args.len() > 3 {
                        match args[3].as_str() {
                            "full" => BackupType::Full,
                            "incremental" => BackupType::Incremental,
                            "differential" => BackupType::Differential,
                            "data" => BackupType::DataOnly,
                            "schema" => BackupType::SchemaOnly,
                            _ => BackupType::Full,
                        }
                    } else {
                        BackupType::Full
                    };

                    let tenant_id = if args.len() > 4 {
                        Uuid::parse_str(&args[4]).ok()
                    } else {
                        None
                    };

                    BackupCommand::Create { backup_type, tenant_id }
                }
                "list" => {
                    let tenant_id = if args.len() > 3 {
                        Uuid::parse_str(&args[3]).ok()
                    } else {
                        None
                    };
                    BackupCommand::List { tenant_id }
                }
                "restore" => {
                    if args.len() < 4 {
                        return Err(AiStudioError::validation("backup_id", "请提供备份 ID"));
                    }
                    let backup_id = Uuid::parse_str(&args[3])
                        .map_err(|_| AiStudioError::validation("backup_id", "无效的备份 ID"))?;
                    
                    BackupCommand::Restore {
                        backup_id,
                        clean: args.contains(&"--clean".to_string()),
                        data_only: args.contains(&"--data-only".to_string()),
                        schema_only: args.contains(&"--schema-only".to_string()),
                    }
                }
                "delete" => {
                    if args.len() < 4 {
                        return Err(AiStudioError::validation("backup_id", "请提供备份 ID"));
                    }
                    let backup_id = Uuid::parse_str(&args[3])
                        .map_err(|_| AiStudioError::validation("backup_id", "无效的备份 ID"))?;
                    BackupCommand::Delete { backup_id }
                }
                "verify" => {
                    if args.len() < 4 {
                        return Err(AiStudioError::validation("backup_id", "请提供备份 ID"));
                    }
                    let backup_id = Uuid::parse_str(&args[3])
                        .map_err(|_| AiStudioError::validation("backup_id", "无效的备份 ID"))?;
                    BackupCommand::Verify { backup_id }
                }
                "cleanup" => {
                    let retention_days = if args.len() > 3 {
                        args[3].parse().unwrap_or(30)
                    } else {
                        30
                    };
                    BackupCommand::Cleanup { retention_days }
                }
                _ => return Err(AiStudioError::validation("backup", "未知的备份子命令")),
            };

            Ok(CliCommand::Backup(subcommand))
        }
        _ => Err(AiStudioError::validation("args", "未知的命令")),
    }
}

/// 打印帮助信息
pub fn print_help() {
    println!("Aionix 数据库管理工具");
    println!();
    println!("用法:");
    println!("  aionix <命令> <子命令> [选项]");
    println!();
    println!("命令:");
    println!("  migration, migrate    数据库迁移管理");
    println!("  seed                  种子数据管理");
    println!("  backup                备份和恢复管理");
    println!();
    println!("迁移命令:");
    println!("  migration init        初始化迁移系统");
    println!("  migration status      检查迁移状态");
    println!("  migration migrate     应用待处理的迁移");
    println!("  migration rollback <version>  回滚指定版本的迁移");
    println!("  migration reset       重置所有迁移");
    println!("  migration validate    验证数据库架构");
    println!();
    println!("种子数据命令:");
    println!("  seed init             初始化种子数据");
    println!("  seed clean            清理种子数据");
    println!("  seed reseed           重新初始化种子数据");
    println!();
    println!("备份命令:");
    println!("  backup create [type] [tenant_id]  创建备份");
    println!("  backup list [tenant_id]           列出备份");
    println!("  backup restore <backup_id>        恢复备份");
    println!("  backup delete <backup_id>         删除备份");
    println!("  backup verify <backup_id>         验证备份");
    println!("  backup cleanup [days]             清理过期备份");
    println!();
    println!("备份类型:");
    println!("  full          完整备份 (默认)");
    println!("  incremental   增量备份");
    println!("  differential  差异备份");
    println!("  data          仅数据备份");
    println!("  schema        仅架构备份");
}