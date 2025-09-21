// 数据库连接管理
// 处理数据库连接池和连接配置

use crate::config::DatabaseConfig;
use crate::errors::AiStudioError;
use sea_orm::{
    ConnectOptions, Database, DatabaseConnection, Statement, ConnectionTrait,
};
use std::sync::Arc;
use std::time::Duration;
use tracing::{info, warn, error, instrument};
use once_cell::sync::OnceCell;

/// 全局数据库连接实例
static DB_CONNECTION: OnceCell<Arc<DatabaseManager>> = OnceCell::new();

/// 数据库连接管理器
pub struct DatabaseManager {
    connection: DatabaseConnection,
    config: DatabaseConfig,
}

impl DatabaseManager {
    /// 初始化全局数据库连接
    #[instrument(skip(config))]
    pub async fn init(config: DatabaseConfig) -> Result<(), AiStudioError> {
        info!("初始化数据库连接...");
        
        let manager = Self::new(config).await?;
        
        // 执行健康检查
        manager.health_check().await?;
        
        // 检查和创建 pgvector 扩展
        manager.ensure_pgvector_extension().await?;
        
        // 存储到全局变量
        DB_CONNECTION.set(Arc::new(manager))
            .map_err(|_| AiStudioError::internal("数据库连接已经初始化"))?;
        
        info!("数据库连接初始化完成");
        Ok(())
    }

    /// 获取全局数据库连接
    pub fn get() -> Result<Arc<DatabaseManager>, AiStudioError> {
        DB_CONNECTION.get()
            .cloned()
            .ok_or_else(|| AiStudioError::internal("数据库连接未初始化"))
    }

    /// 创建新的数据库连接管理器
    #[instrument(skip(config))]
    async fn new(config: DatabaseConfig) -> Result<Self, AiStudioError> {
        let mut opt = ConnectOptions::new(&config.url);
        
        // 配置连接池参数
        opt.max_connections(config.max_connections)
            .min_connections(config.min_connections)
            .connect_timeout(Duration::from_secs(config.connect_timeout))
            .idle_timeout(Duration::from_secs(config.idle_timeout))
            .max_lifetime(Duration::from_secs(config.max_lifetime))
            .sqlx_logging(true)
            .sqlx_logging_level(tracing::log::LevelFilter::Debug);

        info!(
            url = %Self::mask_password(&config.url),
            max_connections = config.max_connections,
            min_connections = config.min_connections,
            "连接数据库"
        );

        let connection = Database::connect(opt).await
            .map_err(|e| AiStudioError::database(format!("数据库连接失败: {}", e)))?;

        Ok(Self { connection, config })
    }

    /// 获取数据库连接
    pub fn get_connection(&self) -> &DatabaseConnection {
        &self.connection
    }

    /// 获取配置
    pub fn get_config(&self) -> &DatabaseConfig {
        &self.config
    }

    /// 数据库健康检查
    #[instrument(skip(self))]
    pub async fn health_check(&self) -> Result<(), AiStudioError> {
        info!("执行数据库健康检查");
        
        // 执行简单查询来检查连接状态
        let result = self.connection
            .execute(Statement::from_string(
                sea_orm::DatabaseBackend::Postgres,
                "SELECT 1".to_string(),
            ))
            .await;

        match result {
            Ok(_) => {
                info!("数据库健康检查通过");
                Ok(())
            }
            Err(e) => {
                error!(error = %e, "数据库健康检查失败");
                Err(AiStudioError::database(format!("数据库健康检查失败: {}", e)))
            }
        }
    }

    /// 检查数据库版本
    #[instrument(skip(self))]
    pub async fn check_version(&self) -> Result<String, AiStudioError> {
        let result = self.connection
            .query_one(Statement::from_string(
                sea_orm::DatabaseBackend::Postgres,
                "SELECT version()".to_string(),
            ))
            .await
            .map_err(|e| AiStudioError::database(format!("获取数据库版本失败: {}", e)))?;

        if let Some(row) = result {
            let version: String = row.try_get("", "version")
                .map_err(|e| AiStudioError::database(format!("解析版本信息失败: {}", e)))?;
            info!(version = %version, "数据库版本");
            Ok(version)
        } else {
            Err(AiStudioError::database("无法获取数据库版本"))
        }
    }

    /// 确保 pgvector 扩展已安装
    #[instrument(skip(self))]
    pub async fn ensure_pgvector_extension(&self) -> Result<(), AiStudioError> {
        info!("检查 pgvector 扩展");

        // 检查扩展是否已安装
        let check_result = self.connection
            .query_one(Statement::from_string(
                sea_orm::DatabaseBackend::Postgres,
                "SELECT EXISTS(SELECT 1 FROM pg_extension WHERE extname = 'vector')".to_string(),
            ))
            .await
            .map_err(|e| AiStudioError::database(format!("检查 pgvector 扩展失败: {}", e)))?;

        if let Some(row) = check_result {
            let exists: bool = row.try_get("", "exists")
                .map_err(|e| AiStudioError::database(format!("解析扩展检查结果失败: {}", e)))?;

            if exists {
                info!("pgvector 扩展已安装");
                return Ok(());
            }
        }

        // 尝试创建扩展
        warn!("pgvector 扩展未安装，尝试创建");
        let create_result = self.connection
            .execute(Statement::from_string(
                sea_orm::DatabaseBackend::Postgres,
                "CREATE EXTENSION IF NOT EXISTS vector".to_string(),
            ))
            .await;

        match create_result {
            Ok(_) => {
                info!("pgvector 扩展创建成功");
                Ok(())
            }
            Err(e) => {
                error!(error = %e, "创建 pgvector 扩展失败");
                Err(AiStudioError::database(format!(
                    "创建 pgvector 扩展失败: {}. 请确保数据库支持 pgvector 扩展", e
                )))
            }
        }
    }

    /// 获取连接池状态
    #[instrument(skip(self))]
    pub async fn get_pool_status(&self) -> Result<PoolStatus, AiStudioError> {
        // 注意：SeaORM 没有直接暴露连接池状态的 API
        // 这里我们通过执行查询来间接检查连接状态
        let start_time = std::time::Instant::now();
        
        self.health_check().await?;
        
        let response_time = start_time.elapsed();

        Ok(PoolStatus {
            max_connections: self.config.max_connections,
            min_connections: self.config.min_connections,
            response_time_ms: response_time.as_millis() as u64,
            is_healthy: true,
        })
    }

    /// 执行数据库迁移检查
    #[instrument(skip(self))]
    pub async fn check_migrations(&self) -> Result<(), AiStudioError> {
        info!("检查数据库迁移状态");

        // 检查是否存在迁移表
        let table_exists = self.connection
            .query_one(Statement::from_string(
                sea_orm::DatabaseBackend::Postgres,
                "SELECT EXISTS (SELECT FROM information_schema.tables WHERE table_name = 'seaql_migrations')".to_string(),
            ))
            .await
            .map_err(|e| AiStudioError::database(format!("检查迁移表失败: {}", e)))?;

        if let Some(row) = table_exists {
            let exists: bool = row.try_get("", "exists")
                .map_err(|e| AiStudioError::database(format!("解析迁移表检查结果失败: {}", e)))?;

            if exists {
                info!("数据库迁移表存在");
            } else {
                warn!("数据库迁移表不存在，可能需要运行迁移");
            }
        }

        Ok(())
    }

    /// 关闭数据库连接
    #[instrument(skip(self))]
    pub async fn close(self) -> Result<(), AiStudioError> {
        info!("关闭数据库连接");
        
        self.connection.close().await
            .map_err(|e| AiStudioError::database(format!("关闭数据库连接失败: {}", e)))?;
        
        info!("数据库连接已关闭");
        Ok(())
    }

    /// 屏蔽密码信息用于日志记录
    pub fn mask_password(url: &str) -> String {
        if let Ok(mut parsed_url) = url::Url::parse(url) {
            if parsed_url.password().is_some() {
                let _ = parsed_url.set_password(Some("***"));
            }
            parsed_url.to_string()
        } else {
            "***".to_string()
        }
    }
}

/// 连接池状态
#[derive(Debug, Clone)]
pub struct PoolStatus {
    pub max_connections: u32,
    pub min_connections: u32,
    pub response_time_ms: u64,
    pub is_healthy: bool,
}

/// 数据库工具函数
pub struct DatabaseUtils;

impl DatabaseUtils {
    /// 测试数据库连接
    #[instrument(skip(config))]
    pub async fn test_connection(config: &DatabaseConfig) -> Result<(), AiStudioError> {
        info!("测试数据库连接");
        
        let manager = DatabaseManager::new(config.clone()).await?;
        manager.health_check().await?;
        manager.close().await?;
        
        info!("数据库连接测试成功");
        Ok(())
    }

    /// 创建数据库（如果不存在）
    #[instrument(skip(admin_config))]
    pub async fn create_database_if_not_exists(
        admin_config: &DatabaseConfig,
        database_name: &str,
    ) -> Result<(), AiStudioError> {
        info!(database = %database_name, "检查并创建数据库");

        // 连接到 postgres 数据库（管理员数据库）
        let mut admin_url = admin_config.url.clone();
        if let Ok(mut parsed_url) = url::Url::parse(&admin_url) {
            parsed_url.set_path("/postgres");
            admin_url = parsed_url.to_string();
        }

        let admin_manager = DatabaseManager::new(DatabaseConfig {
            url: admin_url,
            ..admin_config.clone()
        }).await?;

        // 检查数据库是否存在
        let db_exists = admin_manager.connection
            .query_one(Statement::from_string(
                sea_orm::DatabaseBackend::Postgres,
                format!("SELECT EXISTS(SELECT datname FROM pg_catalog.pg_database WHERE datname = '{}')", database_name),
            ))
            .await
            .map_err(|e| AiStudioError::database(format!("检查数据库是否存在失败: {}", e)))?;

        if let Some(row) = db_exists {
            let exists: bool = row.try_get("", "exists")
                .map_err(|e| AiStudioError::database(format!("解析数据库存在检查结果失败: {}", e)))?;

            if !exists {
                info!(database = %database_name, "数据库不存在，创建数据库");
                
                admin_manager.connection
                    .execute(Statement::from_string(
                        sea_orm::DatabaseBackend::Postgres,
                        format!("CREATE DATABASE \"{}\"", database_name),
                    ))
                    .await
                    .map_err(|e| AiStudioError::database(format!("创建数据库失败: {}", e)))?;

                info!(database = %database_name, "数据库创建成功");
            } else {
                info!(database = %database_name, "数据库已存在");
            }
        }

        admin_manager.close().await?;
        Ok(())
    }
}