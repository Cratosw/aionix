# Aionix 配置系统

## 概述

Aionix AI Studio 使用分层配置系统，支持多种配置源，按优先级从高到低排列：

1. **环境变量** (最高优先级)
2. **配置文件** (`config.toml`)
3. **默认值** (内置)

## 配置文件

### 基本配置文件

创建 `config.toml` 文件来覆盖默认配置：

```toml
[server]
host = "0.0.0.0"
port = 8080
workers = 4

[database]
url = "postgresql://user:pass@localhost:5432/aionix"
max_connections = 20

[ai]
model_endpoint = "http://localhost:11434"
api_key = "your_api_key"
```

### 完整配置示例

参考 `config.example.toml` 文件了解所有可配置选项。

## 环境变量

所有配置项都可以通过环境变量覆盖，使用 `AIONIX_` 前缀：

```bash
# 服务器配置
export AIONIX_SERVER__HOST=0.0.0.0
export AIONIX_SERVER__PORT=8080

# 数据库配置
export AIONIX_DATABASE__URL=postgresql://user:pass@localhost:5432/aionix
export AIONIX_DATABASE__MAX_CONNECTIONS=20

# AI 配置
export AIONIX_AI__MODEL_ENDPOINT=http://localhost:11434
export AIONIX_AI__API_KEY=your_api_key
```

注意：使用双下划线 `__` 分隔嵌套配置项。

## 配置验证

系统启动时会自动验证所有配置项：

- **服务器配置**：端口范围、主机地址格式
- **数据库配置**：URL 格式、连接池参数
- **AI 配置**：端点 URL、温度参数范围
- **安全配置**：JWT 密钥长度、bcrypt 成本
- **存储配置**：路径存在性、文件大小限制
- **向量配置**：维度范围、相似度阈值

## 配置结构

### 服务器配置 (`server`)

| 参数 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `host` | String | "127.0.0.1" | 服务器绑定地址 |
| `port` | u16 | 8080 | 服务器端口 |
| `workers` | Option<usize> | None | 工作线程数 |
| `keep_alive` | u64 | 75 | 连接保持时间(秒) |
| `client_timeout` | u64 | 5000 | 客户端超时(毫秒) |
| `client_shutdown` | u64 | 5000 | 客户端关闭超时(毫秒) |

### 数据库配置 (`database`)

| 参数 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `url` | String | "postgresql://localhost/aionix" | 数据库连接 URL |
| `max_connections` | u32 | 10 | 最大连接数 |
| `min_connections` | u32 | 1 | 最小连接数 |
| `connect_timeout` | u64 | 30 | 连接超时(秒) |
| `idle_timeout` | u64 | 600 | 空闲超时(秒) |
| `max_lifetime` | u64 | 1800 | 连接最大生命周期(秒) |

### AI 配置 (`ai`)

| 参数 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `model_endpoint` | String | "http://localhost:11434" | AI 模型端点 |
| `api_key` | String | "" | API 密钥 |
| `max_tokens` | u32 | 2048 | 最大 token 数 |
| `temperature` | f32 | 0.7 | 温度参数 (0.0-2.0) |
| `timeout` | u64 | 30 | 请求超时(秒) |
| `retry_attempts` | u32 | 3 | 重试次数 |

### Redis 配置 (`redis`) - 需要 `redis` 特性

| 参数 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `url` | String | "redis://localhost:6379" | Redis 连接 URL |
| `max_connections` | u32 | 10 | 最大连接数 |
| `connection_timeout` | u64 | 5 | 连接超时(秒) |
| `response_timeout` | u64 | 5 | 响应超时(秒) |

### 安全配置 (`security`)

| 参数 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `jwt_secret` | String | "your-super-secret..." | JWT 签名密钥 |
| `jwt_expiration` | u64 | 3600 | JWT 过期时间(秒) |
| `bcrypt_cost` | u32 | 12 | bcrypt 哈希成本 |
| `cors_origins` | Vec<String> | ["*"] | CORS 允许的源 |
| `rate_limit_requests` | u32 | 100 | 限流请求数 |
| `rate_limit_window` | u64 | 60 | 限流时间窗口(秒) |

### 存储配置 (`storage`)

| 参数 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `path` | String | "./storage" | 存储路径 |
| `max_file_size` | u64 | 10485760 | 最大文件大小(字节) |
| `allowed_extensions` | Vec<String> | ["pdf", "txt", ...] | 允许的文件扩展名 |

### 日志配置 (`logging`)

| 参数 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `level` | String | "info" | 日志级别 |
| `format` | String | "json" | 日志格式 |
| `file_enabled` | bool | false | 是否启用文件日志 |
| `file_path` | Option<String> | None | 日志文件路径 |
| `max_file_size` | Option<u64> | Some(104857600) | 最大日志文件大小 |
| `max_files` | Option<u32> | Some(10) | 最大日志文件数 |

### 向量配置 (`vector`)

| 参数 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `dimension` | u32 | 1536 | 向量维度 |
| `similarity_threshold` | f32 | 0.8 | 相似度阈值 |
| `index_type` | String | "hnsw" | 索引类型 |
| `ef_construction` | u32 | 200 | HNSW 构建参数 |
| `m` | u32 | 16 | HNSW 连接数 |

### 环境配置 (`environment`)

| 参数 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `name` | String | "development" | 环境名称 |
| `debug` | bool | true | 调试模式 |
| `version` | String | "0.1.0" | 应用版本 |

## 使用示例

### 在代码中访问配置

```rust
use crate::config::ConfigLoader;

// 获取配置
let config = ConfigLoader::get();

// 使用配置
println!("服务器运行在: {}:{}", config.server.host, config.server.port);

// 检查环境
if config.is_development() {
    println!("开发模式");
}
```

### 初始化配置

```rust
use crate::config::ConfigLoader;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // 初始化配置
    let config = ConfigLoader::init()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
    
    // 打印配置摘要
    ConfigLoader::print_summary();
    
    // 启动服务器...
}
```

## 最佳实践

1. **生产环境**：使用环境变量而不是配置文件存储敏感信息
2. **开发环境**：使用 `config.toml` 文件方便调试
3. **测试环境**：设置 `AIONIX_ENVIRONMENT__NAME=test`
4. **安全性**：确保 JWT 密钥足够复杂且定期更换
5. **性能**：根据硬件资源调整连接池大小和工作线程数

## 故障排除

### 配置验证失败

检查错误信息，确保所有必需的配置项都已正确设置：

```bash
# 检查必需的环境变量
export DATABASE_URL=postgresql://user:pass@localhost:5432/aionix
export JWT_SECRET=your-32-character-or-longer-secret-key
```

### 配置文件格式错误

确保 TOML 文件格式正确：

```bash
# 验证 TOML 语法
cargo run --bin config-check  # (如果实现了配置检查工具)
```

### 环境变量优先级

记住环境变量会覆盖配置文件中的设置，检查是否有意外的环境变量设置。