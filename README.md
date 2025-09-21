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

- **Web 框架**: Actix Web 4.x
- **ORM**: SeaORM 0.12.x
- **数据库**: PostgreSQL + pgvector
- **AI 框架**: Rig (待集成)
- **缓存**: Redis (待集成)

## 快速开始

### 构建项目

```bash
cargo build
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

### API 端点

- `GET /` - 欢迎页面
- `GET /health` - 健康检查
- `GET /api/v1/health` - API 健康检查

## 开发状态

✅ **已完成**:
- 基础项目结构
- Actix Web 服务器配置
- 健康检查端点
- 基础模块结构
- 单元测试

🚧 **进行中**:
- 配置管理系统
- 错误处理系统
- 数据库集成

📋 **待实现**:
- 多租户系统
- RAG 功能
- Agent 系统
- 知识库管理
- API 文档

## 贡献

请参考项目的 `.kiro/specs/enterprise-ai-studio/` 目录中的需求和设计文档。

## 许可证

[LICENSE](LICENSE)