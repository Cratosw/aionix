// 错误响应格式化

use crate::errors::AiStudioError;
use actix_web::HttpResponse;
use aionix_common::ApiResponse;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};


/// 错误响应结构
#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub success: bool,
    pub error: ErrorDetail,
    pub timestamp: DateTime<Utc>,
    pub request_id: Option<String>,
    pub trace_id: Option<String>,
}

/// 错误详情
#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorDetail {
    pub code: String,
    pub message: String,
    pub details: Option<serde_json::Value>,
    pub retry_after: Option<u64>,
}

impl ErrorResponse {
    /// 从 AiStudioError 创建错误响应
    pub fn from_error(error: &AiStudioError) -> Self {
        let mut details = None;
        let mut retry_after = None;

        // 根据错误类型设置详细信息
        match error {
            AiStudioError::Database { code, .. } => {
                if let Some(code) = code {
                    details = Some(serde_json::json!({ "database_code": code }));
                }
            }
            AiStudioError::AiService { model, .. } => {
                if let Some(model) = model {
                    details = Some(serde_json::json!({ "model": model }));
                }
            }
            AiStudioError::Validation { field, .. } => {
                details = Some(serde_json::json!({ "field": field }));
            }
            AiStudioError::FileProcessing { file_name, .. } => {
                if let Some(file_name) = file_name {
                    details = Some(serde_json::json!({ "file_name": file_name }));
                }
            }
            AiStudioError::Tenant { tenant_id, .. } => {
                if let Some(tenant_id) = tenant_id {
                    details = Some(serde_json::json!({ "tenant_id": tenant_id }));
                }
            }
            AiStudioError::RateLimit { retry_after: ra } => {
                retry_after = *ra;
            }
            AiStudioError::ExternalService { service, .. } => {
                details = Some(serde_json::json!({ "service": service }));
            }
            AiStudioError::Timeout { operation } => {
                details = Some(serde_json::json!({ "operation": operation }));
            }
            _ => {}
        }

        Self {
            success: false,
            error: ErrorDetail {
                code: error.error_code().to_string(),
                message: error.to_string(),
                details,
                retry_after,
            },
            timestamp: Utc::now(),
            request_id: None,
            trace_id: None,
        }
    }

    /// 设置请求 ID
    pub fn with_request_id(mut self, request_id: String) -> Self {
        self.request_id = Some(request_id);
        self
    }

    /// 设置追踪 ID
    pub fn with_trace_id(mut self, trace_id: String) -> Self {
        self.trace_id = Some(trace_id);
        self
    }

    /// 转换为 HTTP 响应
    pub fn into_http_response(self) -> HttpResponse {
        let status_code = match self.error.code.as_str() {
            "CONFIGURATION_ERROR" => 500,
            "DATABASE_ERROR" => 500,
            "AI_SERVICE_ERROR" => 502,
            "CACHE_ERROR" => 500,
            "AUTHENTICATION_ERROR" => 401,
            "AUTHORIZATION_ERROR" => 403,
            "VALIDATION_ERROR" => 400,
            "NOT_FOUND" => 404,
            "CONFLICT" => 409,
            "RATE_LIMIT" => 429,
            "FILE_PROCESSING_ERROR" => 400,
            "VECTOR_ERROR" => 500,
            "TENANT_ERROR" => 400,
            "EXTERNAL_SERVICE_ERROR" => 502,
            "INTERNAL_ERROR" => 500,
            "SERVICE_UNAVAILABLE" => 503,
            "TIMEOUT_ERROR" => 408,
            _ => 500,
        };

        let mut response = HttpResponse::build(
            actix_web::http::StatusCode::from_u16(status_code)
                .unwrap_or(actix_web::http::StatusCode::INTERNAL_SERVER_ERROR),
        );

        // 添加重试头
        if let Some(retry_after) = self.error.retry_after {
            response.insert_header(("Retry-After", retry_after.to_string()));
        }

        // 添加请求 ID 头
        if let Some(ref request_id) = self.request_id {
            response.insert_header(("X-Request-ID", request_id.clone()));
        }

        // 添加追踪 ID 头
        if let Some(ref trace_id) = self.trace_id {
            response.insert_header(("X-Trace-ID", trace_id.clone()));
        }

        response.json(self)
    }

    /// 创建通用错误响应
    pub fn generic_error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            error: ErrorDetail {
                code: "INTERNAL_ERROR".to_string(),
                message: message.into(),
                details: None,
                retry_after: None,
            },
            timestamp: Utc::now(),
            request_id: None,
            trace_id: None,
        }
    }

    /// 创建验证错误响应
    pub fn validation_error(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            success: false,
            error: ErrorDetail {
                code: "VALIDATION_ERROR".to_string(),
                message: message.into(),
                details: Some(serde_json::json!({ "field": field.into() })),
                retry_after: None,
            },
            timestamp: Utc::now(),
            request_id: None,
            trace_id: None,
        }
    }

    /// 创建未找到错误响应
    pub fn not_found_error(resource: impl Into<String>) -> Self {
        Self {
            success: false,
            error: ErrorDetail {
                code: "NOT_FOUND".to_string(),
                message: format!("资源未找到: {}", resource.into()),
                details: None,
                retry_after: None,
            },
            timestamp: Utc::now(),
            request_id: None,
            trace_id: None,
        }
    }

    /// 创建限流错误响应
    pub fn rate_limit_error(retry_after: Option<u64>) -> Self {
        Self {
            success: false,
            error: ErrorDetail {
                code: "RATE_LIMIT".to_string(),
                message: "请求过于频繁，请稍后重试".to_string(),
                details: None,
                retry_after,
            },
            timestamp: Utc::now(),
            request_id: None,
            trace_id: None,
        }
    }
}

/// 成功响应辅助函数
pub fn success_response<T>(data: T) -> ApiResponse<T> {
    ApiResponse::success(data)
}

/// 成功响应（带消息）辅助函数
pub fn success_response_with_message<T>(data: T, message: String) -> ApiResponse<T> {
    ApiResponse::success_with_message(data, message)
}

/// 错误响应辅助函数
pub fn error_response(error: &AiStudioError) -> ErrorResponse {
    ErrorResponse::from_error(error)
}