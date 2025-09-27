# 插件 API 参考

本文档提供了 Aionix AI Studio 插件系统的完整 API 参考。

## 核心接口

### Plugin Trait

所有插件都必须实现 `Plugin` trait：

```rust
#[async_trait]
pub trait Plugin: Send + Sync {
    fn metadata(&self) -> PluginMetadata;
    async fn initialize(&mut self, config: PluginConfig) -> Result<(), AiStudioError>;
    async fn start(&mut self) -> Result<(), AiStudioError>;
    async fn stop(&mut self) -> Result<(), AiStudioError>;
    async fn shutdown(&mut self) -> Result<(), AiStudioError>;
    fn status(&self) -> PluginStatus;
    async fn handle_call(
        &self,
        method: &str,
        params: HashMap<String, serde_json::Value>,
        context: &PluginContext,
    ) -> Result<serde_json::Value, AiStudioError>;
    async fn health_check(&self) -> Result<PluginHealth, AiStudioError>;
    fn config_schema(&self) -> serde_json::Value;
    fn validate_config(&self, config: &PluginConfig) -> Result<(), AiStudioError>;
}
```

#### 方法说明

##### `metadata() -> PluginMetadata`

返回插件的元数据信息。

**返回值:**
- `PluginMetadata` - 插件元数据

**示例:**
```rust
fn metadata(&self) -> PluginMetadata {
    PluginMetadata {
        id: "my-plugin".to_string(),
        name: "我的插件".to_string(),
        version: "1.0.0".to_string(),
        description: "插件描述".to_string(),
        author: "作者名".to_string(),
        license: "MIT".to_string(),
        plugin_type: PluginType::Tool,
        api_version: "1.0".to_string(),
        min_system_version: "1.0.0".to_string(),
        dependencies: Vec::new(),
        permissions: vec![PluginPermission::FileSystem],
        tags: vec!["utility".to_string()],
        // ...
    }
}
```

##### `initialize(&mut self, config: PluginConfig) -> Result<(), AiStudioError>`

初始化插件，设置配置参数。

**参数:**
- `config: PluginConfig` - 插件配置

**返回值:**
- `Result<(), AiStudioError>` - 成功返回 `Ok(())`，失败返回错误

**示例:**
```rust
async fn initialize(&mut self, config: PluginConfig) -> Result<(), AiStudioError> {
    // 验证配置
    self.validate_config(&config)?;
    
    // 设置配置参数
    if let Some(timeout) = config.parameters.get("timeout") {
        self.timeout = Duration::from_secs(timeout.as_u64().unwrap_or(30));
    }
    
    self.config = Some(config);
    self.status = PluginStatus::Initialized;
    
    Ok(())
}
```

##### `start(&mut self) -> Result<(), AiStudioError>`

启动插件，使其进入运行状态。

**返回值:**
- `Result<(), AiStudioError>` - 成功返回 `Ok(())`，失败返回错误

##### `stop(&mut self) -> Result<(), AiStudioError>`

停止插件，但保持资源不释放。

**返回值:**
- `Result<(), AiStudioError>` - 成功返回 `Ok(())`，失败返回错误

##### `shutdown(&mut self) -> Result<(), AiStudioError>`

关闭插件，释放所有资源。

**返回值:**
- `Result<(), AiStudioError>` - 成功返回 `Ok(())`，失败返回错误

##### `status(&self) -> PluginStatus`

返回插件当前状态。

**返回值:**
- `PluginStatus` - 插件状态枚举

##### `handle_call(&self, method: &str, params: HashMap<String, serde_json::Value>, context: &PluginContext) -> Result<serde_json::Value, AiStudioError>`

处理插件方法调用。

**参数:**
- `method: &str` - 方法名
- `params: HashMap<String, serde_json::Value>` - 方法参数
- `context: &PluginContext` - 调用上下文

**返回值:**
- `Result<serde_json::Value, AiStudioError>` - 方法执行结果

**示例:**
```rust
async fn handle_call(
    &self,
    method: &str,
    params: HashMap<String, serde_json::Value>,
    context: &PluginContext,
) -> Result<serde_json::Value, AiStudioError> {
    match method {
        "echo" => {
            let message = params.get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("Hello");
            
            Ok(serde_json::json!({
                "echo": message,
                "timestamp": chrono::Utc::now()
            }))
        },
        _ => Err(AiStudioError::validation(&format!("未知方法: {}", method)))
    }
}
```

##### `health_check(&self) -> Result<PluginHealth, AiStudioError>`

执行健康检查。

**返回值:**
- `Result<PluginHealth, AiStudioError>` - 健康检查结果

##### `config_schema(&self) -> serde_json::Value`

返回插件配置的 JSON Schema。

**返回值:**
- `serde_json::Value` - JSON Schema

##### `validate_config(&self, config: &PluginConfig) -> Result<(), AiStudioError>`

验证插件配置。

**参数:**
- `config: &PluginConfig` - 要验证的配置

**返回值:**
- `Result<(), AiStudioError>` - 验证结果

## 数据结构

### PluginMetadata

插件元数据结构：

```rust
#[derive(Debug, Clone, Serialize)]
pub struct PluginMetadata {
    pub id: String,                                    // 插件唯一标识
    pub name: String,                                  // 插件名称
    pub version: String,                               // 插件版本
    pub description: String,                           // 插件描述
    pub author: String,                                // 作者
    pub license: String,                               // 许可证
    pub homepage: Option<String>,                      // 主页 URL
    pub repository: Option<String>,                    // 仓库 URL
    pub plugin_type: PluginType,                       // 插件类型
    pub api_version: String,                           // API 版本
    pub min_system_version: String,                    // 最小系统版本
    pub dependencies: Vec<PluginDependency>,           // 依赖列表
    pub permissions: Vec<PluginPermission>,            // 权限列表
    pub tags: Vec<String>,                             // 标签
    pub icon: Option<String>,                          // 图标
    pub created_at: chrono::DateTime<chrono::Utc>,     // 创建时间
}
```

### PluginConfig

插件配置结构：

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    pub plugin_id: String,                             // 插件 ID
    pub parameters: HashMap<String, serde_json::Value>, // 配置参数
    pub environment: HashMap<String, String>,          // 环境变量
    pub resource_limits: ResourceLimits,               // 资源限制
    pub security_settings: SecuritySettings,          // 安全设置
}
```

### PluginContext

插件调用上下文：

```rust
#[derive(Debug, Clone)]
pub struct PluginContext {
    pub tenant_id: Uuid,                              // 租户 ID
    pub user_id: Option<Uuid>,                        // 用户 ID
    pub session_id: Option<Uuid>,                     // 会话 ID
    pub request_id: Uuid,                             // 请求 ID
    pub variables: HashMap<String, serde_json::Value>, // 上下文变量
    pub timestamp: chrono::DateTime<chrono::Utc>,      // 时间戳
}
```

### PluginHealth

健康检查结果：

```rust
#[derive(Debug, Clone, Serialize)]
pub struct PluginHealth {
    pub healthy: bool,                                 // 是否健康
    pub message: String,                               // 状态消息
    pub details: HashMap<String, serde_json::Value>,   // 详细信息
    pub checked_at: chrono::DateTime<chrono::Utc>,     // 检查时间
    pub response_time_ms: u64,                         // 响应时间（毫秒）
}
```

### ResourceLimits

资源限制配置：

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    pub max_memory_mb: Option<u64>,                    // 最大内存（MB）
    pub max_cpu_percent: Option<f32>,                  // 最大 CPU 使用率
    pub max_disk_mb: Option<u64>,                      // 最大磁盘使用（MB）
    pub max_network_kbps: Option<u64>,                 // 最大网络带宽（KB/s）
    pub max_execution_seconds: Option<u64>,            // 最大执行时间（秒）
}
```

### SecuritySettings

安全设置：

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecuritySettings {
    pub enable_sandbox: bool,                          // 启用沙箱
    pub allowed_domains: Vec<String>,                  // 允许的域名
    pub allowed_paths: Vec<String>,                    // 允许的路径
    pub forbidden_operations: Vec<String>,             // 禁止的操作
}
```

## 枚举类型

### PluginStatus

插件状态枚举：

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PluginStatus {
    Uninitialized,    // 未初始化
    Initializing,     // 初始化中
    Initialized,      // 已初始化
    Starting,         // 启动中
    Running,          // 运行中
    Stopping,         // 停止中
    Stopped,          // 已停止
    Error,            // 错误状态
    Unloading,        // 卸载中
    Unloaded,         // 已卸载
}
```

### PluginType

插件类型枚举：

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PluginType {
    Tool,             // 工具插件
    Agent,            // AI Agent 插件
    Workflow,         // 工作流插件
    DataSource,       // 数据源插件
    Authentication,   // 认证插件
    Storage,          // 存储插件
    Notification,     // 通知插件
    Monitoring,       // 监控插件
    Custom,           // 自定义插件
}
```

### PluginPermission

插件权限枚举：

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PluginPermission {
    FileSystem,       // 文件系统访问
    Network,          // 网络访问
    Database,         // 数据库访问
    SystemInfo,       // 系统信息访问
    UserData,         // 用户数据访问
    Admin,            // 管理员权限
    Custom(String),   // 自定义权限
}
```

## 错误处理

### AiStudioError

统一错误类型：

```rust
#[derive(Debug, thiserror::Error)]
pub enum AiStudioError {
    #[error("验证错误: {message}")]
    Validation { message: String },
    
    #[error("未找到: {message}")]
    NotFound { message: String },
    
    #[error("权限被拒绝: {message}")]
    PermissionDenied { message: String },
    
    #[error("网络错误: {message}")]
    Network { message: String },
    
    #[error("数据库错误: {message}")]
    Database { message: String },
    
    #[error("I/O 错误: {message}")]
    Io { message: String },
    
    #[error("超时: {message}")]
    Timeout { message: String },
    
    #[error("内部错误: {message}")]
    Internal { message: String },
}
```

#### 错误构造方法

```rust
impl AiStudioError {
    pub fn validation(msg: &str) -> Self {
        Self::Validation { message: msg.to_string() }
    }
    
    pub fn not_found(msg: &str) -> Self {
        Self::NotFound { message: msg.to_string() }
    }
    
    pub fn permission_denied(msg: &str) -> Self {
        Self::PermissionDenied { message: msg.to_string() }
    }
    
    pub fn network(msg: String) -> Self {
        Self::Network { message: msg }
    }
    
    pub fn database(msg: String) -> Self {
        Self::Database { message: msg }
    }
    
    pub fn io(msg: String) -> Self {
        Self::Io { message: msg }
    }
    
    pub fn timeout(msg: &str) -> Self {
        Self::Timeout { message: msg.to_string() }
    }
    
    pub fn internal(msg: &str) -> Self {
        Self::Internal { message: msg.to_string() }
    }
}
```

## 插件工厂

### PluginFactory Trait

插件工厂接口：

```rust
pub trait PluginFactory: Send + Sync {
    fn create_plugin(&self) -> Result<Box<dyn Plugin>, AiStudioError>;
    fn metadata(&self) -> PluginMetadata;
    fn validate_compatibility(&self, system_version: &str) -> Result<(), AiStudioError>;
}
```

#### 实现示例

```rust
pub struct MyPluginFactory;

impl PluginFactory for MyPluginFactory {
    fn create_plugin(&self) -> Result<Box<dyn Plugin>, AiStudioError> {
        Ok(Box::new(MyPlugin::new()))
    }
    
    fn metadata(&self) -> PluginMetadata {
        MyPlugin::new().metadata()
    }
    
    fn validate_compatibility(&self, system_version: &str) -> Result<(), AiStudioError> {
        let min_version = semver::Version::parse("1.0.0")
            .map_err(|e| AiStudioError::internal(&format!("版本解析失败: {}", e)))?;
        
        let current_version = semver::Version::parse(system_version)
            .map_err(|e| AiStudioError::validation(&format!("无效的系统版本: {}", e)))?;
        
        if current_version < min_version {
            return Err(AiStudioError::validation(&format!(
                "系统版本过低，需要 {} 或更高版本，当前版本: {}",
                min_version, current_version
            )));
        }
        
        Ok(())
    }
}
```

## 插件 API 接口

### PluginApi Trait

插件可以通过此接口访问系统功能：

```rust
#[async_trait]
pub trait PluginApi: Send + Sync {
    // 日志记录
    async fn log(&self, level: LogLevel, message: &str, context: Option<&PluginContext>) -> Result<(), AiStudioError>;
    
    // 配置管理
    async fn get_config(&self, key: &str) -> Result<Option<serde_json::Value>, AiStudioError>;
    async fn set_config(&self, key: &str, value: serde_json::Value) -> Result<(), AiStudioError>;
    
    // 插件间调用
    async fn call_plugin(&self, plugin_id: &str, method: &str, params: HashMap<String, serde_json::Value>) -> Result<serde_json::Value, AiStudioError>;
    
    // 事件系统
    async fn emit_event(&self, event: PluginEvent) -> Result<(), AiStudioError>;
    async fn subscribe_event(&self, event_type: &str, callback: Box<dyn Fn(PluginEvent) + Send + Sync>) -> Result<(), AiStudioError>;
    
    // HTTP 请求
    async fn http_request(&self, method: &str, url: &str, headers: Option<HashMap<String, String>>, body: Option<String>) -> Result<HttpResponse, AiStudioError>;
    
    // 数据存储
    async fn store_data(&self, key: &str, data: serde_json::Value) -> Result<(), AiStudioError>;
    async fn retrieve_data(&self, key: &str) -> Result<Option<serde_json::Value>, AiStudioError>;
    async fn delete_data(&self, key: &str) -> Result<(), AiStudioError>;
    
    // 缓存操作
    async fn cache_set(&self, key: &str, value: serde_json::Value, ttl: Option<Duration>) -> Result<(), AiStudioError>;
    async fn cache_get(&self, key: &str) -> Result<Option<serde_json::Value>, AiStudioError>;
    async fn cache_delete(&self, key: &str) -> Result<(), AiStudioError>;
}
```

### LogLevel

日志级别枚举：

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}
```

### PluginEvent

插件事件结构：

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginEvent {
    pub event_id: Uuid,                               // 事件 ID
    pub plugin_id: String,                            // 插件 ID
    pub event_type: PluginEventType,                  // 事件类型
    pub data: serde_json::Value,                      // 事件数据
    pub timestamp: chrono::DateTime<chrono::Utc>,     // 时间戳
}
```

### PluginEventType

插件事件类型：

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PluginEventType {
    Started,          // 插件启动
    Stopped,          // 插件停止
    Error,            // 插件错误
    ConfigChanged,    // 配置变更
    DataUpdated,      // 数据更新
    Custom(String),   // 自定义事件
}
```

### HttpResponse

HTTP 响应结构：

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpResponse {
    pub status: u16,                                  // 状态码
    pub headers: HashMap<String, String>,             // 响应头
    pub body: String,                                 // 响应体
    pub content_type: Option<String>,                 // 内容类型
    pub content_length: Option<u64>,                  // 内容长度
    pub response_time_ms: u64,                        // 响应时间
}
```

## 宏和辅助函数

### 插件注册宏

```rust
/// 注册插件的便捷宏
#[macro_export]
macro_rules! register_plugin {
    ($factory:expr) => {
        #[no_mangle]
        pub extern "C" fn create_plugin_factory() -> *mut dyn PluginFactory {
            Box::into_raw(Box::new($factory))
        }
        
        #[no_mangle]
        pub extern "C" fn get_plugin_metadata() -> *const PluginMetadata {
            Box::into_raw(Box::new($factory.metadata()))
        }
    };
}
```

### 使用示例

```rust
use aionix_ai_studio::register_plugin;

register_plugin!(MyPluginFactory);
```

## 配置 Schema 规范

插件配置应遵循 JSON Schema 规范：

```json
{
  "type": "object",
  "properties": {
    "timeout_seconds": {
      "type": "integer",
      "minimum": 1,
      "maximum": 300,
      "default": 30,
      "description": "请求超时时间（秒）"
    },
    "api_key": {
      "type": "string",
      "minLength": 1,
      "description": "API 密钥",
      "format": "password"
    },
    "enabled_features": {
      "type": "array",
      "items": {
        "type": "string",
        "enum": ["feature1", "feature2", "feature3"]
      },
      "default": ["feature1"],
      "description": "启用的功能列表"
    }
  },
  "required": ["api_key"],
  "additionalProperties": false
}
```

## 版本兼容性

### API 版本控制

插件 API 使用语义化版本控制：

- **主版本号**: 不兼容的 API 变更
- **次版本号**: 向后兼容的功能添加
- **修订版本号**: 向后兼容的错误修复

### 兼容性检查

```rust
fn check_api_compatibility(plugin_api_version: &str, system_api_version: &str) -> Result<(), AiStudioError> {
    let plugin_version = semver::Version::parse(plugin_api_version)?;
    let system_version = semver::Version::parse(system_api_version)?;
    
    // 主版本号必须匹配
    if plugin_version.major != system_version.major {
        return Err(AiStudioError::validation(&format!(
            "API 主版本不兼容: 插件需要 {}.x.x，系统提供 {}.x.x",
            plugin_version.major, system_version.major
        )));
    }
    
    // 插件的次版本号不能高于系统版本
    if plugin_version.minor > system_version.minor {
        return Err(AiStudioError::validation(&format!(
            "API 次版本不兼容: 插件需要 {}.{}.x，系统提供 {}.{}.x",
            plugin_version.major, plugin_version.minor,
            system_version.major, system_version.minor
        )));
    }
    
    Ok(())
}
```

## 最佳实践

### 1. 错误处理

始终使用适当的错误类型：

```rust
// ✅ 好的做法
if !file.exists() {
    return Err(AiStudioError::not_found(&format!("文件不存在: {}", path)));
}

// ❌ 避免
if !file.exists() {
    return Err(AiStudioError::internal("错误"));
}
```

### 2. 异步操作

使用异步操作避免阻塞：

```rust
// ✅ 异步操作
async fn read_file(&self, path: &str) -> Result<String, AiStudioError> {
    tokio::fs::read_to_string(path).await
        .map_err(|e| AiStudioError::io(e.to_string()))
}

// ❌ 同步操作（阻塞）
fn read_file_sync(&self, path: &str) -> Result<String, AiStudioError> {
    std::fs::read_to_string(path)
        .map_err(|e| AiStudioError::io(e.to_string()))
}
```

### 3. 资源管理

正确管理资源生命周期：

```rust
impl Drop for MyPlugin {
    fn drop(&mut self) {
        // 清理资源
        if let Some(connection) = self.connection.take() {
            // 关闭连接
        }
    }
}
```

### 4. 配置验证

提供详细的配置验证：

```rust
fn validate_config(&self, config: &PluginConfig) -> Result<(), AiStudioError> {
    if let Some(timeout) = config.parameters.get("timeout") {
        let timeout_val = timeout.as_u64()
            .ok_or_else(|| AiStudioError::validation("timeout 必须是数字"))?;
        
        if timeout_val == 0 || timeout_val > 300 {
            return Err(AiStudioError::validation("timeout 必须在 1-300 之间"));
        }
    }
    
    Ok(())
}
```

这个 API 参考文档提供了开发 Aionix AI Studio 插件所需的所有接口和数据结构的详细信息。开发者可以参考这些定义来实现自己的插件。