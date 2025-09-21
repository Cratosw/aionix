// 日志系统测试

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logging::{LoggingSetup, RequestContext};
    use tracing::Level;

    #[test]
    fn test_parse_level() {
        assert_eq!(LoggingSetup::parse_level("trace"), Level::TRACE);
        assert_eq!(LoggingSetup::parse_level("debug"), Level::DEBUG);
        assert_eq!(LoggingSetup::parse_level("info"), Level::INFO);
        assert_eq!(LoggingSetup::parse_level("warn"), Level::WARN);
        assert_eq!(LoggingSetup::parse_level("error"), Level::ERROR);
        assert_eq!(LoggingSetup::parse_level("invalid"), Level::INFO);
    }

    #[test]
    fn test_development_config() {
        let config = LoggingSetup::development_config();
        assert_eq!(config.level, "debug");
        assert_eq!(config.format, "pretty");
        assert!(!config.file_enabled);
    }

    #[test]
    fn test_production_config() {
        let config = LoggingSetup::production_config();
        assert_eq!(config.level, "info");
        assert_eq!(config.format, "json");
        assert!(config.file_enabled);
        assert!(config.file_path.is_some());
    }

    #[test]
    fn test_test_config() {
        let config = LoggingSetup::test_config();
        assert_eq!(config.level, "warn");
        assert_eq!(config.format, "compact");
        assert!(!config.file_enabled);
    }

    #[test]
    fn test_request_context_creation() {
        let context = RequestContext::new();
        
        assert!(!context.request_id.is_empty());
        assert!(!context.trace_id.is_empty());
        assert!(context.user_id.is_none());
        assert!(context.tenant_id.is_none());
    }

    #[test]
    fn test_request_context_with_user() {
        let context = RequestContext::new()
            .with_user_id("user-123".to_string())
            .with_tenant_id("tenant-456".to_string())
            .with_session_id("session-789".to_string());
        
        assert_eq!(context.user_id, Some("user-123".to_string()));
        assert_eq!(context.tenant_id, Some("tenant-456".to_string()));
        assert_eq!(context.session_id, Some("session-789".to_string()));
    }

    #[test]
    fn test_request_context_duration() {
        let context = RequestContext::new();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let duration = context.duration();
        assert!(duration.num_milliseconds() >= 10);
    }

    #[test]
    fn test_request_context_log_fields() {
        let context = RequestContext::new()
            .with_user_id("user-123".to_string())
            .with_tenant_id("tenant-456".to_string());
        
        let fields = context.to_log_fields();
        
        // 检查必需字段
        assert!(fields.iter().any(|(k, _)| *k == "request_id"));
        assert!(fields.iter().any(|(k, _)| *k == "trace_id"));
        assert!(fields.iter().any(|(k, _)| *k == "start_time"));
        assert!(fields.iter().any(|(k, v)| *k == "user_id" && v == "user-123"));
        assert!(fields.iter().any(|(k, v)| *k == "tenant_id" && v == "tenant-456"));
    }

    #[test]
    fn test_log_filter_config_default() {
        let config = crate::logging::LogFilterConfig::default();
        
        assert!(config.enable_sensitive_filter);
        assert!(config.enable_performance_filter);
        assert!(config.enable_development_filter);
        assert!(!config.enable_error_focus_filter);
        assert!(config.enable_request_filter);
        assert_eq!(config.min_level, Level::INFO);
    }

    #[test]
    fn test_log_filter_config_production() {
        let config = crate::logging::LogFilterConfig::production();
        
        assert!(config.enable_sensitive_filter);
        assert!(config.enable_performance_filter);
        assert!(!config.enable_development_filter);
        assert!(config.enable_error_focus_filter);
        assert!(config.enable_request_filter);
        assert_eq!(config.min_level, Level::INFO);
        assert_eq!(config.allowed_modules, vec!["aionix"]);
    }

    #[test]
    fn test_log_filter_config_development() {
        let config = crate::logging::LogFilterConfig::development();
        
        assert!(config.enable_sensitive_filter);
        assert!(!config.enable_performance_filter);
        assert!(config.enable_development_filter);
        assert!(!config.enable_error_focus_filter);
        assert!(!config.enable_request_filter);
        assert_eq!(config.min_level, Level::DEBUG);
        assert!(config.allowed_modules.contains(&"aionix".to_string()));
        assert!(config.allowed_modules.contains(&"actix_web".to_string()));
    }

    #[test]
    fn test_log_filter_config_test() {
        let config = crate::logging::LogFilterConfig::test();
        
        assert!(config.enable_sensitive_filter);
        assert!(config.enable_performance_filter);
        assert!(!config.enable_development_filter);
        assert!(config.enable_error_focus_filter);
        assert!(config.enable_request_filter);
        assert_eq!(config.min_level, Level::WARN);
        assert_eq!(config.allowed_modules, vec!["aionix"]);
    }

    // 注意：由于 HTTP 请求的测试需要 actix-web 测试框架，
    // 这里只测试基本的上下文创建逻辑
    #[test]
    fn test_request_context_default() {
        let context = RequestContext::default();
        assert!(!context.request_id.is_empty());
        assert!(!context.trace_id.is_empty());
    }
}