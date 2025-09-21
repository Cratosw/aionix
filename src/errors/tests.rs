// 错误处理系统测试

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::{AiStudioError, ErrorResponse};
    use actix_web::http::StatusCode;

    #[test]
    fn test_error_creation() {
        let error = AiStudioError::validation("email", "邮箱格式无效");
        assert_eq!(error.error_code(), "VALIDATION_ERROR");
        assert_eq!(error.status_code(), 400);
        assert!(error.is_client_error());
        assert!(!error.is_server_error());
    }

    #[test]
    fn test_database_error() {
        let error = AiStudioError::database_with_code("连接失败", "23505");
        assert_eq!(error.error_code(), "DATABASE_ERROR");
        assert_eq!(error.status_code(), 500);
        assert!(!error.is_client_error());
        assert!(error.is_server_error());
    }

    #[test]
    fn test_ai_service_error() {
        let error = AiStudioError::ai_service_with_model("模型不可用", "gpt-4");
        assert_eq!(error.error_code(), "AI_SERVICE_ERROR");
        assert_eq!(error.status_code(), 502);
    }

    #[test]
    fn test_rate_limit_error() {
        let error = AiStudioError::rate_limit(Some(60));
        assert_eq!(error.error_code(), "RATE_LIMIT");
        assert_eq!(error.status_code(), 429);
    }

    #[test]
    fn test_error_logging() {
        let validation_error = AiStudioError::validation("field", "message");
        assert!(!validation_error.should_log());

        let internal_error = AiStudioError::internal("something went wrong");
        assert!(internal_error.should_log());
    }

    #[test]
    fn test_error_response_creation() {
        let error = AiStudioError::validation("email", "邮箱格式无效");
        let response = ErrorResponse::from_error(&error);
        
        assert!(!response.success);
        assert_eq!(response.error.code, "VALIDATION_ERROR");
        assert!(response.error.message.contains("邮箱格式无效"));
        assert!(response.error.details.is_some());
    }

    #[test]
    fn test_error_response_with_context() {
        let error = AiStudioError::internal("测试错误");
        let response = ErrorResponse::from_error(&error)
            .with_request_id("test-request-123".to_string())
            .with_trace_id("test-trace-456".to_string());
        
        assert_eq!(response.request_id, Some("test-request-123".to_string()));
        assert_eq!(response.trace_id, Some("test-trace-456".to_string()));
    }

    #[test]
    fn test_common_error_conversion() {
        let common_error = aionix_common::CommonError::validation("测试验证错误");
        let ai_studio_error: AiStudioError = common_error.into();
        
        assert_eq!(ai_studio_error.error_code(), "VALIDATION_ERROR");
    }

    #[test]
    fn test_io_error_conversion() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "文件未找到");
        let ai_studio_error: AiStudioError = io_error.into();
        
        assert_eq!(ai_studio_error.error_code(), "NOT_FOUND");
    }

    #[test]
    fn test_json_error_conversion() {
        let json_error = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
        let ai_studio_error: AiStudioError = json_error.into();
        
        assert_eq!(ai_studio_error.error_code(), "VALIDATION_ERROR");
    }

    #[test]
    fn test_uuid_error_conversion() {
        let uuid_error = uuid::Uuid::parse_str("invalid-uuid").unwrap_err();
        let ai_studio_error: AiStudioError = uuid_error.into();
        
        assert_eq!(ai_studio_error.error_code(), "VALIDATION_ERROR");
    }

    #[test]
    fn test_tenant_error_with_id() {
        let error = AiStudioError::tenant_with_id("租户不存在", "tenant-123");
        assert_eq!(error.error_code(), "TENANT_ERROR");
        
        let response = ErrorResponse::from_error(&error);
        assert!(response.error.details.is_some());
        
        if let Some(details) = response.error.details {
            assert_eq!(details["tenant_id"], "tenant-123");
        }
    }

    #[test]
    fn test_file_processing_error_with_name() {
        let error = AiStudioError::file_processing_with_name("文件格式不支持", "document.xyz");
        assert_eq!(error.error_code(), "FILE_PROCESSING_ERROR");
        
        let response = ErrorResponse::from_error(&error);
        assert!(response.error.details.is_some());
        
        if let Some(details) = response.error.details {
            assert_eq!(details["file_name"], "document.xyz");
        }
    }

    #[test]
    fn test_external_service_error() {
        let error = AiStudioError::external_service("OpenAI", "API 限额已用完");
        assert_eq!(error.error_code(), "EXTERNAL_SERVICE_ERROR");
        
        let response = ErrorResponse::from_error(&error);
        assert!(response.error.details.is_some());
        
        if let Some(details) = response.error.details {
            assert_eq!(details["service"], "OpenAI");
        }
    }

    #[test]
    fn test_timeout_error() {
        let error = AiStudioError::timeout("数据库查询");
        assert_eq!(error.error_code(), "TIMEOUT_ERROR");
        assert_eq!(error.status_code(), 408);
        
        let response = ErrorResponse::from_error(&error);
        assert!(response.error.details.is_some());
        
        if let Some(details) = response.error.details {
            assert_eq!(details["operation"], "数据库查询");
        }
    }
}