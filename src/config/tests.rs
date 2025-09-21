// 配置系统测试

#[cfg(test)]
mod tests {
    use crate::config::*;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        
        assert_eq!(config.server.host, "127.0.0.1");
        assert_eq!(config.server.port, 8080);
        assert_eq!(config.database.max_connections, 10);
        assert_eq!(config.ai.max_tokens, 2048);
        assert_eq!(config.vector.dimension, 1536);
    }

    #[test]
    fn test_config_validation() {
        let config = AppConfig::default();
        
        // 默认配置应该通过验证（除了 JWT 密钥长度）
        // 我们需要设置一个足够长的 JWT 密钥
        let mut config = config;
        config.security.jwt_secret = "a".repeat(32);
        
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_invalid_config_validation() {
        let mut config = AppConfig::default();
        
        // 测试无效的端口
        config.server.port = 0;
        assert!(config.validate().is_err());
        
        // 重置端口，测试无效的数据库连接数
        config.server.port = 8080;
        config.database.max_connections = 0;
        assert!(config.validate().is_err());
        
        // 重置数据库连接数，测试无效的 AI 温度
        config.database.max_connections = 10;
        config.ai.temperature = 3.0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_environment_methods() {
        let mut config = AppConfig::default();
        
        config.environment.name = "development".to_string();
        assert!(config.is_development());
        assert!(!config.is_production());
        assert!(!config.is_test());
        
        config.environment.name = "production".to_string();
        assert!(!config.is_development());
        assert!(config.is_production());
        assert!(!config.is_test());
        
        config.environment.name = "test".to_string();
        assert!(!config.is_development());
        assert!(!config.is_production());
        assert!(config.is_test());
    }

    #[test]
    fn test_config_validator_server() {
        use crate::config::ConfigValidator;
        
        let mut server_config = ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 8080,
            workers: Some(4),
            keep_alive: 75,
            client_timeout: 5000,
            client_shutdown: 5000,
        };
        
        // 有效配置
        assert!(ConfigValidator::validate_server(&server_config).is_ok());
        
        // 无效端口
        server_config.port = 0;
        assert!(ConfigValidator::validate_server(&server_config).is_err());
        
        // 过多工作线程
        server_config.port = 8080;
        server_config.workers = Some(100);
        assert!(ConfigValidator::validate_server(&server_config).is_err());
    }

    #[test]
    fn test_config_validator_database() {
        use crate::config::ConfigValidator;
        
        let mut db_config = DatabaseConfig {
            url: "postgresql://localhost/test".to_string(),
            max_connections: 10,
            min_connections: 1,
            connect_timeout: 30,
            idle_timeout: 600,
            max_lifetime: 1800,
        };
        
        // 有效配置
        assert!(ConfigValidator::validate_database(&db_config).is_ok());
        
        // 无效 URL
        db_config.url = "invalid-url".to_string();
        assert!(ConfigValidator::validate_database(&db_config).is_err());
        
        // 最小连接数大于最大连接数
        db_config.url = "postgresql://localhost/test".to_string();
        db_config.min_connections = 20;
        assert!(ConfigValidator::validate_database(&db_config).is_err());
    }

    #[test]
    fn test_config_validator_ai() {
        use crate::config::ConfigValidator;
        
        let mut ai_config = AiConfig {
            model_endpoint: "http://localhost:11434".to_string(),
            api_key: "test_key".to_string(),
            max_tokens: 2048,
            temperature: 0.7,
            timeout: 30,
            retry_attempts: 3,
        };
        
        // 有效配置
        assert!(ConfigValidator::validate_ai(&ai_config).is_ok());
        
        // 无效温度
        ai_config.temperature = 3.0;
        assert!(ConfigValidator::validate_ai(&ai_config).is_err());
        
        // 无效端点 URL
        ai_config.temperature = 0.7;
        ai_config.model_endpoint = "invalid-url".to_string();
        assert!(ConfigValidator::validate_ai(&ai_config).is_err());
    }

    #[test]
    fn test_config_validator_security() {
        use crate::config::ConfigValidator;
        
        let mut security_config = SecurityConfig {
            jwt_secret: "a".repeat(32),
            jwt_expiration: 3600,
            bcrypt_cost: 12,
            cors_origins: vec!["*".to_string()],
            rate_limit_requests: 100,
            rate_limit_window: 60,
        };
        
        // 有效配置
        assert!(ConfigValidator::validate_security(&security_config).is_ok());
        
        // JWT 密钥太短
        security_config.jwt_secret = "short".to_string();
        assert!(ConfigValidator::validate_security(&security_config).is_err());
        
        // 无效的 bcrypt 成本
        security_config.jwt_secret = "a".repeat(32);
        security_config.bcrypt_cost = 50;
        assert!(ConfigValidator::validate_security(&security_config).is_err());
    }

    #[test]
    fn test_config_validator_vector() {
        use crate::config::ConfigValidator;
        
        let mut vector_config = VectorConfig {
            dimension: 1536,
            similarity_threshold: 0.8,
            index_type: "hnsw".to_string(),
            ef_construction: 200,
            m: 16,
        };
        
        // 有效配置
        assert!(ConfigValidator::validate_vector(&vector_config).is_ok());
        
        // 无效相似度阈值
        vector_config.similarity_threshold = 1.5;
        assert!(ConfigValidator::validate_vector(&vector_config).is_err());
        
        // 无效索引类型
        vector_config.similarity_threshold = 0.8;
        vector_config.index_type = "invalid".to_string();
        assert!(ConfigValidator::validate_vector(&vector_config).is_err());
    }
}