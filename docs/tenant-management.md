# 租户管理系统

本文档描述了 Aionix AI Studio 的多租户管理系统的设计和使用方法。

## 概述

多租户系统是 Aionix AI Studio 的核心特性之一，它允许多个独立的组织（租户）在同一个平台上安全地使用服务，同时确保数据隔离和资源管理。

## 核心特性

### 🏢 租户管理

- **租户创建**: 支持创建新的租户实例
- **租户配置**: 灵活的租户级配置管理
- **状态管理**: 支持激活、暂停、停用等状态
- **元数据管理**: 联系信息、描述等元数据

### 📊 配额管理

- **资源限制**: 用户数、知识库数、文档数等限制
- **使用统计**: 实时跟踪资源使用情况
- **配额检查**: 自动检查和防止超限使用
- **灵活配置**: 支持自定义配额策略

### 🔒 数据隔离

- **完全隔离**: 租户间数据完全隔离
- **安全访问**: 基于租户的访问控制
- **查询过滤**: 自动添加租户过滤条件
- **权限验证**: 多层次权限验证机制

## 数据模型

### 租户实体

```rust
pub struct Tenant {
    pub id: Uuid,                    // 租户唯一标识
    pub name: String,                // 租户名称（唯一）
    pub slug: String,                // URL 友好标识符（唯一）
    pub display_name: String,        // 显示名称
    pub description: Option<String>, // 描述
    pub status: TenantStatus,        // 状态
    pub config: TenantConfig,        // 配置
    pub quota_limits: QuotaLimits,   // 配额限制
    pub usage_stats: UsageStats,     // 使用统计
    pub contact_email: Option<String>, // 联系邮箱
    pub contact_phone: Option<String>, // 联系电话
    pub created_at: DateTime<Utc>,   // 创建时间
    pub updated_at: DateTime<Utc>,   // 更新时间
    pub last_active_at: Option<DateTime<Utc>>, // 最后活跃时间
}
```

### 租户状态

```rust
pub enum TenantStatus {
    Active,    // 活跃状态，可正常使用
    Suspended, // 暂停状态，临时禁用
    Inactive,  // 非活跃状态，长期停用
}
```

### 租户配置

```rust
pub struct TenantConfig {
    pub timezone: String,           // 时区设置
    pub language: String,           // 语言设置
    pub theme: String,              // 主题设置
    pub features: TenantFeatures,   // 功能开关
    pub custom_settings: Value,     // 自定义设置
}

pub struct TenantFeatures {
    pub ai_enabled: bool,           // AI 功能
    pub knowledge_base_enabled: bool, // 知识库功能
    pub agent_enabled: bool,        // Agent 功能
    pub api_enabled: bool,          // API 访问
    pub file_upload_enabled: bool,  // 文件上传
}
```

### 配额限制

```rust
pub struct TenantQuotaLimits {
    pub max_users: u32,             // 最大用户数
    pub max_knowledge_bases: u32,   // 最大知识库数
    pub max_documents: u32,         // 最大文档数
    pub max_storage_bytes: u64,     // 最大存储空间
    pub monthly_api_calls: u32,     // 月度 API 调用限制
    pub daily_ai_queries: u32,      // 日度 AI 查询限制
}
```

## API 接口

### 租户管理接口

#### 创建租户

```http
POST /api/v1/tenants
Content-Type: application/json
Authorization: Bearer <admin-token>

{
  "name": "example-corp",
  "slug": "example-corp",
  "display_name": "Example Corporation",
  "description": "示例企业租户",
  "contact_email": "admin@example.com",
  "config": {
    "timezone": "Asia/Shanghai",
    "language": "zh-CN",
    "features": {
      "ai_enabled": true,
      "knowledge_base_enabled": true
    }
  },
  "quota_limits": {
    "max_users": 50,
    "max_knowledge_bases": 5,
    "max_documents": 500
  }
}
```

#### 获取租户详情

```http
GET /api/v1/tenants/{tenant_id}
Authorization: Bearer <admin-token>
```

#### 更新租户

```http
PUT /api/v1/tenants/{tenant_id}
Content-Type: application/json
Authorization: Bearer <admin-token>

{
  "display_name": "Updated Corporation",
  "status": "active",
  "quota_limits": {
    "max_users": 100
  }
}
```

#### 列出租户

```http
GET /api/v1/tenants?page=1&page_size=20&status=active&name_search=example
Authorization: Bearer <admin-token>
```

#### 删除租户

```http
DELETE /api/v1/tenants/{tenant_id}
Authorization: Bearer <admin-token>
```

### 租户操作接口

#### 暂停租户

```http
POST /api/v1/tenants/{tenant_id}/suspend
Content-Type: application/json
Authorization: Bearer <admin-token>

{
  "reason": "违反服务条款"
}
```

#### 激活租户

```http
POST /api/v1/tenants/{tenant_id}/activate
Authorization: Bearer <admin-token>
```

#### 检查配额

```http
GET /api/v1/tenants/{tenant_id}/quota/users?requested_amount=5
Authorization: Bearer <admin-token>
```

### 统计接口

#### 获取租户统计

```http
GET /api/v1/tenants/stats
Authorization: Bearer <admin-token>
```

响应示例：

```json
{
  "success": true,
  "data": {
    "total_tenants": 150,
    "active_tenants": 120,
    "suspended_tenants": 20,
    "inactive_tenants": 10,
    "tenants_created_today": 5,
    "tenants_created_this_month": 25
  }
}
```

## 服务层架构

### TenantService

租户服务层提供了完整的租户管理业务逻辑：

```rust
impl TenantService {
    // 基础 CRUD 操作
    pub async fn create_tenant(&self, request: CreateTenantRequest) -> Result<TenantResponse>;
    pub async fn get_tenant_by_id(&self, tenant_id: Uuid) -> Result<TenantResponse>;
    pub async fn get_tenant_by_slug(&self, slug: &str) -> Result<TenantResponse>;
    pub async fn update_tenant(&self, tenant_id: Uuid, request: UpdateTenantRequest) -> Result<TenantResponse>;
    pub async fn delete_tenant(&self, tenant_id: Uuid) -> Result<()>;

    // 列表和搜索
    pub async fn list_tenants(&self, filter: Option<TenantFilter>, pagination: PaginationQuery) -> Result<PaginatedResponse<TenantResponse>>;

    // 状态管理
    pub async fn suspend_tenant(&self, tenant_id: Uuid, reason: Option<String>) -> Result<TenantResponse>;
    pub async fn activate_tenant(&self, tenant_id: Uuid) -> Result<TenantResponse>;

    // 配额管理
    pub async fn check_tenant_quota(&self, tenant_id: Uuid, resource_type: &str, requested_amount: i64) -> Result<bool>;
    pub async fn update_tenant_usage(&self, tenant_id: Uuid) -> Result<()>;

    // 统计信息
    pub async fn get_tenant_stats(&self) -> Result<TenantStatsResponse>;
}
```

### 数据验证

服务层包含完整的数据验证逻辑：

1. **唯一性验证**: 租户名称和标识符的唯一性
2. **格式验证**: 标识符格式验证（小写字母、数字、连字符）
3. **保留字检查**: 防止使用系统保留的标识符
4. **关联数据检查**: 删除前检查是否有关联数据

## 配额管理

### 配额类型

系统支持以下配额类型：

- `users`: 用户数量限制
- `knowledge_bases`: 知识库数量限制
- `documents`: 文档数量限制
- `storage`: 存储空间限制（字节）
- `monthly_api_calls`: 月度 API 调用限制
- `daily_ai_queries`: 日度 AI 查询限制

### 配额检查流程

```rust
// 检查配额
let can_create = service.check_tenant_quota(
    tenant_id,
    "users",
    1  // 请求创建 1 个用户
).await?;

if !can_create {
    return Err(AiStudioError::quota_exceeded("用户数量"));
}

// 执行操作
create_user(tenant_id, user_data).await?;

// 更新使用统计
service.update_tenant_usage(tenant_id).await?;
```

### 使用统计更新

系统会自动跟踪和更新租户的资源使用情况：

```rust
pub struct TenantUsageStats {
    pub current_users: u32,
    pub current_knowledge_bases: u32,
    pub current_documents: u32,
    pub current_storage_bytes: u64,
    pub monthly_api_calls: u32,
    pub daily_ai_queries: u32,
    pub last_updated: DateTime<Utc>,
}
```

## 数据隔离

### 自动过滤

所有数据库查询都会自动添加租户过滤条件：

```rust
// 使用租户感知查询
let query = TenantAwareQuery::<users::Entity>::new(tenant_context);
let users = query.find().all(&db).await?;

// 自动添加 WHERE tenant_id = ?
```

### 权限验证

多层次的权限验证机制：

1. **租户级验证**: 验证用户是否属于指定租户
2. **资源级验证**: 验证资源是否属于用户的租户
3. **操作级验证**: 验证用户是否有权限执行特定操作

## 最佳实践

### 租户标识符设计

1. **格式规范**: 使用小写字母、数字和连字符
2. **长度限制**: 建议 3-50 个字符
3. **语义化**: 使用有意义的标识符，如公司名称
4. **避免冲突**: 检查是否与现有标识符冲突

### 配额设置

1. **合理规划**: 根据业务需求设置合理的配额
2. **分层设置**: 不同级别的租户使用不同配额
3. **监控告警**: 设置配额使用告警机制
4. **弹性调整**: 支持动态调整配额限制

### 数据管理

1. **定期清理**: 清理非活跃租户的数据
2. **备份策略**: 按租户进行数据备份
3. **迁移支持**: 支持租户数据迁移
4. **审计日志**: 记录租户操作审计日志

## 监控和运维

### 关键指标

- 租户总数和增长趋势
- 活跃租户比例
- 配额使用率分布
- 资源使用统计
- API 调用频率

### 告警设置

- 租户配额超限告警
- 租户状态异常告警
- 资源使用异常告警
- 数据隔离违规告警

### 运维操作

```bash
# 查看租户统计
curl -H "Authorization: Bearer $ADMIN_TOKEN" \
     https://api.aionix.ai/v1/tenants/stats

# 暂停违规租户
curl -X POST \
     -H "Authorization: Bearer $ADMIN_TOKEN" \
     -H "Content-Type: application/json" \
     -d '{"reason": "违反服务条款"}' \
     https://api.aionix.ai/v1/tenants/{tenant_id}/suspend

# 检查租户配额
curl -H "Authorization: Bearer $ADMIN_TOKEN" \
     https://api.aionix.ai/v1/tenants/{tenant_id}/quota/users?requested_amount=10
```

## 安全考虑

### 数据隔离

1. **数据库级隔离**: 所有查询都包含租户过滤条件
2. **应用级隔离**: 业务逻辑层验证租户权限
3. **API 级隔离**: 接口层验证请求来源

### 访问控制

1. **管理员权限**: 只有管理员可以管理租户
2. **租户权限**: 租户只能访问自己的数据
3. **操作审计**: 记录所有敏感操作

### 安全防护

1. **输入验证**: 严格验证所有输入参数
2. **SQL 注入防护**: 使用参数化查询
3. **权限提升防护**: 防止权限提升攻击
4. **数据泄露防护**: 防止跨租户数据泄露

这个多租户管理系统为 Aionix AI Studio 提供了完整的企业级多租户支持，确保了数据安全、资源管理和系统可扩展性。
