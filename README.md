# Aionix AI Studio

基于 Rust 的企业级 AI 问答系统，支持多租户、RAG（检索增强生成）、Agent 编排等功能。

## 项目结构

```
aionix/
├── src/                    # 主要源代码
│   ├── ai/                # AI 相关模块 (待实现)
│   ├── api/               # API 路由和处理器
│   ├── config/            # 配置管理
│   ├── db/                # 数据库相关代码
│   ├── health.rs          # 健康检查端点
│   ├── lib.rs             # 库入口
│   └── main.rs            # 应用程序入口点
├── packages/              # 共享包和模块
│   └── common/            # 通用功能模块
├── Cargo.toml             # 项目配置和依赖
└── README.md              # 项目说明
```

## 技术栈

- **Web 框架**: Actix Web 4.x + CORS + HTTP Auth
- **ORM**: SeaORM 0.12.x + SQLx
- **数据库**: PostgreSQL + pgvector (向量数据库)
- **AI 框架**: Rig Core
- **缓存**: Redis
- **认证**: JWT + bcrypt
- **日志**: tracing + tracing-subscriber
- **API 文档**: utoipa + Swagger UI

## 快速开始

### 环境准备

1. 复制环境变量配置文件：
```bash
cp .env.example .env
```

2. 复制配置文件模板（可选）：
```bash
cp config.example.toml config.toml
```

3. 根据需要修改配置文件中的设置

### 配置系统

项目支持多层配置系统，按优先级从高到低：

1. **环境变量** - 最高优先级，支持 `AIONIX_` 前缀
2. **配置文件** - `config.toml` 文件
3. **默认值** - 内置默认配置

配置验证会在启动时自动进行，确保所有配置项的有效性。

### 构建项目

```bash
# 检查代码
cargo check

# 构建项目
cargo build

# 构建发布版本
cargo build --release
```

### 运行测试

```bash
cargo test
```

### 启动服务器

```bash
cargo run
```

服务器将在 `http://127.0.0.1:8080` 启动。

### 功能特性

项目支持多种功能特性，详见 [功能特性文档](docs/features.md)。

### API 端点

- `GET /` - 欢迎页面
- `GET /health` - 健康检查
- `GET /api/v1/health` - API 健康检查

## 开发状态

✅ **已完成**:
- 基础项目结构和配置管理
- Actix Web 服务器和 API 路由
- 数据库基础设施（PostgreSQL + SeaORM）
- 多租户认证和授权系统
- 配额管理和限流系统
- AI 基础设施和 Rig 框架集成
- 文档处理和向量化
- 向量检索和相似度搜索

🚧 **进行中**:
- 知识库管理 API
- RAG 智能问答系统

📋 **待实现**:
- Agent 系统核心
- 工作流编排系统
- 插件系统和扩展性
- API 文档和接口规范
- 监控、日志和运维
- 安全和数据保护

## 贡献

请参考项目的 `.kiro/specs/enterprise-ai-studio/` 目录中的需求和设计文档。

## 许可证

[LICENSE](LICENSE)