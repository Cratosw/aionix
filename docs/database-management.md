# 数据库管理系统

本文档介绍 Aionix 数据库管理系统的使用方法，包括迁移、种子数据和备份恢复功能。

## 概述

Aionix 数据库管理系统提供以下功能：

- **数据库迁移**: 版本化的数据库架构管理
- **种子数据**: 开发和测试环境的初始数据
- **租户数据隔离**: 多租户架构的数据安全保障
- **备份和恢复**: 数据保护和灾难恢复

## 快速开始

### 1. 初始化数据库

```bash
# 初始化迁移系统
cargo run --bin aionix-db migration init

# 应用所有迁移
cargo run --bin aionix-db migration migrate

# 初始化种子数据
cargo run --bin aionix-db seed init
```

### 2. 检查状态

```bash
# 检查迁移状态
cargo run --bin aionix-db migration status

# 验证数据库架构
cargo run --bin aionix-db migration validate
```

## 数据库迁移

### 迁移系统特性

- **版本控制**: 每个迁移都有唯一的版本号
- **依赖管理**: 支持迁移之间的依赖关系
- **校验和验证**: 确保迁移文件的完整性
- **事务安全**: 迁移在事务中执行，失败时自动回滚

### 迁移命令

```bash
# 初始化迁移系统
cargo run --bin aionix-db migration init

# 检查迁移状态
cargo run --bin aionix-db migration status

# 应用待处理的迁移
cargo run --bin aionix-db migration migrate

# 回滚指定版本的迁移
cargo run --bin aionix-db migration rollback 20240101_000001

# 验证数据库架构
cargo run --bin aionix-db migration validate
```

### 迁移文件结构

迁移文件位于 `src/db/migrations/migrations.rs`，每个迁移包含：

```rust
Migration {
    version: "20240101_000001".to_string(),
    name: "create_tenants_table".to_string(),
    description: "创建租户表".to_string(),
    up_sql: "CREATE TABLE tenants (...)",
    down_sql: "DROP TABLE tenants",
    dependencies: vec![],
}
```

## 种子数据

### 种子数据功能

- **开发环境数据**: 提供开发和测试所需的基础数据
- **示例数据**: 包含完整的示例租户、用户、知识库等
- **可重置**: 支持清理和重新初始化

### 种子数据命令

```bash
# 初始化种子数据
cargo run --bin aionix-db seed init

# 清理种子数据
cargo run --bin aionix-db seed clean

# 重新初始化种子数据
cargo run --bin aionix-db seed reseed
```

### 种子数据内容

种子数据包括：

1. **默认租户**: 用于开发和测试的租户
2. **管理员用户**: 系统管理员账户
3. **示例知识库**: 包含示例文档的知识库
4. **示例 Agent**: 预配置的智能助手
5. **示例工作流**: 文档处理工作流

## 租户数据隔离

### 隔离机制

- **自动过滤**: 所有查询自动添加租户过滤条件
- **权限验证**: 验证用户对租户数据的访问权限
- **配额管理**: 监控和限制租户资源使用

### 使用租户过滤器

```rust
use crate::db::migrations::tenant_filter::{TenantContext, TenantAwareQuery};

// 创建租户上下文
let context = TenantContext::new(tenant_id, tenant_slug, false);

// 创建租户感知的查询
let query = TenantAwareQuery::<users::Entity>::new(context);

// 查询会自动添加租户过滤条件
let users = query.find().all(&db).await?;
```

### 配额检查

```rust
use crate::db::migrations::tenant_filter::TenantQuotaChecker;

// 检查是否超出配额
let can_create = TenantQuotaChecker::check_quota(
    &db,
    tenant_id,
    "max_documents",
    1
).await?;

if !can_create {
    return Err(AiStudioError::quota_exceeded("文档数量超出限制"));
}
```

## 备份和恢复

### 备份类型

- **完整备份**: 包含所有数据和架构
- **增量备份**: 只包含自上次备份以来的变更
- **差异备份**: 包含自上次完整备份以来的变更
- **数据备份**: 只包含数据，不包含架构
- **架构备份**: 只包含架构，不包含数据

### 备份命令

```bash
# 创建完整备份
cargo run --bin aionix-db backup create full

# 创建增量备份
cargo run --bin aionix-db backup create incremental

# 为特定租户创建备份
cargo run --bin aionix-db backup create full <tenant-id>

# 列出所有备份
cargo run --bin aionix-db backup list

# 列出特定租户的备份
cargo run --bin aionix-db backup list <tenant-id>
```

### 恢复命令

```bash
# 恢复备份
cargo run --bin aionix-db backup restore <backup-id>

# 恢复前清理数据库
cargo run --bin aionix-db backup restore <backup-id> --clean

# 只恢复数据
cargo run --bin aionix-db backup restore <backup-id> --data-only

# 只恢复架构
cargo run --bin aionix-db backup restore <backup-id> --schema-only
```

### 备份管理

```bash
# 验证备份完整性
cargo run --bin aionix-db backup verify <backup-id>

# 删除备份
cargo run --bin aionix-db backup delete <backup-id>

# 清理过期备份（保留30天）
cargo run --bin aionix-db backup cleanup 30
```

## 配置

### 数据库配置

在 `config.toml` 中配置数据库连接：

```toml
[database]
url = "postgresql://user:password@localhost/aionix"
max_connections = 10
min_connections = 1
```

### 备份配置

```toml
[backup]
directory = "./backups"
pg_dump_path = "/usr/bin/pg_dump"
pg_restore_path = "/usr/bin/pg_restore"
retention_days = 30
```

## 最佳实践

### 迁移最佳实践

1. **版本命名**: 使用时间戳格式 `YYYYMMDD_HHMMSS`
2. **原子操作**: 每个迁移应该是原子的，要么全部成功，要么全部失败
3. **向后兼容**: 尽量保持向后兼容性
4. **测试**: 在开发环境充分测试迁移

### 备份最佳实践

1. **定期备份**: 设置自动化的定期备份
2. **多地存储**: 将备份存储在多个位置
3. **验证备份**: 定期验证备份的完整性
4. **恢复测试**: 定期测试备份恢复流程

### 租户隔离最佳实践

1. **始终过滤**: 所有查询都应该包含租户过滤条件
2. **权限验证**: 在业务逻辑层验证租户访问权限
3. **配额监控**: 监控租户资源使用情况
4. **审计日志**: 记录跨租户的数据访问

## 故障排除

### 常见问题

1. **迁移失败**

   - 检查数据库连接
   - 查看迁移日志
   - 验证 SQL 语法

2. **备份失败**

   - 检查 pg_dump 路径
   - 验证数据库权限
   - 检查磁盘空间

3. **恢复失败**
   - 验证备份文件完整性
   - 检查目标数据库状态
   - 查看恢复日志

### 日志查看

```bash
# 查看应用日志
tail -f logs/aionix.log

# 查看数据库日志
tail -f /var/log/postgresql/postgresql.log
```

## 监控和告警

### 监控指标

- 迁移执行时间
- 备份成功率
- 数据库连接数
- 租户资源使用量

### 告警设置

- 迁移失败告警
- 备份失败告警
- 配额超限告警
- 数据库连接异常告警

## 安全考虑

### 数据安全

- 备份文件加密
- 传输过程加密
- 访问权限控制
- 审计日志记录

### 租户安全

- 数据隔离验证
- 权限边界检查
- 跨租户访问监控
- 安全事件记录
