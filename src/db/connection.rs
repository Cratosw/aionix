// 数据库连接管理
// 处理数据库连接池和连接配置

use sea_orm::{Database, DatabaseConnection, DbErr};

/// 数据库连接管理器
pub struct DatabaseManager {
    connection: DatabaseConnection,
}

impl DatabaseManager {
    /// 创建新的数据库连接
    pub async fn new(database_url: &str) -> Result<Self, DbErr> {
        let connection = Database::connect(database_url).await?;
        Ok(Self { connection })
    }

    /// 获取数据库连接
    pub fn get_connection(&self) -> &DatabaseConnection {
        &self.connection
    }

    /// 健康检查
    pub async fn health_check(&self) -> Result<(), DbErr> {
        // 执行简单查询来检查连接状态
        self.connection.ping().await
    }
}