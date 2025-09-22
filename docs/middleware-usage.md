# 中间件使用指南

本文档介绍了 Aionix AI Studio 中间件系统的使用方法，包括认证、授权、租户隔离和访问控制。

## 概述

中间件系统提供了以下核心功能：

1. **认证中间件** - JWT 令牌和 API 密钥验证
2. **租户中间件** - 租户识别和数据隔离
3. **访问控制中间件** - 综合的权限和配额检查
4. **基础中间件** - 请求日志、安全头、响应时间等

## 中间件类型

### 1. 认证中间件

#### JWT 认证中间件
```rust
use crate::api::middleware::JwtAuthMiddleware;

// 基础 JWT 认证
cfg.wrap(JwtAuthMiddleware::new(secret_key));

// 带权限要求的 JWT 认证
cfg.wrap(JwtAuthMiddleware::with_permissions(
    secret_key, 
    vec!["read".to_string(), "write".to_string()]
));
```

#### API 密钥认证中间件
```rust
use crate::api::middleware::ApiKeyAuthMiddleware;

// 基础 API 密钥认证
cfg.wrap(ApiKeyAuthMiddleware::new());

// 带权限要求的 API 密钥认证
cfg.wrap(ApiKeyAuthMiddleware::with_permissions(
    vec!["api_access".to_string()]
));
```

### 2. 租户中间件

#### 租户识别中间件
```rust
use crate::api::middleware::{TenantIdentificationMiddleware, TenantIdentificationStrategy};

// 默认策略（头部 + 子域名 + 查询参数）
cfg.wrap(TenantIdentificationMiddleware::default());

// 仅头部识别
cfg.wrap(TenantIdentificationMiddleware::new(
    TenantIdentificationStrategy::Header
));

// 仅子域名识别
cfg.wrap(TenantIdentificationMiddleware::new(
    TenantIdentificationStrategy::Subdomain
));

// 可选租户识别（不强制要求）
cfg.wrap(TenantIdentificationMiddleware::optional(
    TenantIdentificationStrategy::Combined(vec![
        TenantIdentificationStrategy::Header,
        TenantIdentificationStrategy::Subdomain,
    ])
));
```

#### 租户隔离中间件
```rust
use crate::api::middleware::TenantIsolationMiddleware;

// 租户数据隔离和配额检查
cfg.wrap(TenantIsolationMiddleware);
```

### 3. 访问控制中间件

#### 预定义配置
```rust
use crate::api::middleware::AccessControlMiddleware;

// 标准 API 访问控制（JWT/API密钥 + 租户 + 配额检查）
cfg.wrap(AccessControlMiddleware::api_standard());

// 管理员专用访问控制
cfg.wrap(AccessControlMiddleware::admin_only());

// 公开访问（仅租户识别）
cfg.wrap(AccessControlMiddleware::public());
```

#### 自定义配置
```rust
use crate::api::middleware::{AccessControlMiddleware, AccessControlPolicy, AuthMethod, TenantIdentificationStrategy};

let policy = AccessControlPolicy {
    require_auth: true,
    auth_methods: vec![AuthMethod::Jwt, AuthMethod::ApiKey],
    require_tenant: true,
    tenant_strategy: TenantIdentificationStrategy::Header,
    required_permissions: vec!["read".to_string(), "write".to_string()],
    required_roles: vec!["user".to_string()],
    check_quota: true,
    check_ip_whitelist: true,
    enable_rate_limit: true,
};

cfg.wrap(AccessControlMiddleware::new(policy));
```

## 中间件配置器

为了简化中间件的使用，提供了 `MiddlewareConfig` 配置器：

```rust
use crate::api::middleware::MiddlewareConfig;

// 标准 API 中间件
cfg.wrap(MiddlewareConfig::api_standard());

// 管理员中间件
cfg.wrap(MiddlewareConfig::admin_only());

// 公开中间件
cfg.wrap(MiddlewareConfig::public());

// 带权限要求的中间件
cfg.wrap(MiddlewareConfig::with_permissions(vec!["admin".to_string()]));

// 带角色要求的中间件
cfg.wrap(MiddlewareConfig::with_roles(vec!["admin".to_string()]));
```

## 路由配置示例

### 1. 基础路由配置

```rust
use actix_web::{web, App};
use crate::api::middleware::MiddlewareConfig;

fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/v1")
            // 公开路由
            .service(
                web::scope("/public")
                    .wrap(MiddlewareConfig::public())
                    .route("/health", web::get().to(health_check))
                    .route("/version", web::get().to(get_version))
            )
            // 需要认证的路由
            .service(
                web::scope("/protected")
                    .wrap(MiddlewareConfig::api_standard())
                    .route("/profile", web::get().to(get_profile))
                    .route("/data", web::get().to(get_data))
            )
            // 管理员路由
            .service(
                web::scope("/admin")
                    .wrap(MiddlewareConfig::admin_only())
                    .route("/users", web::get().to(list_users))
                    .route("/tenants", web::get().to(list_tenants))
            )
    );
}
```

### 2. 复杂路由配置

```rust
fn configure_complex_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/v1")
            // 租户管理路由
            .service(
                web::scope("/tenants")
                    // 管理员权限的路由
                    .service(
                        web::scope("")
                            .wrap(MiddlewareConfig::admin_only())
                            .route("", web::post().to(create_tenant))
                            .route("", web::get().to(list_tenants))
                            .route("/{id}", web::delete().to(delete_tenant))
                    )
                    // 标准认证的路由
                    .service(
                        web::scope("")
                            .wrap(MiddlewareConfig::api_standard())
                            .route("/{id}", web::get().to(get_tenant))
                            .route("/{id}/stats", web::get().to(get_tenant_stats))
                    )
            )
            // 知识库路由
            .service(
                web::scope("/knowledge-bases")
                    .wrap(MiddlewareConfig::with_permissions(vec![
                        "knowledge_base_read".to_string(),
                        "knowledge_base_write".to_string()
                    ]))
                    .route("", web::get().to(list_knowledge_bases))
                    .route("", web::post().to(create_knowledge_base))
                    .route("/{id}", web::get().to(get_knowledge_base))
                    .route("/{id}", web::put().to(update_knowledge_base))
                    .route("/{id}", web::delete().to(delete_knowledge_base))
            )
    );
}
```

## 认证方式

### 1. JWT 令牌认证

**请求头格式：**
```
Authorization: Bearer <jwt_token>
```

**JWT 载荷结构：**
```json
{
  "sub": "user_id",
  "tenant_id": "tenant_id",
  "username": "username",
  "role": "user",
  "permissions": ["read", "write"],
  "is_admin": false,
  "iat": 1234567890,
  "exp": 1234567890,
  "iss": "aionix-ai-studio"
}
```

### 2. API 密钥认证

**请求头格式：**
```
X-API-Key: ak_1234567890abcdef1234567890abcdef
```

**API 密钥格式：**
- 前缀：`ak_`
- 长度：32 字符（不包括前缀）
- 字符集：A-Z, a-z, 0-9

### 3. 租户识别

#### 请求头方式
```
X-Tenant-ID: 12345678-1234-1234-1234-123456789012
X-Tenant-Slug: my-company
```

#### 子域名方式
```
https://my-company.api.example.com/api/v1/...
```

#### 查询参数方式
```
https://api.example.com/api/v1/...?tenant_id=12345678-1234-1234-1234-123456789012
https://api.example.com/api/v1/...?tenant_slug=my-company
```

## 权限系统

### 1. 权限类型

- **系统权限**：`admin`, `user`, `viewer`
- **功能权限**：`knowledge_base_read`, `knowledge_base_write`, `ai_query`, `api_access`
- **资源权限**：`tenant:read`, `tenant:write`, `user:read`, `user:write`

### 2. 角色层次

1. **admin** - 系统管理员，拥有所有权限
2. **manager** - 租户管理员，拥有租户内所有权限
3. **user** - 普通用户，拥有基础功能权限
4. **viewer** - 只读用户，仅拥有查看权限

### 3. 权限检查

```rust
// 在处理器中获取用户信息
use crate::api::middleware::auth::AuthenticatedUser;

async fn my_handler(user: web::ReqData<AuthenticatedUser>) -> ActixResult<HttpResponse> {
    // 检查权限
    if !user.permissions.contains(&"knowledge_base_write".to_string()) {
        return Err(AiStudioError::forbidden("缺少知识库写入权限").into());
    }
    
    // 检查角色
    if user.role != "admin" && user.role != "manager" {
        return Err(AiStudioError::forbidden("需要管理员权限").into());
    }
    
    // 处理业务逻辑
    Ok(HttpResponse::Ok().json("success"))
}
```

## 配额和限制

### 1. 租户配额

- **用户数量限制**：`max_users`
- **知识库数量限制**：`max_knowledge_bases`
- **文档数量限制**：`max_documents`
- **存储空间限制**：`max_storage_bytes`
- **API 调用限制**：`monthly_api_calls`
- **AI 查询限制**：`daily_ai_queries`

### 2. API 密钥限制

- **请求频率限制**：每分钟、每小时、每日请求数
- **IP 白名单**：限制访问的 IP 地址
- **权限范围**：限制可访问的资源和操作

### 3. 速率限制

```rust
// API 密钥速率限制配置
{
  "rate_limit": {
    "requests_per_minute": 60,
    "requests_per_hour": 1000,
    "requests_per_day": 10000
  }
}
```

## 错误处理

### 1. 认证错误

- **401 Unauthorized**：缺少或无效的认证凭据
- **403 Forbidden**：权限不足
- **429 Too Many Requests**：请求频率超限

### 2. 租户错误

- **400 Bad Request**：无法识别租户
- **403 Forbidden**：租户已被暂停或停用
- **429 Too Many Requests**：租户配额超限

### 3. 错误响应格式

```json
{
  "success": false,
  "error": {
    "code": "AUTHENTICATION_ERROR",
    "message": "无效的访问令牌",
    "details": null,
    "timestamp": "2023-12-07T10:30:00Z"
  }
}
```

## 最佳实践

### 1. 中间件顺序

推荐的中间件应用顺序：
1. 基础中间件（请求 ID、日志、安全头等）
2. 访问控制中间件（认证、授权、租户隔离）
3. 业务中间件（内容类型验证等）

### 2. 权限设计

- 使用最小权限原则
- 权限名称使用 `resource:action` 格式
- 避免过度细分权限
- 定期审查和清理权限

### 3. 安全考虑

- 使用 HTTPS 传输敏感信息
- 定期轮换 JWT 密钥和 API 密钥
- 实施 IP 白名单和速率限制
- 记录所有认证和授权事件

### 4. 性能优化

- 缓存用户权限信息
- 使用连接池管理数据库连接
- 异步处理非关键操作（如使用统计更新）
- 合理设置中间件超时时间

## 故障排除

### 1. 常见问题

**问题：JWT 令牌验证失败**
- 检查令牌格式和签名
- 确认密钥配置正确
- 检查令牌是否过期

**问题：租户识别失败**
- 检查请求头或子域名格式
- 确认租户存在且状态为活跃
- 检查租户识别策略配置

**问题：权限检查失败**
- 确认用户拥有必要权限
- 检查角色配置
- 验证租户归属关系

### 2. 调试技巧

- 启用详细日志记录
- 使用请求 ID 跟踪请求流程
- 检查中间件执行顺序
- 验证数据库连接和查询

### 3. 监控指标

- 认证成功/失败率
- 权限检查通过/拒绝率
- 配额使用情况
- API 响应时间
- 错误率和错误类型分布