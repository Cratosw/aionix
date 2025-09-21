# Aionix 功能特性配置

## 可用特性

### 默认特性
```toml
default = ["postgres", "redis", "ai"]
```

### 数据库特性
- `postgres` - PostgreSQL 数据库支持（默认）
- `sqlite` - SQLite 数据库支持（开发环境）

### 缓存特性
- `redis` - Redis 缓存支持（默认）

### AI 特性
- `ai` - AI 功能支持，包括 Rig 框架集成（默认）

### 向量数据库特性
- `vector` - pgvector 向量数据库支持

## 使用示例

### 仅使用 SQLite（开发环境）
```toml
[dependencies]
aionix = { version = "0.1.0", default-features = false, features = ["sqlite"] }
```

### 完整功能（生产环境）
```toml
[dependencies]
aionix = { version = "0.1.0", features = ["postgres", "redis", "ai", "vector"] }
```

### 最小配置
```toml
[dependencies]
aionix = { version = "0.1.0", default-features = false, features = ["postgres"] }
```

## 环境变量配置

参考 `.env.example` 文件了解所有可配置的环境变量。

## 依赖说明

### 核心依赖
- `actix-web` - Web 框架
- `sea-orm` - ORM 框架
- `tokio` - 异步运行时
- `serde` - 序列化框架

### 可选依赖
- `redis` - Redis 客户端（需要 `redis` 特性）
- `rig-core` - AI 框架（需要 `ai` 特性）
- `pgvector` - 向量数据库（需要 `vector` 特性）