// 数据库健康检查
// 提供数据库状态监控和诊断功能

use crate::db::DatabaseManager;
use crate::errors::AiStudioError;
use serde::{Deserialize, Serialize};
use std::time::Instant;
use tracing::{info, warn, error, instrument};
use sea_orm::ConnectionTrait;

/// 数据库健康状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseHealth {
    pub status: HealthStatus,
    pub response_time_ms: u64,
    pub version: Option<String>,
    pub pool_status: Option<PoolHealthStatus>,
    pub extensions: Vec<ExtensionStatus>,
    pub last_check: chrono::DateTime<chrono::Utc>,
    pub error_message: Option<String>,
}

/// 健康状态枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

/// 连接池健康状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolHealthStatus {
    pub max_connections: u32,
    pub min_connections: u32,
    pub active_connections: Option<u32>,
    pub idle_connections: Option<u32>,
}

/// 扩展状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionStatus {
    pub name: String,
    pub installed: bool,
    pub version: Option<String>,
}

/// 数据库健康检查器
pub struct DatabaseHealthChecker;

impl DatabaseHealthChecker {
    /// 执行完整的健康检查
    #[instrument]
    pub async fn check_health() -> DatabaseHealth {
        let start_time = Instant::now();
        let mut health = DatabaseHealth {
            status: HealthStatus::Unhealthy,
            response_time_ms: 0,
            version: None,
            pool_status: None,
            extensions: Vec::new(),
            last_check: chrono::Utc::now(),
            error_message: None,
        };

        // 获取数据库管理器
        let db_manager = match DatabaseManager::get() {
            Ok(manager) => manager,
            Err(e) => {
                error!(error = %e, "无法获取数据库管理器");
                health.error_message = Some(e.to_string());
                return health;
            }
        };

        // 基础连接检查
        match db_manager.health_check().await {
            Ok(_) => {
                info!("数据库基础健康检查通过");
                health.status = HealthStatus::Healthy;
            }
            Err(e) => {
                error!(error = %e, "数据库基础健康检查失败");
                health.error_message = Some(e.to_string());
                health.response_time_ms = start_time.elapsed().as_millis() as u64;
                return health;
            }
        }

        // 获取数据库版本
        match db_manager.check_version().await {
            Ok(version) => {
                health.version = Some(version);
            }
            Err(e) => {
                warn!(error = %e, "获取数据库版本失败");
                health.status = HealthStatus::Degraded;
            }
        }

        // 获取连接池状态
        match db_manager.get_pool_status().await {
            Ok(pool_status) => {
                health.pool_status = Some(PoolHealthStatus {
                    max_connections: pool_status.max_connections,
                    min_connections: pool_status.min_connections,
                    active_connections: None, // SeaORM 不直接暴露这些信息
                    idle_connections: None,
                });
            }
            Err(e) => {
                warn!(error = %e, "获取连接池状态失败");
                health.status = HealthStatus::Degraded;
            }
        }

        // 检查扩展状态
        health.extensions = Self::check_extensions(&db_manager).await;

        health.response_time_ms = start_time.elapsed().as_millis() as u64;
        health
    }

    /// 快速健康检查（仅检查连接）
    #[instrument]
    pub async fn quick_check() -> Result<bool, AiStudioError> {
        let db_manager = DatabaseManager::get()?;
        db_manager.health_check().await?;
        Ok(true)
    }

    /// 检查数据库扩展
    #[instrument(skip(db_manager))]
    async fn check_extensions(db_manager: &DatabaseManager) -> Vec<ExtensionStatus> {
        let mut extensions = Vec::new();

        // 检查 pgvector 扩展
        let pgvector_status = Self::check_extension(db_manager, "vector").await;
        extensions.push(pgvector_status);

        // 可以添加其他扩展检查
        // let uuid_status = Self::check_extension(db_manager, "uuid-ossp").await;
        // extensions.push(uuid_status);

        extensions
    }

    /// 检查单个扩展
    #[instrument(skip(db_manager))]
    async fn check_extension(db_manager: &DatabaseManager, extension_name: &str) -> ExtensionStatus {
        let query = format!(
            "SELECT EXISTS(SELECT 1 FROM pg_extension WHERE extname = '{}'), 
             COALESCE((SELECT extversion FROM pg_extension WHERE extname = '{}'), '') as version",
            extension_name, extension_name
        );

        match db_manager.get_connection().query_one(
            sea_orm::Statement::from_string(sea_orm::DatabaseBackend::Postgres, query)
        ).await {
            Ok(Some(row)) => {
                let installed: bool = row.try_get("", "exists").unwrap_or(false);
                let version: String = row.try_get("", "version").unwrap_or_default();
                
                ExtensionStatus {
                    name: extension_name.to_string(),
                    installed,
                    version: if version.is_empty() { None } else { Some(version) },
                }
            }
            Ok(None) | Err(_) => {
                ExtensionStatus {
                    name: extension_name.to_string(),
                    installed: false,
                    version: None,
                }
            }
        }
    }

    /// 获取详细的数据库统计信息
    #[instrument]
    pub async fn get_database_stats() -> Result<DatabaseStats, AiStudioError> {
        let db_manager = DatabaseManager::get()?;
        let connection = db_manager.get_connection();

        // 获取数据库大小
        let size_query = "SELECT pg_size_pretty(pg_database_size(current_database())) as size";
        let size_result = connection.query_one(
            sea_orm::Statement::from_string(sea_orm::DatabaseBackend::Postgres, size_query.to_string())
        ).await.map_err(|e| AiStudioError::database(format!("获取数据库大小失败: {}", e)))?;

        let database_size = if let Some(row) = size_result {
            row.try_get("", "size").unwrap_or_default()
        } else {
            "未知".to_string()
        };

        // 获取连接数统计
        let connections_query = "SELECT count(*) as total_connections FROM pg_stat_activity";
        let connections_result = connection.query_one(
            sea_orm::Statement::from_string(sea_orm::DatabaseBackend::Postgres, connections_query.to_string())
        ).await.map_err(|e| AiStudioError::database(format!("获取连接数失败: {}", e)))?;

        let total_connections: i64 = if let Some(row) = connections_result {
            row.try_get("", "total_connections").unwrap_or(0)
        } else {
            0
        };

        Ok(DatabaseStats {
            database_size,
            total_connections: total_connections as u32,
            uptime: None, // 可以通过其他查询获取
            last_updated: chrono::Utc::now(),
        })
    }
}

/// 数据库统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseStats {
    pub database_size: String,
    pub total_connections: u32,
    pub uptime: Option<String>,
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

/// 数据库监控器
pub struct DatabaseMonitor;

impl DatabaseMonitor {
    /// 启动定期健康检查
    #[instrument]
    pub async fn start_monitoring(interval_seconds: u64) {
        info!(interval = interval_seconds, "启动数据库监控");

        let mut interval = tokio::time::interval(std::time::Duration::from_secs(interval_seconds));
        
        loop {
            interval.tick().await;
            
            let health = DatabaseHealthChecker::check_health().await;
            
            match health.status {
                HealthStatus::Healthy => {
                    info!(
                        response_time_ms = health.response_time_ms,
                        "数据库健康检查正常"
                    );
                }
                HealthStatus::Degraded => {
                    warn!(
                        response_time_ms = health.response_time_ms,
                        error = ?health.error_message,
                        "数据库健康检查降级"
                    );
                }
                HealthStatus::Unhealthy => {
                    error!(
                        response_time_ms = health.response_time_ms,
                        error = ?health.error_message,
                        "数据库健康检查失败"
                    );
                }
            }
        }
    }

    /// 检查数据库性能指标
    #[instrument]
    pub async fn check_performance_metrics() -> Result<PerformanceMetrics, AiStudioError> {
        let db_manager = DatabaseManager::get()?;
        let connection = db_manager.get_connection();

        // 获取慢查询统计
        let slow_queries_query = "
            SELECT count(*) as slow_query_count 
            FROM pg_stat_statements 
            WHERE mean_exec_time > 1000
        ";

        let slow_queries_result = connection.query_one(
            sea_orm::Statement::from_string(sea_orm::DatabaseBackend::Postgres, slow_queries_query.to_string())
        ).await;

        let slow_query_count = match slow_queries_result {
            Ok(Some(row)) => row.try_get("", "slow_query_count").unwrap_or(0i64) as u32,
            _ => 0,
        };

        Ok(PerformanceMetrics {
            slow_query_count,
            avg_response_time_ms: 0, // 需要更复杂的查询来计算
            cache_hit_ratio: 0.0,    // 需要查询缓存统计
            last_measured: chrono::Utc::now(),
        })
    }
}

/// 性能指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub slow_query_count: u32,
    pub avg_response_time_ms: u64,
    pub cache_hit_ratio: f64,
    pub last_measured: chrono::DateTime<chrono::Utc>,
}