// 日志上下文管理

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use uuid::Uuid;

/// 请求上下文
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestContext {
    pub request_id: String,
    pub trace_id: String,
    pub user_id: Option<String>,
    pub tenant_id: Option<String>,
    pub session_id: Option<String>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub method: Option<String>,
    pub path: Option<String>,
    pub query_params: Option<HashMap<String, String>>,
    pub start_time: chrono::DateTime<chrono::Utc>,
}

impl RequestContext {
    /// 创建新的请求上下文
    pub fn new() -> Self {
        Self {
            request_id: Uuid::new_v4().to_string(),
            trace_id: Uuid::new_v4().to_string(),
            user_id: None,
            tenant_id: None,
            session_id: None,
            ip_address: None,
            user_agent: None,
            method: None,
            path: None,
            query_params: None,
            start_time: chrono::Utc::now(),
        }
    }

    /// 从 HTTP 请求创建上下文
    pub fn from_http_request(req: &actix_web::HttpRequest) -> Self {
        let mut context = Self::new();

        // 设置请求信息
        context.method = Some(req.method().to_string());
        context.path = Some(req.path().to_string());

        // 设置 IP 地址
        context.ip_address = req
            .connection_info()
            .realip_remote_addr()
            .map(|s| s.to_string());

        // 设置 User-Agent
        context.user_agent = req
            .headers()
            .get("user-agent")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string());

        // 设置查询参数
        let query_params: HashMap<String, String> = req
            .query_string()
            .split('&')
            .filter_map(|pair| {
                let mut parts = pair.split('=');
                match (parts.next(), parts.next()) {
                    (Some(key), Some(value)) => Some((key.to_string(), value.to_string())),
                    _ => None,
                }
            })
            .collect();

        if !query_params.is_empty() {
            context.query_params = Some(query_params);
        }

        // 尝试从请求头获取现有的请求 ID 和追踪 ID
        if let Some(request_id) = req
            .headers()
            .get("X-Request-ID")
            .and_then(|h| h.to_str().ok())
        {
            context.request_id = request_id.to_string();
        }

        if let Some(trace_id) = req
            .headers()
            .get("X-Trace-ID")
            .and_then(|h| h.to_str().ok())
        {
            context.trace_id = trace_id.to_string();
        }

        context
    }

    /// 设置用户 ID
    pub fn with_user_id(mut self, user_id: String) -> Self {
        self.user_id = Some(user_id);
        self
    }

    /// 设置租户 ID
    pub fn with_tenant_id(mut self, tenant_id: String) -> Self {
        self.tenant_id = Some(tenant_id);
        self
    }

    /// 设置会话 ID
    pub fn with_session_id(mut self, session_id: String) -> Self {
        self.session_id = Some(session_id);
        self
    }

    /// 获取持续时间
    pub fn duration(&self) -> chrono::Duration {
        chrono::Utc::now() - self.start_time
    }

    /// 转换为日志字段
    pub fn to_log_fields(&self) -> Vec<(&'static str, String)> {
        let mut fields = vec![
            ("request_id", self.request_id.clone()),
            ("trace_id", self.trace_id.clone()),
            ("start_time", self.start_time.to_rfc3339()),
        ];

        if let Some(ref user_id) = self.user_id {
            fields.push(("user_id", user_id.clone()));
        }

        if let Some(ref tenant_id) = self.tenant_id {
            fields.push(("tenant_id", tenant_id.clone()));
        }

        if let Some(ref session_id) = self.session_id {
            fields.push(("session_id", session_id.clone()));
        }

        if let Some(ref ip_address) = self.ip_address {
            fields.push(("ip_address", ip_address.clone()));
        }

        if let Some(ref user_agent) = self.user_agent {
            fields.push(("user_agent", user_agent.clone()));
        }

        if let Some(ref method) = self.method {
            fields.push(("method", method.clone()));
        }

        if let Some(ref path) = self.path {
            fields.push(("path", path.clone()));
        }

        fields
    }
}

impl Default for RequestContext {
    fn default() -> Self {
        Self::new()
    }
}

/// 日志上下文宏
#[macro_export]
macro_rules! log_with_context {
    ($level:ident, $context:expr, $($arg:tt)*) => {
        tracing::$level!(
            request_id = %$context.request_id,
            trace_id = %$context.trace_id,
            user_id = ?$context.user_id,
            tenant_id = ?$context.tenant_id,
            $($arg)*
        );
    };
}

/// 创建带上下文的 span
#[macro_export]
macro_rules! span_with_context {
    ($level:ident, $name:expr, $context:expr) => {
        tracing::span!(
            tracing::Level::$level,
            $name,
            request_id = %$context.request_id,
            trace_id = %$context.trace_id,
            user_id = ?$context.user_id,
            tenant_id = ?$context.tenant_id,
        )
    };
    ($level:ident, $name:expr, $context:expr, $($field:tt)*) => {
        tracing::span!(
            tracing::Level::$level,
            $name,
            request_id = %$context.request_id,
            trace_id = %$context.trace_id,
            user_id = ?$context.user_id,
            tenant_id = ?$context.tenant_id,
            $($field)*
        )
    };
}

/// 性能监控宏
#[macro_export]
macro_rules! measure_time {
    ($name:expr, $context:expr, $block:block) => {{
        let start = std::time::Instant::now();
        let _span = $crate::span_with_context!(INFO, $name, $context);
        let _enter = _span.enter();
        
        let result = $block;
        
        let duration = start.elapsed();
        tracing::info!(
            operation = $name,
            duration_ms = duration.as_millis(),
            "操作完成"
        );
        
        result
    }};
}

/// 错误记录宏
#[macro_export]
macro_rules! log_error {
    ($context:expr, $error:expr, $($arg:tt)*) => {
        tracing::error!(
            request_id = %$context.request_id,
            trace_id = %$context.trace_id,
            user_id = ?$context.user_id,
            tenant_id = ?$context.tenant_id,
            error = %$error,
            error_type = std::any::type_name_of_val(&$error),
            $($arg)*
        );
    };
}