// API 请求提取器
// 定义自定义的请求提取器，用于处理认证、租户上下文等

use actix_web::{dev::Payload, web, FromRequest, HttpRequest, Result as ActixResult};
use futures::future::{Ready, ready};
use serde::Deserialize;
use uuid::Uuid;
use std::pin::Pin;
use std::future::Future;

use crate::api::responses::{ErrorResponse, HttpResponseBuilder};
use crate::db::migrations::tenant_filter::TenantContext;
use crate::errors::AiStudioError;

/// 租户上下文提取器
#[derive(Debug, Clone)]
pub struct TenantExtractor {
    pub context: TenantContext,
}

impl FromRequest for TenantExtractor {
    type Error = actix_web::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let req = req.clone();
        
        Box::pin(async move {
            // 从请求头获取租户 ID
            let tenant_id = req
                .headers()
                .get("X-Tenant-ID")
                .and_then(|h| h.to_str().ok())
                .and_then(|s| Uuid::parse_str(s).ok());

            // 从子域名获取租户信息
            let host = req
                .headers()
                .get("Host")
                .and_then(|h| h.to_str().ok())
                .unwrap_or("");

            let subdomain = extract_subdomain(host);

            // 从用户认证信息获取租户上下文（这里需要实现 JWT 解析）
            let auth_header = req
                .headers()
                .get("Authorization")
                .and_then(|h| h.to_str().ok());

            // 构建租户上下文
            let context = if let Some(tenant_id) = tenant_id {
                TenantContext::new(tenant_id, subdomain.unwrap_or("unknown".to_string()), false)
            } else if let Some(subdomain) = subdomain {
                // 这里应该根据子域名查询数据库获取租户信息
                // 为了简化，这里生成一个示例上下文
                TenantContext::new(Uuid::new_v4(), subdomain, false)
            } else {
                return Err(actix_web::error::ErrorUnauthorized("无法确定租户上下文"));
            };

            Ok(TenantExtractor { context })
        })
    }
}

/// 用户认证提取器
#[derive(Debug, Clone)]
pub struct AuthExtractor {
    pub user_id: Uuid,
    pub tenant_id: Uuid,
    pub username: String,
    pub role: String,
    pub permissions: Vec<String>,
}

impl FromRequest for AuthExtractor {
    type Error = actix_web::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let req = req.clone();
        
        Box::pin(async move {
            let auth_header = req
                .headers()
                .get("Authorization")
                .and_then(|h| h.to_str().ok())
                .ok_or_else(|| actix_web::error::ErrorUnauthorized("缺少认证头"))?;

            if !auth_header.starts_with("Bearer ") {
                return Err(actix_web::error::ErrorUnauthorized("无效的认证格式"));
            }

            let token = &auth_header[7..];
            
            // 这里应该验证 JWT 令牌并提取用户信息
            // 为了简化，这里返回一个示例用户
            let user = AuthExtractor {
                user_id: Uuid::new_v4(),
                tenant_id: Uuid::new_v4(),
                username: "demo_user".to_string(),
                role: "user".to_string(),
                permissions: vec!["read".to_string(), "write".to_string()],
            };

            Ok(user)
        })
    }
}

/// 可选的用户认证提取器
#[derive(Debug, Clone)]
pub struct OptionalAuthExtractor {
    pub user: Option<AuthExtractor>,
}

impl FromRequest for OptionalAuthExtractor {
    type Error = actix_web::Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        match AuthExtractor::from_request(req, payload) {
            fut => {
                // 由于 AuthExtractor 返回 Future，我们需要处理这个异步情况
                // 这里简化处理，实际应该正确处理异步
                ready(Ok(OptionalAuthExtractor { user: None }))
            }
        }
    }
}

/// 管理员权限提取器
#[derive(Debug, Clone)]
pub struct AdminExtractor {
    pub user: AuthExtractor,
}

impl FromRequest for AdminExtractor {
    type Error = actix_web::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        let auth_future = AuthExtractor::from_request(req, payload);
        
        Box::pin(async move {
            let user = auth_future.await?;
            
            if user.role != "admin" && !user.permissions.contains(&"admin".to_string()) {
                return Err(actix_web::error::ErrorForbidden("需要管理员权限"));
            }

            Ok(AdminExtractor { user })
        })
    }
}

/// API 密钥提取器
#[derive(Debug, Clone)]
pub struct ApiKeyExtractor {
    pub key: String,
    pub tenant_id: Uuid,
    pub permissions: Vec<String>,
}

impl FromRequest for ApiKeyExtractor {
    type Error = actix_web::Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let api_key = req
            .headers()
            .get("X-API-Key")
            .and_then(|h| h.to_str().ok());

        match api_key {
            Some(key) => {
                // 这里应该验证 API 密钥并获取相关权限
                // 为了简化，这里返回一个示例
                ready(Ok(ApiKeyExtractor {
                    key: key.to_string(),
                    tenant_id: Uuid::new_v4(),
                    permissions: vec!["api_access".to_string()],
                }))
            }
            None => ready(Err(actix_web::error::ErrorUnauthorized("缺少 API 密钥"))),
        }
    }
}

/// 分页参数提取器
#[derive(Debug, Clone, Deserialize)]
pub struct PaginationExtractor {
    #[serde(default = "default_page")]
    pub page: u32,
    #[serde(default = "default_page_size")]
    pub page_size: u32,
    pub sort_by: Option<String>,
    #[serde(default = "default_sort_order")]
    pub sort_order: String,
}

impl FromRequest for PaginationExtractor {
    type Error = actix_web::Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let query_string = req.query_string();
        
        match serde_urlencoded::from_str::<PaginationExtractor>(query_string) {
            Ok(mut pagination) => {
                // 验证和修正参数
                if pagination.page == 0 {
                    pagination.page = 1;
                }
                if pagination.page_size == 0 {
                    pagination.page_size = 20;
                }
                if pagination.page_size > 100 {
                    pagination.page_size = 100;
                }
                ready(Ok(pagination))
            }
            Err(_) => {
                // 使用默认值
                ready(Ok(PaginationExtractor {
                    page: 1,
                    page_size: 20,
                    sort_by: None,
                    sort_order: "desc".to_string(),
                }))
            }
        }
    }
}

/// 搜索参数提取器
#[derive(Debug, Clone, Deserialize)]
pub struct SearchExtractor {
    pub q: Option<String>,
    pub fields: Option<String>,
    pub filters: Option<String>,
    #[serde(flatten)]
    pub pagination: PaginationExtractor,
}

impl FromRequest for SearchExtractor {
    type Error = actix_web::Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let query_string = req.query_string();
        
        match serde_urlencoded::from_str::<SearchExtractor>(query_string) {
            Ok(search) => ready(Ok(search)),
            Err(_) => {
                ready(Ok(SearchExtractor {
                    q: None,
                    fields: None,
                    filters: None,
                    pagination: PaginationExtractor {
                        page: 1,
                        page_size: 20,
                        sort_by: None,
                        sort_order: "desc".to_string(),
                    },
                }))
            }
        }
    }
}

/// 请求 ID 提取器
#[derive(Debug, Clone)]
pub struct RequestIdExtractor {
    pub request_id: String,
}

impl FromRequest for RequestIdExtractor {
    type Error = actix_web::Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let request_id = req
            .headers()
            .get("X-Request-ID")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string())
            .unwrap_or_else(|| Uuid::new_v4().to_string());

        ready(Ok(RequestIdExtractor { request_id }))
    }
}

/// 内容类型验证提取器
#[derive(Debug, Clone)]
pub struct JsonContentTypeExtractor;

impl FromRequest for JsonContentTypeExtractor {
    type Error = actix_web::Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let content_type = req
            .headers()
            .get("Content-Type")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("");

        if content_type.starts_with("application/json") {
            ready(Ok(JsonContentTypeExtractor))
        } else {
            ready(Err(actix_web::error::ErrorBadRequest("需要 JSON 内容类型")))
        }
    }
}

// 辅助函数

/// 从主机名提取子域名
fn extract_subdomain(host: &str) -> Option<String> {
    let parts: Vec<&str> = host.split('.').collect();
    if parts.len() >= 3 {
        Some(parts[0].to_string())
    } else {
        None
    }
}

/// 默认页码
fn default_page() -> u32 {
    1
}

/// 默认页面大小
fn default_page_size() -> u32 {
    20
}

/// 默认排序顺序
fn default_sort_order() -> String {
    "desc".to_string()
}

impl PaginationExtractor {
    /// 计算偏移量
    pub fn offset(&self) -> u64 {
        ((self.page - 1) * self.page_size) as u64
    }

    /// 获取限制数量
    pub fn limit(&self) -> u64 {
        self.page_size as u64
    }
}

impl SearchExtractor {
    /// 解析搜索字段
    pub fn parse_fields(&self) -> Vec<String> {
        self.fields
            .as_ref()
            .map(|f| f.split(',').map(|s| s.trim().to_string()).collect())
            .unwrap_or_default()
    }

    /// 解析过滤条件
    pub fn parse_filters(&self) -> Result<Option<serde_json::Value>, serde_json::Error> {
        match &self.filters {
            Some(filters) => serde_json::from_str(filters).map(Some),
            None => Ok(None),
        }
    }
}