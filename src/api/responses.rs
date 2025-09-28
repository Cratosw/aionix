use actix_web::{HttpResponse, Result as ActixResult};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// 统一 API 响应结构
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ApiResponse<T> {
    /// 是否成功
    pub success: bool,
    /// 响应数据
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    /// 错误信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ApiError>,
    /// 请求 ID
    pub request_id: String,
    /// 响应时间戳
    pub timestamp: DateTime<Utc>,
    /// API 版本
    pub version: String,
}

impl<T> ApiResponse<T> {
    /// 创建成功响应
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            request_id: generate_request_id(),
            timestamp: Utc::now(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
    
    /// 创建创建成功响应
    pub fn created(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            request_id: generate_request_id(),
            timestamp: Utc::now(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
    
    /// 创建无内容响应
    pub fn no_content() -> Self {
        Self {
            success: true,
            data: None,
            error: None,
            request_id: generate_request_id(),
            timestamp: Utc::now(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
    
    /// 创建接受响应
    pub fn accepted(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            request_id: generate_request_id(),
            timestamp: Utc::now(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

/// API 错误信息
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ApiError {
    /// 错误代码
    pub code: String,
    /// 错误消息
    pub message: String,
    /// 错误详情
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
    /// 错误字段（用于表单验证错误）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    /// 帮助链接
    #[serde(skip_serializing_if = "Option::is_none")]
    pub help_url: Option<String>,
}

impl ApiError {
    /// 创建错误响应
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self {
            code: "BAD_REQUEST".to_string(),
            message: message.into(),
            details: None,
            field: None,
            help_url: None,
        }
    }
    
    /// 创建内部服务器错误
    pub fn internal_server_error(message: impl Into<String>) -> Self {
        Self {
            code: "INTERNAL_ERROR".to_string(),
            message: message.into(),
            details: None,
            field: None,
            help_url: None,
        }
    }
    
    /// 创建资源不存在错误响应
    pub fn not_found(message: impl Into<String>) -> Self {
        Self {
            code: "NOT_FOUND".to_string(),
            message: message.into(),
            details: None,
            field: None,
            help_url: None,
        }
    }
    
    /// 创建冲突错误响应
    pub fn conflict(message: impl Into<String>) -> Self {
        Self {
            code: "CONFLICT".to_string(),
            message: message.into(),
            details: None,
            field: None,
            help_url: None,
        }
    }
    
    /// 创建请求实体过大错误响应
    pub fn payload_too_large(message: impl Into<String>) -> Self {
        Self {
            code: "PAYLOAD_TOO_LARGE".to_string(),
            message: message.into(),
            details: None,
            field: None,
            help_url: None,
        }
    }
    
    /// 创建接受响应
    pub fn accepted(message: impl Into<String>) -> Self {
        Self {
            code: "ACCEPTED".to_string(),
            message: message.into(),
            details: None,
            field: None,
            help_url: None,
        }
    }
}

impl actix_web::ResponseError for ApiError {
    fn error_response(&self) -> HttpResponse {
        let status_code = match self.code.as_str() {
            "BAD_REQUEST" => actix_web::http::StatusCode::BAD_REQUEST,
            "UNAUTHORIZED" => actix_web::http::StatusCode::UNAUTHORIZED,
            "FORBIDDEN" => actix_web::http::StatusCode::FORBIDDEN,
            "NOT_FOUND" => actix_web::http::StatusCode::NOT_FOUND,
            "CONFLICT" => actix_web::http::StatusCode::CONFLICT,
            "PAYLOAD_TOO_LARGE" => actix_web::http::StatusCode::PAYLOAD_TOO_LARGE,
            "UNPROCESSABLE_ENTITY" => actix_web::http::StatusCode::UNPROCESSABLE_ENTITY,
            "TOO_MANY_REQUESTS" => actix_web::http::StatusCode::TOO_MANY_REQUESTS,
            "INTERNAL_ERROR" => actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
            _ => actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
        };

        HttpResponse::build(status_code).json(self)
    }
}

/// 成功响应构建器
pub struct SuccessResponse;

impl SuccessResponse {
    /// 创建成功响应
    pub fn ok<T: Serialize>(data: T) -> ApiResponse<T> {
        ApiResponse {
            success: true,
            data: Some(data),
            error: None,
            request_id: generate_request_id(),
            timestamp: Utc::now(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    /// 创建无数据成功响应
    pub fn no_content() -> ApiResponse<()> {
        ApiResponse {
            success: true,
            data: None,
            error: None,
            request_id: generate_request_id(),
            timestamp: Utc::now(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    /// 创建创建成功响应
    pub fn created<T: Serialize>(data: T) -> ApiResponse<T> {
        ApiResponse {
            success: true,
            data: Some(data),
            error: None,
            request_id: generate_request_id(),
            timestamp: Utc::now(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    /// 创建接受响应
    pub fn accepted<T: Serialize>(data: T) -> ApiResponse<T> {
        ApiResponse {
            success: true,
            data: Some(data),
            error: None,
            request_id: generate_request_id(),
            timestamp: Utc::now(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

/// 错误响应构建器
pub struct ErrorResponse;

impl ErrorResponse {
    /// 创建错误响应
    pub fn error<T>(code: String, message: String) -> ApiResponse<T> {
        ApiResponse {
            success: false,
            data: None,
            error: Some(ApiError {
                code,
                message,
                details: None,
                field: None,
                help_url: None,
            }),
            request_id: generate_request_id(),
            timestamp: Utc::now(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    /// 创建详细错误响应
    pub fn detailed_error<T>(
        code: String,
        message: String,
        details: Option<serde_json::Value>,
        field: Option<String>,
    ) -> ApiResponse<T> {
        ApiResponse {
            success: false,
            data: None,
            error: Some(ApiError {
                code,
                message,
                details,
                field,
                help_url: None,
            }),
            request_id: generate_request_id(),
            timestamp: Utc::now(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    /// 创建验证错误响应
    pub fn validation_error<T>(field: String, message: String) -> ApiResponse<T> {
        ApiResponse {
            success: false,
            data: None,
            error: Some(ApiError {
                code: "VALIDATION_ERROR".to_string(),
                message,
                details: None,
                field: Some(field),
                help_url: Some("https://docs.aionix.ai/api/validation".to_string()),
            }),
            request_id: generate_request_id(),
            timestamp: Utc::now(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    /// 创建未授权错误响应
    pub fn unauthorized<T>() -> ApiResponse<T> {
        ApiResponse {
            success: false,
            data: None,
            error: Some(ApiError {
                code: "UNAUTHORIZED".to_string(),
                message: "未授权访问".to_string(),
                details: None,
                field: None,
                help_url: Some("https://docs.aionix.ai/api/authentication".to_string()),
            }),
            request_id: generate_request_id(),
            timestamp: Utc::now(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    /// 创建禁止访问错误响应
    pub fn forbidden<T>(message: &str) -> ApiResponse<T> {
        ApiResponse {
            success: false,
            data: None,
            error: Some(ApiError {
                code: "FORBIDDEN".to_string(),
                message: message.to_string(),
                details: None,
                field: None,
                help_url: Some("https://docs.aionix.ai/api/permissions".to_string()),
            }),
            request_id: generate_request_id(),
            timestamp: Utc::now(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    /// 创建资源不存在错误响应
    pub fn not_found<T>(resource: &str) -> ApiResponse<T> {
        ApiResponse {
            success: false,
            data: None,
            error: Some(ApiError {
                code: "NOT_FOUND".to_string(),
                message: format!("{} 不存在", resource),
                details: None,
                field: None,
                help_url: None,
            }),
            request_id: generate_request_id(),
            timestamp: Utc::now(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    /// 创建冲突错误响应
    pub fn conflict<T>(message: String) -> ApiResponse<T> {
        ApiResponse {
            success: false,
            data: None,
            error: Some(ApiError {
                code: "CONFLICT".to_string(),
                message,
                details: None,
                field: None,
                help_url: None,
            }),
            request_id: generate_request_id(),
            timestamp: Utc::now(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    /// 创建配额超限错误响应
    pub fn quota_exceeded<T>(resource: &str) -> ApiResponse<T> {
        ApiResponse {
            success: false,
            data: None,
            error: Some(ApiError {
                code: "QUOTA_EXCEEDED".to_string(),
                message: format!("{} 配额已超限", resource),
                details: None,
                field: None,
                help_url: Some("https://docs.aionix.ai/api/quotas".to_string()),
            }),
            request_id: generate_request_id(),
            timestamp: Utc::now(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    /// 创建限流错误响应
    pub fn rate_limited<T>() -> ApiResponse<T> {
        ApiResponse {
            success: false,
            data: None,
            error: Some(ApiError {
                code: "RATE_LIMITED".to_string(),
                message: "请求频率过高，请稍后重试".to_string(),
                details: None,
                field: None,
                help_url: Some("https://docs.aionix.ai/api/rate-limits".to_string()),
            }),
            request_id: generate_request_id(),
            timestamp: Utc::now(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    /// 创建内部服务器错误响应
    pub fn internal_server_error<T>(message: &str) -> ApiResponse<T> {
        ApiResponse {
            success: false,
            data: None,
            error: Some(ApiError {
                code: "INTERNAL_ERROR".to_string(),
                message: message.to_string(),
                details: None,
                field: None,
                help_url: Some("https://docs.aionix.ai/api/errors".to_string()),
            }),
            request_id: generate_request_id(),
            timestamp: Utc::now(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

/// HTTP 响应构建器
pub struct HttpResponseBuilder;

impl HttpResponseBuilder {
    /// 创建 200 OK 响应
    pub fn ok<T: Serialize>(data: T) -> ActixResult<HttpResponse> {
        Ok(HttpResponse::Ok().json(SuccessResponse::ok(data)))
    }

    /// 创建 201 Created 响应
    pub fn created<T: Serialize>(data: T) -> ActixResult<HttpResponse> {
        Ok(HttpResponse::Created().json(SuccessResponse::created(data)))
    }

    /// 创建 204 No Content 响应
    pub fn no_content() -> ActixResult<HttpResponse> {
        Ok(HttpResponse::NoContent().json(SuccessResponse::no_content()))
    }

    /// 创建 400 Bad Request 响应
    pub fn bad_request<T: serde::Serialize>(message: String) -> ActixResult<HttpResponse> {
        Ok(HttpResponse::BadRequest().json(ErrorResponse::error::<T>(
            "BAD_REQUEST".to_string(),
            message,
        )))
    }

    /// 创建 401 Unauthorized 响应
    pub fn unauthorized<T: serde::Serialize>() -> ActixResult<HttpResponse> {
        Ok(HttpResponse::Unauthorized().json(ErrorResponse::unauthorized::<T>()))
    }

    /// 创建 403 Forbidden 响应
    pub fn forbidden<T: serde::Serialize>(message: &str) -> ActixResult<HttpResponse> {
        Ok(HttpResponse::Forbidden().json(ErrorResponse::forbidden::<T>(message)))
    }

    /// 创建 404 Not Found 响应
    pub fn not_found<T: serde::Serialize>(resource: &str) -> ActixResult<HttpResponse> {
        Ok(HttpResponse::NotFound().json(ErrorResponse::not_found::<T>(resource)))
    }

    /// 创建 409 Conflict 响应
    pub fn conflict<T: serde::Serialize>(message: String) -> ActixResult<HttpResponse> {
        Ok(HttpResponse::Conflict().json(ErrorResponse::conflict::<T>(message)))
    }

    /// 创建 413 Payload Too Large 响应
    pub fn payload_too_large<T: serde::Serialize>(message: &str) -> ActixResult<HttpResponse> {
        Ok(HttpResponse::PayloadTooLarge().json(ErrorResponse::error::<T>(
            "PAYLOAD_TOO_LARGE".to_string(),
            message.to_string(),
        )))
    }

    /// 创建 422 Unprocessable Entity 响应
    pub fn validation_error<T: serde::Serialize>(field: String, message: String) -> ActixResult<HttpResponse> {
        Ok(HttpResponse::UnprocessableEntity().json(ErrorResponse::validation_error::<T>(field, message)))
    }

    /// 创建 429 Too Many Requests 响应
    pub fn rate_limited<T: serde::Serialize>() -> ActixResult<HttpResponse> {
        Ok(HttpResponse::TooManyRequests().json(ErrorResponse::rate_limited::<T>()))
    }

    /// 创建 500 Internal Server Error 响应
    pub fn internal_error<T: serde::Serialize>() -> ActixResult<HttpResponse> {
        Ok(HttpResponse::InternalServerError().json(ErrorResponse::internal_server_error::<T>()))
    }
}

/// 生成请求 ID
fn generate_request_id() -> String {
    Uuid::new_v4().to_string()
}

/// API 响应扩展 trait
pub trait ApiResponseExt<T> {
    /// 转换为 HTTP 响应
    fn into_http_response(self) -> ActixResult<HttpResponse>;
    /// 转换为 HTTP 响应（into）
    fn into(self) -> ActixResult<HttpResponse> {
        self.into_http_response()
    }
}

impl<T: Serialize> ApiResponseExt<T> for ApiResponse<T> {
    fn into_http_response(self) -> ActixResult<HttpResponse> {
        let status_code = if self.success {
            actix_web::http::StatusCode::OK
        } else {
            match self.error.as_ref().map(|e| e.code.as_str()) {
                Some("VALIDATION_ERROR") => actix_web::http::StatusCode::BAD_REQUEST,
                Some("UNAUTHORIZED") => actix_web::http::StatusCode::UNAUTHORIZED,
                Some("FORBIDDEN") => actix_web::http::StatusCode::FORBIDDEN,
                Some("NOT_FOUND") => actix_web::http::StatusCode::NOT_FOUND,
                Some("CONFLICT") => actix_web::http::StatusCode::CONFLICT,
                Some("QUOTA_EXCEEDED") | Some("RATE_LIMITED") => actix_web::http::StatusCode::TOO_MANY_REQUESTS,
                Some("INTERNAL_ERROR") => actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
                _ => actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
            }
        };

        Ok(HttpResponse::build(status_code).json(self))
    }
}

/// 为 ApiError 实现 Display trait
impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ApiError {{ code: {}, message: {} }}", self.code, self.message)
    }
}

/// 为 ApiResponse 实现 Display trait
impl<T: std::fmt::Debug> std::fmt::Display for ApiResponse<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ApiResponse {{ success: {}, data: {:?}, error: {:?} }}", 
               self.success, self.data, self.error)
    }
}