// 数据库系统测试

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DatabaseConfig;
    use crate::db::{DatabaseManager, DatabaseHealthChecker, MigrationManager};

    // 注意：这些测试需要实际的数据库连接，在 CI/CD 中可能需要跳过
    // 或者使用测试数据库

    #[tokio::test]
    #[ignore] // 需要实际数据库连接
    async fn test_database_connection() {
        let config = DatabaseConfig {
            url: "postgresql://test:test@localhost:5432/test_db".to_string(),
            max_connections: 5,
            min_connections: 1,
            connect_timeout: 30,
            idle_timeout: 600,
            max_lifetime: 1800,
        };

        // 测试连接
        let result = DatabaseManager::init(config).await;
        assert!(result.is_ok());

        // 测试获取连接
        let manager = DatabaseManager::get();
        assert!(manager.is_ok());

        // 测试健康检查
        let health_result = manager.unwrap().health_check().await;
        assert!(health_result.is_ok());
    }

    #[tokio::test]
    async fn test_database_config_validation() {
        let config = DatabaseConfig {
            url: "postgresql://invalid:invalid@nonexistent:5432/invalid".to_string(),
            max_connections: 5,
            min_connections: 1,
            connect_timeout: 30,
            idle_timeout: 600,
            max_lifetime: 1800,
        };

        let result = DatabaseManager::init(config).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_password_masking() {
        let url_with_password = "postgresql://user:password@localhost:5432/db";
        let masked = DatabaseManager::mask_password(url_with_password);
        assert!(!masked.contains("password"));
        assert!(masked.contains("***"));
    }

    #[test]
    fn test_pool_status_creation() {
        let status = crate::db::PoolStatus {
            max_connections: 10,
            min_connections: 1,
            response_time_ms: 50,
            is_healthy: true,
        };

        assert_eq!(status.max_connections, 10);
        assert_eq!(status.min_connections, 1);
        assert_eq!(status.response_time_ms, 50);
        assert!(status.is_healthy);
    }

    #[tokio::test]
    #[ignore] // 需要实际数据库连接
    async fn test_health_checker() {
        // 这个测试需要先初始化数据库连接
        let health = DatabaseHealthChecker::check_health().await;
        
        // 在没有数据库连接的情况下，状态应该是 Unhealthy
        assert_eq!(health.status, crate::db::HealthStatus::Unhealthy);
        assert!(health.error_message.is_some());
    }

    #[test]
    fn test_migration_status() {
        let now = chrono::Utc::now().naive_utc();
        let status = crate::db::MigrationStatus {
            name: "test_migration".to_string(),
            version: 20240101000001, // 假设版本是数字而非字符串
            applied_at: now,
            checksum: "dummy_checksum".to_string(),
        };

        assert_eq!(status.name, "test_migration");
        assert_eq!(status.version, 20240101000001);
        assert_eq!(status.applied_at, now);
        assert_eq!(status.checksum, "dummy_checksum");
    }

    #[test]
    fn test_schema_validation() {
        let validation = crate::db::SchemaValidation {
            is_valid: false,
            missing_tables: vec!["users".to_string(), "tenants".to_string()],
            missing_columns: vec!["email".to_string()],
            missing_indexes: vec!["idx_user_email".to_string()],
            errors: vec!["Connection failed".to_string()],
        };

        assert!(!validation.is_valid);
        assert_eq!(validation.missing_tables.len(), 2);
        assert_eq!(validation.missing_columns.len(), 1);
        assert_eq!(validation.missing_indexes.len(), 1);
        assert_eq!(validation.errors.len(), 1);
    }

    #[test]
    fn test_database_stats() {
        let stats = crate::db::DatabaseStats {
            database_size: "10 MB".to_string(),
            total_connections: 5,
            uptime: Some("2 days".to_string()),
            last_updated: chrono::Utc::now(),
        };

        assert_eq!(stats.database_size, "10 MB");
        assert_eq!(stats.total_connections, 5);
        assert!(stats.uptime.is_some());
    }

    #[test]
    fn test_performance_metrics() {
        let metrics = crate::db::PerformanceMetrics {
            slow_query_count: 3,
            avg_response_time_ms: 150,
            cache_hit_ratio: 0.95,
            last_measured: chrono::Utc::now(),
        };

        assert_eq!(metrics.slow_query_count, 3);
        assert_eq!(metrics.avg_response_time_ms, 150);
        assert_eq!(metrics.cache_hit_ratio, 0.95);
    }

    #[test]
    fn test_extension_status() {
        let extension = crate::db::ExtensionStatus {
            name: "vector".to_string(),
            installed: true,
            version: Some("0.5.0".to_string()),
        };

        assert_eq!(extension.name, "vector");
        assert!(extension.installed);
        assert_eq!(extension.version, Some("0.5.0".to_string()));
    }

    #[test]
    fn test_health_status_enum() {
        use crate::db::HealthStatus;
        
        assert_eq!(HealthStatus::Healthy, HealthStatus::Healthy);
        assert_ne!(HealthStatus::Healthy, HealthStatus::Degraded);
        assert_ne!(HealthStatus::Degraded, HealthStatus::Unhealthy);
    }
}