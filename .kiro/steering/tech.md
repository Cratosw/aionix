# 技术栈和构建系统

## 核心技术栈

### Web 框架
- **Actix Web 4**: 高性能、功能丰富的 Rust Web 框架
- **异步运行时**: 基于 tokio 的异步处理

### 数据库和 ORM
- **PostgreSQL**: 主数据库，存储问答历史记录
- **SeaORM 0.12**: 异步、动态的 Rust ORM，与 Actix Web 完美集成
- **SQLx**: PostgreSQL 驱动，支持编译时 SQL 检查

### AI 框架
- **rig**: 用于构建和部署 AI/ML 模型的 Rust 框架（注意：相对较新的框架）

### 序列化和配置
- **serde**: JSON 序列化/反序列化
- **config**: 配置管理
- **dotenvy**: 环境变量加载

### API 文档
- **utoipa**: OpenAPI 规范生成
- **utoipa-swagger-ui**: Swagger UI 集成

## 构建系统

### Cargo 配置
- **Rust Edition**: 2021
- **包管理**: 使用 Cargo.toml 管理依赖

### 常用命令

```bash
# 构建项目
cargo build

# 运行开发服务器
cargo run

# 运行测试
cargo test

# 检查代码
cargo check

# 格式化代码
cargo fmt

# 代码检查
cargo clippy

# 构建发布版本
cargo build --release
```

### 开发环境设置

1. 安装 Rust 工具链
2. 配置 PostgreSQL 数据库
3. 设置环境变量（通过 .env 文件）
4. 运行数据库迁移（如果有的话）

## 依赖版本策略

- 使用稳定版本的主要依赖
- SeaORM 使用 `^0.12.0` 允许小版本更新
- 其他依赖使用精确版本控制