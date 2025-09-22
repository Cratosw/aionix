// API 请求和响应模型
// 定义所有 API 端点的请求和响应结构体

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// API 版本信息
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ApiVersion {
    /// API 版本号
    pub version: String,
    /// 构建时间
    pub build_time: String,
    /// Git 提交哈希
    pub git_hash: Option<String>,
    /// 支持的功能列表
    pub features: Vec<String>,
}

/// 分页请求参数
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct PaginationQuery {
    /// 页码，从 1 开始
    #[serde(default = "default_page")]
    pub page: u32,
    /// 每页大小，默认 20，最大 100
    #[serde(default = "default_page_size")]
    pub page_size: u32,
    /// 排序字段
    pub sort_by: Option<String>,
    /// 排序方向：asc 或 desc
    #[serde(default = "default_sort_order")]
    pub sort_order: SortOrder,
}

/// 排序方向
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum SortOrder {
    Asc,
    Desc,
}

/// 分页响应
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PaginatedResponse<T> {
    /// 数据列表
    pub data: Vec<T>,
    /// 分页信息
    pub pagination: PaginationInfo,
}

/// 分页信息
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PaginationInfo {
    /// 当前页码
    pub page: u32,
    /// 每页大小
    pub page_size: u32,
    /// 总记录数
    pub total: u64,
    /// 总页数
    pub total_pages: u32,
    /// 是否有下一页
    pub has_next: bool,
    /// 是否有上一页
    pub has_prev: bool,
}

/// 搜索查询参数
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct SearchQuery {
    /// 搜索关键词
    pub q: Option<String>,
    /// 搜索字段
    pub fields: Option<Vec<String>>,
    /// 过滤条件
    pub filters: Option<serde_json::Value>,
    /// 分页参数
    #[serde(flatten)]
    pub pagination: PaginationQuery,
}

/// 批量操作请求
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct BatchRequest<T> {
    /// 操作类型
    pub operation: BatchOperation,
    /// 数据列表
    pub items: Vec<T>,
    /// 操作选项
    pub options: Option<serde_json::Value>,
}

/// 批量操作类型
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum BatchOperation {
    Create,
    Update,
    Delete,
    Import,
    Export,
}

/// 批量操作响应
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BatchResponse<T> {
    /// 操作类型
    pub operation: BatchOperation,
    /// 成功处理的数量
    pub success_count: u32,
    /// 失败处理的数量
    pub error_count: u32,
    /// 成功的结果
    pub success_items: Vec<T>,
    /// 失败的错误信息
    pub errors: Vec<BatchError>,
}

/// 批量操作错误
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BatchError {
    /// 错误索引
    pub index: u32,
    /// 错误代码
    pub code: String,
    /// 错误消息
    pub message: String,
    /// 错误详情
    pub details: Option<serde_json::Value>,
}

/// 健康检查响应
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HealthResponse {
    /// 服务状态
    pub status: HealthStatus,
    /// 检查时间
    pub timestamp: DateTime<Utc>,
    /// 版本信息
    pub version: String,
    /// 依赖服务状态
    pub dependencies: Vec<DependencyHealth>,
    /// 系统信息
    pub system: SystemInfo,
}

/// 健康状态
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

/// 依赖服务健康状态
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DependencyHealth {
    /// 服务名称
    pub name: String,
    /// 服务状态
    pub status: HealthStatus,
    /// 响应时间（毫秒）
    pub response_time_ms: Option<u64>,
    /// 错误信息
    pub error: Option<String>,
}

/// 系统信息
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SystemInfo {
    /// 启动时间
    pub uptime_seconds: u64,
    /// 内存使用量（字节）
    pub memory_usage_bytes: u64,
    /// CPU 使用率（百分比）
    pub cpu_usage_percent: f64,
    /// 活跃连接数
    pub active_connections: u32,
}

/// 租户信息
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TenantInfo {
    /// 租户 ID
    pub id: Uuid,
    /// 租户名称
    pub name: String,
    /// 租户标识符
    pub slug: String,
    /// 显示名称
    pub display_name: String,
    /// 描述
    pub description: Option<String>,
    /// 状态
    pub status: String,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

/// 用户信息
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserInfo {
    /// 用户 ID
    pub id: Uuid,
    /// 租户 ID
    pub tenant_id: Uuid,
    /// 用户名
    pub username: String,
    /// 邮箱
    pub email: String,
    /// 显示名称
    pub display_name: String,
    /// 头像 URL
    pub avatar_url: Option<String>,
    /// 角色
    pub role: String,
    /// 状态
    pub status: String,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 最后登录时间
    pub last_login_at: Option<DateTime<Utc>>,
}

/// 认证请求
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct AuthRequest {
    /// 用户名或邮箱
    pub username: String,
    /// 密码
    pub password: String,
    /// 租户标识符
    pub tenant_slug: Option<String>,
}

/// 认证响应
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AuthResponse {
    /// 访问令牌
    pub access_token: String,
    /// 刷新令牌
    pub refresh_token: String,
    /// 令牌类型
    pub token_type: String,
    /// 过期时间（秒）
    pub expires_in: u64,
    /// 用户信息
    pub user: UserInfo,
}

/// 刷新令牌请求
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct RefreshTokenRequest {
    /// 刷新令牌
    pub refresh_token: String,
}

// 默认值函数
fn default_page() -> u32 {
    1
}

fn default_page_size() -> u32 {
    20
}

fn default_sort_order() -> SortOrder {
    SortOrder::Desc
}

impl PaginationQuery {
    /// 验证分页参数
    pub fn validate(&mut self) {
        if self.page == 0 {
            self.page = 1;
        }
        if self.page_size == 0 {
            self.page_size = 20;
        }
        if self.page_size > 100 {
            self.page_size = 100;
        }
    }

    /// 计算偏移量
    pub fn offset(&self) -> u64 {
        ((self.page - 1) * self.page_size) as u64
    }

    /// 计算限制数量
    pub fn limit(&self) -> u64 {
        self.page_size as u64
    }
}

impl PaginationInfo {
    /// 创建分页信息
    pub fn new(page: u32, page_size: u32, total: u64) -> Self {
        let total_pages = ((total as f64) / (page_size as f64)).ceil() as u32;
        
        Self {
            page,
            page_size,
            total,
            total_pages,
            has_next: page < total_pages,
            has_prev: page > 1,
        }
    }
}

impl<T> PaginatedResponse<T> {
    /// 创建分页响应
    pub fn new(data: Vec<T>, pagination: PaginationInfo) -> Self {
        Self { data, pagination }
    }
}

impl<T> BatchResponse<T> {
    /// 创建批量响应
    pub fn new(operation: BatchOperation) -> Self {
        Self {
            operation,
            success_count: 0,
            error_count: 0,
            success_items: Vec::new(),
            errors: Vec::new(),
        }
    }

    /// 添加成功项
    pub fn add_success(&mut self, item: T) {
        self.success_items.push(item);
        self.success_count += 1;
    }

    /// 添加错误
    pub fn add_error(&mut self, index: u32, code: String, message: String, details: Option<serde_json::Value>) {
        self.errors.push(BatchError {
            index,
            code,
            message,
            details,
        });
        self.error_count += 1;
    }
}