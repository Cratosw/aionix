// 统一错误类型定义

use actix_web::{HttpResponse, ResponseError};
use aionix_common::CommonError;
use serde::{Deserialize, Serialize};

use thiserror::Error;
use tracing::error;

/// AI Studio 统一错误类型
#[derive(Debug, Error, Serialize, Deserialize)]
#[serde(tag = "error_type", content = "details")]
pub enum AiStudioError {
    /// 配置错误
    #[error("配置错误: {message}")]
    Configuration { message: String },

    /// 数据库错误
    #[error("数据库错误: {message}")]
    Database { message: String, code: Option<String> },

    /// AI 服务错误
    #[error("AI 服务错误: {message}")]
    AiService { message: String, model: Option<String> },

    /// 缓存错误
    #[cfg(feature = "redis")]
    #[error("缓存错误: {message}")]
    Cache { message: String },

    /// 认证错误
    #[error("认证错误: {message}")]
    Authentication { message: String },

    /// 授权错误
    #[error("授权错误: {message}")]
    Authorization { message: String },

    /// 验证错误
    #[error("验证错误: {field} - {message}")]
    Validation { field: String, message: String },

    /// 资源未找到
    #[error("资源未找到: {resource}")]
    NotFound { resource: String },

    /// 资源冲突
    #[error("资源冲突: {message}")]
    Conflict { message: String },

    /// 限流错误
    #[error("请求过于频繁，请稍后重试")]
    RateLimit { retry_after: Option<u64> },

    /// 文件处理错误
    #[error("文件处理错误: {message}")]
    FileProcessing { message: String, file_name: Option<String> },

    /// 向量数据库错误
    #[error("向量数据库错误: {message}")]
    Vector { message: String },

    /// 租户错误
    #[error("租户错误: {message}")]
    Tenant { message: String, tenant_id: Option<String> },

    /// 外部服务错误
    #[error("外部服务错误: {service} - {message}")]
    ExternalService { service: String, message: String },

    /// 内部服务器错误
    #[error("内部服务器错误: {message}")]
    Internal { message: String },

    /// 服务不可用
    #[error("服务暂时不可用: {message}")]
    ServiceUnavailable { message: String },

    /// 超时错误
    #[error("请求超时: {operation}")]
    Timeout { operation: String },
}

impl AiStudioError {
    /// 获取错误代码
    pub fn error_code(&self) -> &'static str {
        match self {
            Self::Configuration { .. } => "CONFIGURATION_ERROR",
            Self::Database { .. } => "DATABASE_ERROR",
            Self::AiService { .. } => "AI_SERVICE_ERROR",
            #[cfg(feature = "redis")]
            Self::Cache { .. } => "CACHE_ERROR",
            Self::Authentication { .. } => "AUTHENTICATION_ERROR",
            Self::Authorization { .. } => "AUTHORIZATION_ERROR",
            Self::Validation { .. } => "VALIDATION_ERROR",
            Self::NotFound { .. } => "NOT_FOUND",
            Self::Conflict { .. } => "CONFLICT",
            Self::RateLimit { .. } => "RATE_LIMIT",
            Self::FileProcessing { .. } => "FILE_PROCESSING_ERROR",
            Self::Vector { .. } => "VECTOR_ERROR",
            Self::Tenant { .. } => "TENANT_ERROR",
            Self::ExternalService { .. } => "EXTERNAL_SERVICE_ERROR",
            Self::Internal { .. } => "INTERNAL_ERROR",
            Self::ServiceUnavailable { .. } => "SERVICE_UNAVAILABLE",
            Self::Timeout { .. } => "TIMEOUT_ERROR",
        }
    }

    /// 获取 HTTP 状态码
    pub fn status_code(&self) -> u16 {
        match self {
            Self::Configuration { .. } => 500,
            Self::Database { .. } => 500,
            Self::AiService { .. } => 502,
            #[cfg(feature = "redis")]
            Self::Cache { .. } => 500,
            Self::Authentication { .. } => 401,
            Self::Authorization { .. } => 403,
            Self::Validation { .. } => 400,
            Self::NotFound { .. } => 404,
            Self::Conflict { .. } => 409,
            Self::RateLimit { .. } => 429,
            Self::FileProcessing { .. } => 400,
            Self::Vector { .. } => 500,
            Self::Tenant { .. } => 400,
            Self::ExternalService { .. } => 502,
            Self::Internal { .. } => 500,
            Self::ServiceUnavailable { .. } => 503,
            Self::Timeout { .. } => 408,
        }
    }

    /// 是否为客户端错误
    pub fn is_client_error(&self) -> bool {
        matches!(self.status_code(), 400..=499)
    }

    /// 是否为服务器错误
    pub fn is_server_error(&self) -> bool {
        matches!(self.status_code(), 500..=599)
    }

    /// 是否应该记录错误日志
    pub fn should_log(&self) -> bool {
        match self {
            Self::Validation { .. } | Self::NotFound { .. } | Self::Authentication { .. } => false,
            _ => true,
        }
    }

    /// 创建配置错误
    pub fn configuration(message: impl Into<String>) -> Self {
        Self::Configuration {
            message: message.into(),
        }
    }

    /// 创建数据库错误
    pub fn database(message: impl Into<String>) -> Self {
        Self::Database {
            message: message.into(),
            code: None,
        }
    }

    /// 创建数据库错误（带错误代码）
    pub fn database_with_code(message: impl Into<String>, code: impl Into<String>) -> Self {
        Self::Database {
            message: message.into(),
            code: Some(code.into()),
        }
    }

    /// 创建 AI 服务错误
    pub fn ai_service(message: impl Into<String>) -> Self {
        Self::AiService {
            message: message.into(),
            model: None,
        }
    }

    /// 创建 AI 服务错误（带模型信息）
    pub fn ai_service_with_model(message: impl Into<String>, model: impl Into<String>) -> Self {
        Self::AiService {
            message: message.into(),
            model: Some(model.into()),
        }
    }

    /// 创建缓存错误
    #[cfg(feature = "redis")]
    pub fn cache(message: impl Into<String>) -> Self {
        Self::Cache {
            message: message.into(),
        }
    }

    /// 创建认证错误
    pub fn authentication(message: impl Into<String>) -> Self {
        Self::Authentication {
            message: message.into(),
        }
    }

    /// 创建授权错误
    pub fn authorization(message: impl Into<String>) -> Self {
        Self::Authorization {
            message: message.into(),
        }
    }

    /// 创建验证错误
    pub fn validation(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Validation {
            field: field.into(),
            message: message.into(),
        }
    }

    /// 创建资源未找到错误
    pub fn not_found(resource: impl Into<String>) -> Self {
        Self::NotFound {
            resource: resource.into(),
        }
    }

    /// 创建冲突错误
    pub fn conflict(message: impl Into<String>) -> Self {
        Self::Conflict {
            message: message.into(),
        }
    }

    /// 创建限流错误
    pub fn rate_limit(retry_after: Option<u64>) -> Self {
        Self::RateLimit { retry_after }
    }

    /// 创建文件处理错误
    pub fn file_processing(message: impl Into<String>) -> Self {
        Self::FileProcessing {
            message: message.into(),
            file_name: None,
        }
    }

    /// 创建文件处理错误（带文件名）
    pub fn file_processing_with_name(
        message: impl Into<String>,
        file_name: impl Into<String>,
    ) -> Self {
        Self::FileProcessing {
            message: message.into(),
            file_name: Some(file_name.into()),
        }
    }

    /// 创建向量数据库错误
    pub fn vector(message: impl Into<String>) -> Self {
        Self::Vector {
            message: message.into(),
        }
    }

    /// 创建租户错误
    pub fn tenant(message: impl Into<String>) -> Self {
        Self::Tenant {
            message: message.into(),
            tenant_id: None,
        }
    }

    /// 创建租户错误（带租户 ID）
    pub fn tenant_with_id(message: impl Into<String>, tenant_id: impl Into<String>) -> Self {
        Self::Tenant {
            message: message.into(),
            tenant_id: Some(tenant_id.into()),
        }
    }

    /// 创建外部服务错误
    pub fn external_service(service: impl Into<String>, message: impl Into<String>) -> Self {
        Self::ExternalService {
            service: service.into(),
            message: message.into(),
        }
    }

    /// 创建内部错误
    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal {
            message: message.into(),
        }
    }

    /// 创建服务不可用错误
    pub fn service_unavailable(message: impl Into<String>) -> Self {
        Self::ServiceUnavailable {
            message: message.into(),
        }
    }

    /// 创建超时错误
    pub fn timeout(operation: impl Into<String>) -> Self {
        Self::Timeout {
            operation: operation.into(),
        }
    }

    /// 创建未授权错误
    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::Authentication {
            message: message.into(),
        }
    }

    /// 创建禁止访问错误
    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::Authorization {
            message: message.into(),
        }
    }

    /// 创建配额超限错误
    pub fn quota_exceeded(message: impl Into<String>) -> Self {
        Self::RateLimit { retry_after: None }
    }

    /// 创建请求过多错误
    pub fn too_many_requests(message: impl Into<String>) -> Self {
        Self::RateLimit { retry_after: Some(60) }
    }

    /// 创建简单验证错误
    pub fn validation_simple(message: impl Into<String>) -> Self {
        Self::Validation {
            field: "general".to_string(),
            message: message.into(),
        }
    }
}

/// 实现 ResponseError trait 以便与 Actix Web 集成
impl ResponseError for AiStudioError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        actix_web::http::StatusCode::from_u16(self.status_code())
            .unwrap_or(actix_web::http::StatusCode::INTERNAL_SERVER_ERROR)
    }

    fn error_response(&self) -> HttpResponse {
        // 记录错误日志
        if self.should_log() {
            error!(
                error_code = %self.error_code(),
                error_message = %self,
                "处理请求时发生错误"
            );
        }

        // 构建错误响应
        crate::errors::ErrorResponse::from_error(self).into_http_response()
    }
}

/// 从 CommonError 转换
impl From<CommonError> for AiStudioError {
    fn from(err: CommonError) -> Self {
        match err {
            CommonError::Validation { message } => Self::validation("general", message),
            CommonError::Permission { message } => Self::authorization(message),
            CommonError::NotFound { resource } => Self::not_found(resource),
            CommonError::Configuration { message } => Self::configuration(message),
            CommonError::ExternalService { service, message } => {
                Self::external_service(service, message)
            }
            CommonError::Internal { message } => Self::internal(message),
        }
    }
}

/// 从 sea_orm::DbErr 转换
impl From<sea_orm::DbErr> for AiStudioError {
    fn from(err: sea_orm::DbErr) -> Self {
        match err {
            sea_orm::DbErr::ConnectionAcquire(_) => {
                Self::database("无法获取数据库连接")
            }
            sea_orm::DbErr::TryIntoErr { .. } => {
                Self::database("数据类型转换错误")
            }
            sea_orm::DbErr::Conn(msg) => {
                Self::database(format!("数据库连接错误: {}", msg))
            }
            sea_orm::DbErr::Exec(msg) => {
                Self::database(format!("数据库执行错误: {}", msg))
            }
            sea_orm::DbErr::Query(msg) => {
                Self::database(format!("数据库查询错误: {}", msg))
            }
            _ => Self::database(format!("数据库错误: {}", err)),
        }
    }
}

/// 从 config::ConfigError 转换
impl From<config::ConfigError> for AiStudioError {
    fn from(err: config::ConfigError) -> Self {
        Self::configuration(format!("配置加载错误: {}", err))
    }
}

/// 从 std::io::Error 转换
impl From<std::io::Error> for AiStudioError {
    fn from(err: std::io::Error) -> Self {
        match err.kind() {
            std::io::ErrorKind::NotFound => Self::not_found("文件或目录"),
            std::io::ErrorKind::PermissionDenied => Self::authorization("文件访问权限不足"),
            std::io::ErrorKind::TimedOut => Self::timeout("文件操作"),
            _ => Self::internal(format!("IO 错误: {}", err)),
        }
    }
}

/// 从 serde_json::Error 转换
impl From<serde_json::Error> for AiStudioError {
    fn from(err: serde_json::Error) -> Self {
        Self::validation("json", format!("JSON 解析错误: {}", err))
    }
}

/// 从 uuid::Error 转换
impl From<uuid::Error> for AiStudioError {
    fn from(err: uuid::Error) -> Self {
        Self::validation("uuid", format!("UUID 格式错误: {}", err))
    }
}