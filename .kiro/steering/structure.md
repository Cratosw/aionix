# 项目结构和组织

## 目录结构

```
aionix/
├── src/                    # 主要源代码
│   ├── ai/                # AI 相关模块
│   ├── api/               # API 路由和处理器
│   ├── config/            # 配置管理
│   └── db/                # 数据库相关代码
├── packages/              # 共享包和模块
│   └── common/            # 通用功能模块
├── .kiro/                 # Kiro IDE 配置
├── .vscode/               # VS Code 配置
├── Cargo.toml             # 项目配置和依赖
├── README.md              # 项目说明
└── LICENSE                # 许可证
```

## 代码组织原则

### 模块化设计
- **src/**: 主应用程序代码
- **packages/**: 可重用的共享模块
- 每个功能模块有独立的目录

### 核心模块说明

#### `src/ai/`
- AI 模型集成
- rig 框架相关代码
- 问答处理逻辑

#### `src/api/`
- HTTP 路由定义
- 请求/响应处理器
- API 中间件

#### `src/config/`
- 应用配置管理
- 环境变量处理
- 配置结构定义

#### `src/db/`
- 数据库连接管理
- SeaORM 实体定义
- 数据库迁移
- 查询和操作

#### `packages/common/`
- 跨模块共享的类型定义
- 通用工具函数
- 错误处理

## 命名约定

### 文件和目录
- 使用 snake_case 命名
- 模块目录包含 `mod.rs` 或 `lib.rs`
- 测试文件使用 `_test.rs` 后缀

### 代码风格
- 遵循 Rust 标准命名约定
- 结构体使用 PascalCase
- 函数和变量使用 snake_case
- 常量使用 SCREAMING_SNAKE_CASE

## 依赖管理

### Workspace 结构
- 根目录 `Cargo.toml` 定义主要依赖
- `packages/` 下的子包可以有独立的依赖

### 功能特性
- 使用 Cargo features 管理可选功能
- 数据库相关功能通过 SeaORM features 启用