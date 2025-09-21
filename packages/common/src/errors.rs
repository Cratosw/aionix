// 通用错误类型定义

use serde::{Deserialize, Serialize};
use std::fmt;

/// 通用错误类型
#[derive(Debug, Serialize, Deserialize)]
pub struct CommonError {
    pub code: String,
    pub message: String,
    pub details: Option<String>,
}

impl fmt::Display for CommonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for CommonError {}

impl CommonError {
    pub fn new(code: &str, message: &str) -> Self {
        Self {
            code: code.to_string(),
            message: message.to_string(),
            details: None,
        }
    }
    
    pub fn with_details(code: &str, message: &str, details: &str) -> Self {
        Self {
            code: code.to_string(),
            message: message.to_string(),
            details: Some(details.to_string()),
        }
    }
}