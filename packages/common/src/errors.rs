// 通用错误类型定义

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 通用错误类型
#[derive(Debug, Error, Serialize, Deserialize)]
pub enum CommonError {
    #[error("验证错误: {message}")]
    Validation { message: String },
    
    #[error("权限错误: {message}")]
    Permission { message: String },
    
    #[error("资源未找到: {resource}")]
    NotFound { resource: String },
    
    #[error("配置错误: {message}")]
    Configuration { message: String },
    
    #[error("外部服务错误: {service} - {message}")]
    ExternalService { service: String, message: String },
    
    #[error("内部错误: {message}")]
    Internal { message: String },
}

impl CommonError {
    pub fn validation(message: impl Into<String>) -> Self {
        Self::Validation {
            message: message.into(),
        }
    }
    
    pub fn permission(message: impl Into<String>) -> Self {
        Self::Permission {
            message: message.into(),
        }
    }
    
    pub fn not_found(resource: impl Into<String>) -> Self {
        Self::NotFound {
            resource: resource.into(),
        }
    }
    
    pub fn configuration(message: impl Into<String>) -> Self {
        Self::Configuration {
            message: message.into(),
        }
    }
    
    pub fn external_service(service: impl Into<String>, message: impl Into<String>) -> Self {
        Self::ExternalService {
            service: service.into(),
            message: message.into(),
        }
    }
    
    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal {
            message: message.into(),
        }
    }
    
    /// 获取错误代码
    pub fn error_code(&self) -> &'static str {
        match self {
            Self::Validation { .. } => "VALIDATION_ERROR",
            Self::Permission { .. } => "PERMISSION_ERROR",
            Self::NotFound { .. } => "NOT_FOUND",
            Self::Configuration { .. } => "CONFIGURATION_ERROR",
            Self::ExternalService { .. } => "EXTERNAL_SERVICE_ERROR",
            Self::Internal { .. } => "INTERNAL_ERROR",
        }
    }
}