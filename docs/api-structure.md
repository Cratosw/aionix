# API 架构文档

本文档描述了 Aionix AI Studio 的 API 架构设计和实现。

## 概述

Aionix AI Studio API 采用 RESTful 设计原则，提供完整的企业级 AI 服务接口。API 支持多租户架构、版本控制、自动文档生成和全面的错误处理。

## 架构特性

### 🏗️ 模块化设计

```
src/api/
├── mod.rs              # 模块入口和导出
├── routes.rs           # 路由配置和 OpenAPI 文档
├── models.rs           # 请求/响应模型定义
├── responses.rs        # 统一响应格式
├── extractors.rs       # 请求提取器
├── middleware.rs       # API 中间件
└── handlers/           # 处理器实现
    ├── mod.rs
    ├── health.rs       # 健康检查
    └── version.rs      # 版本信息
```

### 🔄 版本控制

- **URL 版本控制**: `/api/v1/`, `/api/v2/`
- **向后兼容**: 保持旧版本 API 的兼容性
- **版本信息**: 通过 `/api/v1/version` 获取详细版本信息

### 📝 自动文档生成

- **OpenAPI 3.0**: 使用 `utoipa` 生成 OpenAPI 规范
- **Swagger UI**: 提供交互式 API 文档界面
- **类型安全**: 编译时验证 API 文档的正确性

### 🛡️ 安全和中间件

- **请求 ID**: 自动生成和传播请求 ID
- **安全头**: 自动添加安全相关的 HTTP 头
- **CORS**: 跨域资源共享配置
- **日志记录**: 结构化请求/响应日志
- **错误处理**: 统一的错误响应格式

## API 端点

### 基础端点

| 端点                   | 方法 | 描述            |
| ---------------------- | ---- | --------------- |
| `/`                    | GET  | 服务根信息      |
| `/health`              | GET  | 简单健康检查    |
| `/api/v1`              | GET  | API 根信息      |
| `/api/v1/docs`         | GET  | Swagger UI 文档 |
| `/api/v1/openapi.json` | GET  | OpenAPI 规范    |

### 健康检查端点

| 端点                      | 方法 | 描述         |
| ------------------------- | ---- | ------------ |
| `/api/v1/health`          | GET  | 简单健康检查 |
| `/api/v1/health/detailed` | GET  | 详细健康检查 |
| `/api/v1/ready`           | GET  | 就绪检查     |
| `/api/v1/live`            | GET  | 存活检查     |

### 版本信息端点

| 端点                         | 方法 | 描述         |
| ---------------------------- | ---- | ------------ |
| `/api/v1/version`            | GET  | API 版本信息 |
| `/api/v1/version/build-info` | GET  | 构建信息     |
| `/api/v1/version/spec`       | GET  | API 规范信息 |

## 请求/响应格式

### 统一响应格式

所有 API 响应都遵循统一的格式：

```json
{
  "success": true,
  "data": { ... },
  "error": null,
  "request_id": "uuid",
  "timestamp": "2024-01-01T00:00:00Z",
  "version": "1.0.0"
}
```

### 错误响应格式

```json
{
  "success": false,
  "data": null,
  "error": {
    "code": "ERROR_CODE",
    "message": "错误描述",
    "details": { ... },
    "field": "field_name",
    "help_url": "https://docs.aionix.ai/api/errors"
  },
  "request_id": "uuid",
  "timestamp": "2024-01-01T00:00:00Z",
  "version": "1.0.0"
}
```

### 分页响应格式

```json
{
  "success": true,
  "data": {
    "data": [ ... ],
    "pagination": {
      "page": 1,
      "page_size": 20,
      "total": 100,
      "total_pages": 5,
      "has_next": true,
      "has_prev": false
    }
  },
  "request_id": "uuid",
  "timestamp": "2024-01-01T00:00:00Z",
  "version": "1.0.0"
}
```

## 请求提取器

### 认证提取器

```rust
// 必需认证
async fn handler(auth: AuthExtractor) -> ActixResult<HttpResponse> {
    // auth.user_id, auth.tenant_id, auth.role, auth.permissions
}

// 可选认证
async fn handler(auth: OptionalAuthExtractor) -> ActixResult<HttpResponse> {
    if let Some(user) = auth.user {
        // 已认证用户
    } else {
        // 匿名用户
    }
}

// 管理员权限
async fn handler(admin: AdminExtractor) -> ActixResult<HttpResponse> {
    // 只有管理员可以访问
}
```

### 租户提取器

```rust
async fn handler(tenant: TenantExtractor) -> ActixResult<HttpResponse> {
    // tenant.context.tenant_id, tenant.context.tenant_slug
}
```

### 分页提取器

```rust
async fn handler(pagination: PaginationExtractor) -> ActixResult<HttpResponse> {
    // pagination.page, pagination.page_size, pagination.sort_by
}
```

### API 密钥提取器

```rust
async fn handler(api_key: ApiKeyExtractor) -> ActixResult<HttpResponse> {
    // api_key.key, api_key.tenant_id, api_key.permissions
}
```

## 中间件

### 请求 ID 中间件

自动为每个请求生成唯一的请求 ID，用于日志追踪和调试。

```http
X-Request-ID: 550e8400-e29b-41d4-a716-446655440000
```

### API 版本中间件

在响应头中添加 API 版本信息。

```http
X-API-Version: 1.0.0
```

### 安全头中间件

自动添加安全相关的 HTTP 头：

```http
X-Content-Type-Options: nosniff
X-Frame-Options: DENY
X-XSS-Protection: 1; mode=block
Strict-Transport-Security: max-age=31536000; includeSubDomains
Referrer-Policy: strict-origin-when-cross-origin
```

### 响应时间中间件

在响应头中添加请求处理时间。

```http
X-Response-Time: 123ms
```

### 请求日志中间件

记录结构化的请求/响应日志：

```json
{
  "level": "INFO",
  "timestamp": "2024-01-01T00:00:00Z",
  "request_id": "uuid",
  "method": "GET",
  "path": "/api/v1/health",
  "status": 200,
  "duration_ms": 123,
  "remote_addr": "127.0.0.1",
  "user_agent": "curl/7.68.0"
}
```

## 错误处理

### 错误类型

| HTTP 状态码 | 错误代码               | 描述           |
| ----------- | ---------------------- | -------------- |
| 400         | BAD_REQUEST            | 请求格式错误   |
| 400         | VALIDATION_ERROR       | 数据验证失败   |
| 401         | UNAUTHORIZED           | 未授权访问     |
| 403         | FORBIDDEN              | 禁止访问       |
| 404         | NOT_FOUND              | 资源不存在     |
| 409         | CONFLICT               | 资源冲突       |
| 422         | UNPROCESSABLE_ENTITY   | 无法处理的实体 |
| 429         | RATE_LIMITED           | 请求频率过高   |
| 429         | QUOTA_EXCEEDED         | 配额超限       |
| 500         | INTERNAL_ERROR         | 内部服务器错误 |
| 502         | EXTERNAL_SERVICE_ERROR | 外部服务错误   |
| 504         | TIMEOUT                | 请求超时       |

### 错误响应示例

```json
{
  "success": false,
  "data": null,
  "error": {
    "code": "VALIDATION_ERROR",
    "message": "用户名不能为空",
    "field": "username",
    "help_url": "https://docs.aionix.ai/api/validation"
  },
  "request_id": "550e8400-e29b-41d4-a716-446655440000",
  "timestamp": "2024-01-01T00:00:00Z",
  "version": "1.0.0"
}
```

## 开发和调试

### 开发环境端点

在开发环境中，提供额外的调试端点：

| 端点         | 方法 | 描述         |
| ------------ | ---- | ------------ |
| `/dev/test`  | GET  | 开发测试端点 |
| `/dev/debug` | GET  | 调试信息端点 |

### 日志配置

使用结构化日志记录所有 API 请求和响应：

```toml
[logging]
level = "info"
format = "json"
output = "stdout"
```

### 健康检查

提供多层次的健康检查：

1. **简单检查**: 服务是否运行
2. **详细检查**: 包含依赖服务状态
3. **就绪检查**: 服务是否准备好接收请求
4. **存活检查**: 服务是否存活

## 性能和监控

### 响应时间

- 所有请求都会记录响应时间
- 响应头中包含 `X-Response-Time`
- 日志中记录详细的性能指标

### 监控指标

- 请求数量和频率
- 响应时间分布
- 错误率统计
- 依赖服务健康状态

### 缓存策略

- API 响应缓存
- 静态资源缓存
- 数据库查询缓存

## 安全考虑

### 认证和授权

- JWT 令牌认证
- API 密钥认证
- 基于角色的访问控制 (RBAC)
- 租户级数据隔离

### 输入验证

- 请求参数验证
- 数据类型检查
- 长度和格式限制
- SQL 注入防护

### 安全头

- 内容类型嗅探防护
- 点击劫持防护
- XSS 防护
- HTTPS 强制

### 限流和配额

- API 调用频率限制
- 租户级配额管理
- 资源使用监控

## 部署和运维

### 容器化部署

```dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/aionix /usr/local/bin/
EXPOSE 8080
CMD ["aionix"]
```

### 环境配置

```toml
[server]
host = "0.0.0.0"
port = 8080
workers = 4

[api]
version = "v1"
cors_origins = ["*"]
rate_limit = 1000
```

### 监控和告警

- 健康检查端点监控
- 响应时间告警
- 错误率告警
- 依赖服务状态监控

## 最佳实践

### API 设计

1. **RESTful 原则**: 使用标准的 HTTP 方法和状态码
2. **资源导向**: URL 表示资源，HTTP 方法表示操作
3. **版本控制**: 使用 URL 路径进行版本控制
4. **一致性**: 保持 API 接口的一致性

### 错误处理

1. **统一格式**: 使用统一的错误响应格式
2. **详细信息**: 提供足够的错误信息帮助调试
3. **用户友好**: 错误消息对用户友好
4. **文档化**: 在 API 文档中说明所有可能的错误

### 性能优化

1. **分页**: 对大量数据使用分页
2. **缓存**: 合理使用缓存减少数据库查询
3. **压缩**: 启用 HTTP 压缩
4. **异步**: 使用异步处理提高并发性能

### 安全

1. **认证**: 所有敏感操作都需要认证
2. **授权**: 实施细粒度的权限控制
3. **验证**: 验证所有输入数据
4. **日志**: 记录安全相关的操作日志

这个 API 架构为 Aionix AI Studio 提供了坚实的基础，支持企业级的多租户 AI 服务需求。
