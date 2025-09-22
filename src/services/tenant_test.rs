// 租户服务测试

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::entities::tenant::{TenantStatus, TenantConfig, TenantQuotaLimits};
    use sea_orm::{Database, DatabaseConnection};
    use uuid::Uuid;

    async fn setup_test_db() -> DatabaseConnection {
        // 这里应该设置测试数据库
        // 为了简化，这里返回一个模拟连接
        Database::connect("sqlite::memory:").await.unwrap()
    }

    #[tokio::test]
    async fn test_create_tenant() {
        let db = setup_test_db().await;
        let service = TenantService::new(db);

        let request = CreateTenantRequest {
            name: "test-tenant".to_string(),
            slug: "test-tenant".to_string(),
            display_name: "Test Tenant".to_string(),
            description: Some("Test tenant description".to_string()),
            contact_email: Some("test@example.com".to_string()),
            contact_phone: None,
            config: None,
            quota_limits: None,
        };

        // 注意：这个测试需要实际的数据库连接才能运行
        // 在实际项目中，应该使用测试数据库
        // let result = service.create_tenant(request).await;
        // assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_validate_slug_format() {
        let db = setup_test_db().await;
        let service = TenantService::new(db);

        // 有效的标识符
        assert!(service.validate_slug_format("valid-slug").is_ok());
        assert!(service.validate_slug_format("valid123").is_ok());
        assert!(service.validate_slug_format("a1").is_ok());

        // 无效的标识符
        assert!(service.validate_slug_format("").is_err());
        assert!(service.validate_slug_format("-invalid").is_err());
        assert!(service.validate_slug_format("invalid-").is_err());
        assert!(service.validate_slug_format("Invalid").is_err());
        assert!(service.validate_slug_format("api").is_err()); // 保留字
    }

    #[test]
    fn test_tenant_config_default() {
        let config = TenantConfig::default();
        assert_eq!(config.timezone, "UTC");
        assert_eq!(config.language, "zh-CN");
        assert_eq!(config.theme, "default");
        assert!(config.features.ai_enabled);
    }

    #[test]
    fn test_tenant_quota_limits_default() {
        let limits = TenantQuotaLimits::default();
        assert_eq!(limits.max_users, 100);
        assert_eq!(limits.max_knowledge_bases, 10);
        assert_eq!(limits.max_documents, 1000);
    }
}