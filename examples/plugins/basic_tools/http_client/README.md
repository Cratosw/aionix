# HTTP 客户端插件

这是一个 Aionix AI Studio 的 HTTP 客户端插件示例，提供完整的 HTTP 请求和响应处理功能。

## 功能特性

### 核心功能
- **HTTP 方法支持**: GET、POST、PUT、DELETE 等标准 HTTP 方法
- **请求定制**: 自定义请求头、请求体、超时时间
- **响应处理**: 完整的响应信息包括状态码、头部、内容
- **文件下载**: 支持文件下载和 Base64 编码

### 安全特性
- **域名白名单**: 可配置允许访问的域名列表
- **SSL 验证**: 支持 SSL 证书验证控制
- **响应大小限制**: 防止过大响应导致内存问题
- **超时控制**: 可配置的请求超时时间

### 高级特性
- **代理支持**: 支持 HTTP/HTTPS 代理
- **自定义 User-Agent**: 可配置请求标识
- **健康检查**: 内置网络连接健康检查
- **错误处理**: 完善的网络错误处理和重试机制

## 安装和使用

### 依赖要求

```toml
[dependencies]
tokio = { version = "1.0", features = ["full"] }
async-trait = "0.1"
reqwest = { version = "0.11", features = ["json", "stream"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
base64 = "0.21"
tracing = "0.1"
uuid = { version = "1.0", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
```

### 基本使用

```rust
use http_client_plugin::*;
use std::collections::HashMap;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建插件实例
    let mut plugin = HttpClientPlugin::new();
    
    // 配置插件
    let mut config_params = HashMap::new();
    config_params.insert("default_timeout_seconds".to_string(), json!(30));
    config_params.insert("max_response_size_mb".to_string(), json!(10));
    
    let config = PluginConfig {
        plugin_id: "http-client".to_string(),
        parameters: config_params,
        environment: HashMap::new(),
        resource_limits: Default::default(),
        security_settings: Default::default(),
    };
    
    // 初始化和启动
    plugin.initialize(config).await?;
    plugin.start().await?;
    
    // 使用插件功能
    // ... 插件调用代码 ...
    
    // 关闭插件
    plugin.stop().await?;
    plugin.shutdown().await?;
    
    Ok(())
}
```

## API 参考

### 支持的方法

#### `get`
执行 GET 请求

**参数:**
- `url` (string): 请求 URL
- `headers` (object, 可选): 请求头
- `timeout` (integer, 可选): 超时时间（秒）

**返回:**
```json
{
  "status": 200,
  "headers": {
    "content-type": "application/json",
    "content-length": "1024"
  },
  "body": "响应内容",
  "content_type": "application/json",
  "content_length": 1024,
  "response_time_ms": 150
}
```

#### `post`
执行 POST 请求

**参数:**
- `url` (string): 请求 URL
- `headers` (object, 可选): 请求头
- `body` (string, 可选): 请求体
- `timeout` (integer, 可选): 超时时间（秒）

**返回:** 同 GET 请求

#### `put`
执行 PUT 请求

**参数:** 同 POST 请求
**返回:** 同 GET 请求

#### `delete`
执行 DELETE 请求

**参数:**
- `url` (string): 请求 URL
- `headers` (object, 可选): 请求头
- `timeout` (integer, 可选): 超时时间（秒）

**返回:** 同 GET 请求

#### `download`
下载文件

**参数:**
- `url` (string): 文件 URL
- `max_size` (integer, 可选): 最大文件大小（字节）

**返回:**
```json
{
  "success": true,
  "url": "https://example.com/file.pdf",
  "size": 1048576,
  "data": "base64编码的文件数据",
  "response_time_ms": 2000
}
```

## 配置选项

### `default_timeout_seconds`
- **类型**: integer
- **描述**: 默认请求超时时间（秒）
- **范围**: 1-300
- **默认值**: 30

### `max_response_size_mb`
- **类型**: integer
- **描述**: 最大响应大小限制（MB）
- **范围**: 1-100
- **默认值**: 10

### `allowed_domains`
- **类型**: array of strings
- **描述**: 允许访问的域名列表（空数组表示无限制）
- **默认值**: []

### `proxy_url`
- **类型**: string
- **描述**: 代理服务器 URL
- **默认值**: null

### `verify_ssl`
- **类型**: boolean
- **描述**: 是否验证 SSL 证书
- **默认值**: true

### `health_check_url`
- **类型**: string
- **描述**: 健康检查 URL
- **默认值**: null

### `user_agent`
- **类型**: string
- **描述**: 自定义 User-Agent
- **默认值**: "Aionix-AI-Studio-HTTP-Plugin/1.0"

### 配置示例

```json
{
  "default_timeout_seconds": 30,
  "max_response_size_mb": 10,
  "allowed_domains": ["api.example.com", "secure.api.com"],
  "proxy_url": "http://proxy.company.com:8080",
  "verify_ssl": true,
  "health_check_url": "https://httpbin.org/status/200",
  "user_agent": "MyApp/1.0"
}
```

## 使用示例

### 基本 GET 请求

```rust
let mut params = HashMap::new();
params.insert("url".to_string(), json!("https://api.example.com/data"));
params.insert("headers".to_string(), json!({
    "Authorization": "Bearer your-token",
    "Accept": "application/json"
}));

let response = plugin.handle_call("get", params, &context).await?;
println!("状态码: {}", response["status"]);
println!("响应体: {}", response["body"]);
```

### POST JSON 数据

```rust
let data = json!({
    "name": "John Doe",
    "email": "john@example.com"
});

let mut params = HashMap::new();
params.insert("url".to_string(), json!("https://api.example.com/users"));
params.insert("headers".to_string(), json!({
    "Content-Type": "application/json",
    "Authorization": "Bearer your-token"
}));
params.insert("body".to_string(), json!(data.to_string()));

let response = plugin.handle_call("post", params, &context).await?;
```

### 文件下载

```rust
let mut params = HashMap::new();
params.insert("url".to_string(), json!("https://example.com/file.pdf"));
params.insert("max_size".to_string(), json!(5242880)); // 5MB

let response = plugin.handle_call("download", params, &context).await?;
let file_data = base64::decode(response["data"].as_str().unwrap())?;
```

### 自定义超时

```rust
let mut params = HashMap::new();
params.insert("url".to_string(), json!("https://slow-api.example.com/data"));
params.insert("timeout".to_string(), json!(60)); // 60 秒超时

let response = plugin.handle_call("get", params, &context).await?;
```

## 错误处理

### 错误类型

- **ValidationError**: 参数验证错误（无效 URL、缺少参数等）
- **NetworkError**: 网络请求错误（连接失败、DNS 解析失败等）
- **TimeoutError**: 请求超时错误
- **InternalError**: 内部错误（客户端初始化失败等）

### 错误示例

```json
{
  "error": "NetworkError",
  "message": "网络请求失败: Connection refused",
  "details": {}
}
```

## 安全考虑

### 网络安全
- 支持域名白名单，限制可访问的外部服务
- SSL 证书验证，防止中间人攻击
- 代理支持，适应企业网络环境

### 资源保护
- 响应大小限制，防止内存耗尽
- 请求超时控制，避免长时间阻塞
- 并发请求限制（通过插件系统控制）

### 数据安全
- 敏感数据（如 API 密钥）通过请求头传递
- 支持 HTTPS 加密传输
- 不记录敏感请求内容

## 性能优化

### 连接复用
- 使用 reqwest 客户端的连接池
- 支持 HTTP/2 和 Keep-Alive
- 自动处理重定向

### 压缩支持
- 自动支持 gzip、deflate、brotli 压缩
- 减少网络传输时间
- 可通过 features 控制

### 异步处理
- 完全异步的网络操作
- 不阻塞插件系统
- 支持并发请求

## 测试

### 运行测试

```bash
# 运行所有测试
cargo test

# 运行特定测试
cargo test test_get_request

# 运行示例
cargo run --example basic_requests
```

### 测试覆盖

- 插件生命周期测试
- HTTP 方法功能测试
- 错误处理测试
- 配置验证测试
- 网络连接测试（需要网络）

## 示例

### 基本请求示例

```bash
cargo run --example basic_requests
```

### 高级功能示例

```bash
cargo run --example advanced_features
```

## 开发指南

### 扩展功能

1. **添加新的 HTTP 方法**: 在插件中实现新的请求方法
2. **自定义认证**: 添加 OAuth、JWT 等认证支持
3. **请求中间件**: 实现请求/响应拦截器
4. **缓存支持**: 添加 HTTP 缓存机制

### 性能调优

1. **连接池配置**: 调整 reqwest 客户端的连接池设置
2. **超时优化**: 根据网络环境调整超时时间
3. **压缩算法**: 选择合适的压缩算法
4. **并发控制**: 实现请求并发限制

### 监控和调试

1. **请求日志**: 记录详细的请求和响应信息
2. **性能指标**: 收集响应时间、成功率等指标
3. **错误追踪**: 详细的错误信息和堆栈跟踪
4. **健康检查**: 定期检查网络连接状态

## 许可证

MIT License - 详见 [LICENSE](LICENSE) 文件

## 贡献

欢迎贡献代码！请遵循以下步骤：

1. Fork 项目
2. 创建功能分支
3. 提交更改
4. 推送到分支
5. 创建 Pull Request

## 支持

如有问题或建议，请：

1. 查看 [文档](../../../docs/)
2. 提交 [Issue](https://github.com/aionix/ai-studio/issues)
3. 参与 [讨论](https://github.com/aionix/ai-studio/discussions)