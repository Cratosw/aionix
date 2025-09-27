# 插件开发最佳实践

本文档提供了开发 Aionix AI Studio 插件的最佳实践指南，帮助开发者创建高质量、安全、高性能的插件。

## 设计原则

### 1. 单一职责原则
每个插件应该专注于一个特定的功能领域：

```rust
// ✅ 好的设计 - 专注于文件操作
pub struct FileOperationsPlugin {
    // 只处理文件相关操作
}

// ❌ 避免 - 功能过于复杂
pub struct EverythingPlugin {
    // 文件操作 + 网络请求 + 数据库 + AI 处理...
}
```

### 2. 接口隔离原则
提供清晰、简洁的 API 接口：

```rust
// ✅ 清晰的方法名和参数
async fn read_file(&self, path: &str) -> Result<String, AiStudioError>

// ❌ 模糊的方法名
async fn do_something(&self, data: &str) -> Result<serde_json::Value, AiStudioError>
```

### 3. 依赖倒置原则
依赖抽象而不是具体实现：

```rust
// ✅ 使用 trait 抽象
pub trait StorageProvider: Send + Sync {
    async fn store(&self, key: &str, data: &[u8]) -> Result<(), AiStudioError>;
    async fn retrieve(&self, key: &str) -> Result<Vec<u8>, AiStudioError>;
}

// 插件可以使用任何实现了 StorageProvider 的存储后端
```

## 代码质量

### 1. 错误处理

#### 使用适当的错误类型
```rust
// ✅ 具体的错误类型
match operation() {
    Ok(result) => Ok(result),
    Err(e) if e.kind() == io::ErrorKind::NotFound => {
        Err(AiStudioError::not_found("文件不存在"))
    },
    Err(e) if e.kind() == io::ErrorKind::PermissionDenied => {
        Err(AiStudioError::permission_denied("权限不足"))
    },
    Err(e) => Err(AiStudioError::io(e.to_string())),
}

// ❌ 泛化的错误处理
match operation() {
    Ok(result) => Ok(result),
    Err(e) => Err(AiStudioError::internal(&e.to_string())),
}
```

#### 提供有用的错误信息
```rust
// ✅ 详细的错误信息
if !path.exists() {
    return Err(AiStudioError::not_found(&format!(
        "文件不存在: {} (当前目录: {})", 
        path.display(), 
        std::env::current_dir()?.display()
    )));
}

// ❌ 模糊的错误信息
if !path.exists() {
    return Err(AiStudioError::not_found("文件不存在"));
}
```

### 2. 输入验证

#### 严格验证所有输入
```rust
fn validate_email(email: &str) -> Result<(), AiStudioError> {
    if email.is_empty() {
        return Err(AiStudioError::validation("邮箱地址不能为空"));
    }
    
    if !email.contains('@') {
        return Err(AiStudioError::validation("邮箱地址格式无效"));
    }
    
    if email.len() > 254 {
        return Err(AiStudioError::validation("邮箱地址过长"));
    }
    
    // 使用正则表达式进行更严格的验证
    let email_regex = regex::Regex::new(r"^[^\s@]+@[^\s@]+\.[^\s@]+$")?;
    if !email_regex.is_match(email) {
        return Err(AiStudioError::validation("邮箱地址格式无效"));
    }
    
    Ok(())
}
```

#### 防止注入攻击
```rust
// ✅ 安全的路径处理
fn validate_path(path: &str) -> Result<PathBuf, AiStudioError> {
    let path_buf = PathBuf::from(path);
    
    // 检查路径遍历
    if path.contains("..") || path.contains("~") {
        return Err(AiStudioError::validation("路径包含非法字符"));
    }
    
    // 规范化路径
    let canonical = path_buf.canonicalize()
        .map_err(|_| AiStudioError::validation("无效的路径"))?;
    
    Ok(canonical)
}

// ❌ 不安全的路径处理
fn unsafe_path_handling(path: &str) -> PathBuf {
    PathBuf::from(path) // 直接使用用户输入
}
```

### 3. 资源管理

#### 正确管理资源生命周期
```rust
pub struct DatabasePlugin {
    connection_pool: Option<Pool<ConnectionManager<PgConnection>>>,
}

impl Plugin for DatabasePlugin {
    async fn initialize(&mut self, config: PluginConfig) -> Result<(), AiStudioError> {
        let database_url = config.parameters.get("database_url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AiStudioError::validation("缺少数据库 URL"))?;
        
        let manager = ConnectionManager::<PgConnection>::new(database_url);
        let pool = Pool::builder()
            .max_size(10)
            .build(manager)
            .map_err(|e| AiStudioError::internal(&format!("创建连接池失败: {}", e)))?;
        
        self.connection_pool = Some(pool);
        Ok(())
    }
    
    async fn shutdown(&mut self) -> Result<(), AiStudioError> {
        if let Some(pool) = self.connection_pool.take() {
            // 优雅关闭连接池
            drop(pool);
        }
        Ok(())
    }
}
```

#### 使用 RAII 模式
```rust
pub struct FileHandle {
    file: std::fs::File,
    path: PathBuf,
}

impl FileHandle {
    pub fn open(path: PathBuf) -> Result<Self, AiStudioError> {
        let file = std::fs::File::open(&path)
            .map_err(|e| AiStudioError::io(format!("打开文件失败: {}", e)))?;
        
        Ok(Self { file, path })
    }
}

impl Drop for FileHandle {
    fn drop(&mut self) {
        // 自动清理资源
        debug!("关闭文件: {:?}", self.path);
    }
}
```

## 性能优化

### 1. 异步编程

#### 使用异步操作避免阻塞
```rust
// ✅ 异步操作
async fn process_files(&self, files: Vec<PathBuf>) -> Result<Vec<String>, AiStudioError> {
    let mut tasks = Vec::new();
    
    for file in files {
        let task = tokio::spawn(async move {
            tokio::fs::read_to_string(file).await
        });
        tasks.push(task);
    }
    
    let mut results = Vec::new();
    for task in tasks {
        let content = task.await
            .map_err(|e| AiStudioError::internal(&format!("任务执行失败: {}", e)))?
            .map_err(|e| AiStudioError::io(format!("读取文件失败: {}", e)))?;
        results.push(content);
    }
    
    Ok(results)
}

// ❌ 同步操作（阻塞）
fn process_files_sync(&self, files: Vec<PathBuf>) -> Result<Vec<String>, AiStudioError> {
    let mut results = Vec::new();
    for file in files {
        let content = std::fs::read_to_string(file)?; // 阻塞操作
        results.push(content);
    }
    Ok(results)
}
```

#### 合理使用并发
```rust
// ✅ 控制并发数量
async fn download_urls(&self, urls: Vec<String>) -> Result<Vec<String>, AiStudioError> {
    use futures::stream::{self, StreamExt};
    
    let client = reqwest::Client::new();
    let concurrent_limit = 10; // 限制并发数
    
    let results: Result<Vec<_>, _> = stream::iter(urls)
        .map(|url| {
            let client = client.clone();
            async move {
                client.get(&url).send().await?.text().await
            }
        })
        .buffer_unordered(concurrent_limit)
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .collect();
    
    results.map_err(|e| AiStudioError::network(e.to_string()))
}
```

### 2. 缓存策略

#### 实现智能缓存
```rust
use std::collections::HashMap;
use std::time::{Duration, Instant};

pub struct CachedPlugin {
    cache: HashMap<String, CacheEntry>,
    cache_ttl: Duration,
}

struct CacheEntry {
    data: serde_json::Value,
    created_at: Instant,
}

impl CachedPlugin {
    async fn get_with_cache(&mut self, key: &str) -> Result<serde_json::Value, AiStudioError> {
        // 检查缓存
        if let Some(entry) = self.cache.get(key) {
            if entry.created_at.elapsed() < self.cache_ttl {
                debug!("缓存命中: {}", key);
                return Ok(entry.data.clone());
            } else {
                // 缓存过期，移除
                self.cache.remove(key);
            }
        }
        
        // 缓存未命中，获取数据
        debug!("缓存未命中: {}", key);
        let data = self.fetch_data(key).await?;
        
        // 更新缓存
        self.cache.insert(key.to_string(), CacheEntry {
            data: data.clone(),
            created_at: Instant::now(),
        });
        
        Ok(data)
    }
    
    async fn fetch_data(&self, key: &str) -> Result<serde_json::Value, AiStudioError> {
        // 实际的数据获取逻辑
        todo!()
    }
}
```

### 3. 内存管理

#### 避免内存泄漏
```rust
// ✅ 使用弱引用避免循环引用
use std::rc::{Rc, Weak};
use std::cell::RefCell;

struct Parent {
    children: Vec<Rc<RefCell<Child>>>,
}

struct Child {
    parent: Weak<RefCell<Parent>>, // 使用弱引用
}

// ✅ 及时释放大对象
async fn process_large_data(&self, data: Vec<u8>) -> Result<String, AiStudioError> {
    let processed = expensive_operation(data).await?;
    // data 在这里自动释放
    
    Ok(processed)
}
```

## 安全性

### 1. 输入验证和清理

#### 防止路径遍历攻击
```rust
fn secure_path_join(base: &Path, user_path: &str) -> Result<PathBuf, AiStudioError> {
    let user_path = user_path.trim_start_matches('/');
    let joined = base.join(user_path);
    
    // 确保结果路径在基础路径内
    let canonical_base = base.canonicalize()
        .map_err(|_| AiStudioError::validation("无效的基础路径"))?;
    let canonical_joined = joined.canonicalize()
        .map_err(|_| AiStudioError::validation("无效的目标路径"))?;
    
    if !canonical_joined.starts_with(&canonical_base) {
        return Err(AiStudioError::validation("路径超出允许范围"));
    }
    
    Ok(canonical_joined)
}
```

#### SQL 注入防护
```rust
// ✅ 使用参数化查询
async fn get_user_by_id(&self, user_id: i32) -> Result<User, AiStudioError> {
    let query = "SELECT * FROM users WHERE id = $1";
    let row = sqlx::query_as::<_, User>(query)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AiStudioError::database(e.to_string()))?;
    
    Ok(row)
}

// ❌ 字符串拼接（易受注入攻击）
async fn get_user_by_name_unsafe(&self, name: &str) -> Result<User, AiStudioError> {
    let query = format!("SELECT * FROM users WHERE name = '{}'", name);
    // 危险！容易受到 SQL 注入攻击
    todo!()
}
```

### 2. 权限控制

#### 实现最小权限原则
```rust
#[derive(Debug, Clone)]
pub struct PermissionChecker {
    allowed_operations: HashSet<String>,
    allowed_paths: Vec<PathBuf>,
}

impl PermissionChecker {
    pub fn check_operation(&self, operation: &str) -> Result<(), AiStudioError> {
        if !self.allowed_operations.contains(operation) {
            return Err(AiStudioError::permission_denied(&format!(
                "操作未授权: {}", operation
            )));
        }
        Ok(())
    }
    
    pub fn check_path_access(&self, path: &Path) -> Result<(), AiStudioError> {
        let canonical_path = path.canonicalize()
            .map_err(|_| AiStudioError::validation("无效路径"))?;
        
        for allowed_path in &self.allowed_paths {
            if canonical_path.starts_with(allowed_path) {
                return Ok(());
            }
        }
        
        Err(AiStudioError::permission_denied("路径访问被拒绝"))
    }
}
```

### 3. 敏感数据处理

#### 安全存储敏感信息
```rust
use secrecy::{Secret, ExposeSecret};

pub struct ApiKeyPlugin {
    api_key: Option<Secret<String>>,
}

impl ApiKeyPlugin {
    pub fn set_api_key(&mut self, key: String) {
        self.api_key = Some(Secret::new(key));
    }
    
    async fn make_authenticated_request(&self, url: &str) -> Result<String, AiStudioError> {
        let api_key = self.api_key.as_ref()
            .ok_or_else(|| AiStudioError::validation("API 密钥未设置"))?;
        
        let client = reqwest::Client::new();
        let response = client
            .get(url)
            .header("Authorization", format!("Bearer {}", api_key.expose_secret()))
            .send()
            .await
            .map_err(|e| AiStudioError::network(e.to_string()))?;
        
        response.text().await
            .map_err(|e| AiStudioError::network(e.to_string()))
    }
}

// API 密钥在 Drop 时自动清零
impl Drop for ApiKeyPlugin {
    fn drop(&mut self) {
        if let Some(key) = self.api_key.take() {
            // secrecy crate 会自动清零内存
            drop(key);
        }
    }
}
```

## 测试策略

### 1. 单元测试

#### 测试覆盖关键路径
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_file_operations() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut plugin = FileOperationsPlugin::new();
        
        // 测试初始化
        let config = create_test_config(temp_dir.path());
        plugin.initialize(config).await.unwrap();
        plugin.start().await.unwrap();
        
        // 测试写入文件
        let write_result = plugin.write_file("test.txt", "Hello, World!").await;
        assert!(write_result.is_ok());
        
        // 测试读取文件
        let read_result = plugin.read_file("test.txt").await;
        assert_eq!(read_result.unwrap(), "Hello, World!");
        
        // 测试删除文件
        let delete_result = plugin.delete_file("test.txt").await;
        assert!(delete_result.is_ok());
        
        // 清理
        plugin.stop().await.unwrap();
        plugin.shutdown().await.unwrap();
    }
    
    #[tokio::test]
    async fn test_error_handling() {
        let mut plugin = FileOperationsPlugin::new();
        let config = create_test_config("/nonexistent");
        
        // 测试无效配置
        let result = plugin.initialize(config).await;
        assert!(result.is_err());
        
        // 测试读取不存在的文件
        let result = plugin.read_file("nonexistent.txt").await;
        assert!(result.is_err());
    }
}
```

### 2. 集成测试

#### 测试插件间交互
```rust
#[tokio::test]
async fn test_plugin_integration() {
    let mut file_plugin = FileOperationsPlugin::new();
    let mut http_plugin = HttpClientPlugin::new();
    
    // 初始化插件
    file_plugin.initialize(create_file_config()).await.unwrap();
    http_plugin.initialize(create_http_config()).await.unwrap();
    
    file_plugin.start().await.unwrap();
    http_plugin.start().await.unwrap();
    
    // 测试工作流：下载文件 -> 保存到磁盘
    let url = "https://httpbin.org/json";
    let response = http_plugin.get(url).await.unwrap();
    let content = response["body"].as_str().unwrap();
    
    file_plugin.write_file("downloaded.json", content).await.unwrap();
    
    let saved_content = file_plugin.read_file("downloaded.json").await.unwrap();
    assert_eq!(content, saved_content);
    
    // 清理
    file_plugin.stop().await.unwrap();
    http_plugin.stop().await.unwrap();
    file_plugin.shutdown().await.unwrap();
    http_plugin.shutdown().await.unwrap();
}
```

### 3. 性能测试

#### 基准测试
```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_file_operations(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut plugin = FileOperationsPlugin::new();
    
    rt.block_on(async {
        plugin.initialize(create_test_config()).await.unwrap();
        plugin.start().await.unwrap();
    });
    
    c.bench_function("write_small_file", |b| {
        b.to_async(&rt).iter(|| async {
            plugin.write_file(
                black_box("benchmark.txt"),
                black_box("Hello, World!")
            ).await.unwrap();
        });
    });
    
    c.bench_function("read_small_file", |b| {
        b.to_async(&rt).iter(|| async {
            plugin.read_file(black_box("benchmark.txt")).await.unwrap();
        });
    });
}

criterion_group!(benches, benchmark_file_operations);
criterion_main!(benches);
```

## 文档和维护

### 1. 代码文档

#### 编写清晰的文档注释
```rust
/// 文件操作插件
/// 
/// 提供安全的文件系统操作功能，包括读写、创建、删除文件和目录。
/// 所有操作都在配置的基础路径内进行，防止路径遍历攻击。
/// 
/// # 示例
/// 
/// ```rust
/// use file_operations_plugin::FileOperationsPlugin;
/// 
/// let mut plugin = FileOperationsPlugin::new();
/// plugin.initialize(config).await?;
/// plugin.start().await?;
/// 
/// // 写入文件
/// plugin.write_file("example.txt", "Hello, World!").await?;
/// 
/// // 读取文件
/// let content = plugin.read_file("example.txt").await?;
/// assert_eq!(content, "Hello, World!");
/// ```
/// 
/// # 安全性
/// 
/// - 所有路径操作都经过验证，防止路径遍历攻击
/// - 支持基础路径限制，确保操作在允许范围内
/// - 文件大小限制，防止资源耗尽
pub struct FileOperationsPlugin {
    // ...
}

/// 读取文件内容
/// 
/// # 参数
/// 
/// * `path` - 相对于基础路径的文件路径
/// 
/// # 返回
/// 
/// 返回文件的文本内容。如果文件不存在或无法读取，返回错误。
/// 
/// # 错误
/// 
/// * `AiStudioError::NotFound` - 文件不存在
/// * `AiStudioError::PermissionDenied` - 权限不足
/// * `AiStudioError::Validation` - 路径无效
/// * `AiStudioError::Io` - I/O 错误
/// 
/// # 示例
/// 
/// ```rust
/// let content = plugin.read_file("config.json").await?;
/// println!("配置内容: {}", content);
/// ```
pub async fn read_file(&self, path: &str) -> Result<String, AiStudioError> {
    // ...
}
```

### 2. 版本管理

#### 语义化版本控制
```toml
[package]
name = "my-plugin"
version = "1.2.3"  # MAJOR.MINOR.PATCH

# MAJOR: 不兼容的 API 变更
# MINOR: 向后兼容的功能添加
# PATCH: 向后兼容的错误修复
```

#### 变更日志
```markdown
# 变更日志

## [1.2.3] - 2024-01-15

### 修复
- 修复了文件路径验证中的安全漏洞
- 改进了错误消息的准确性

### 变更
- 提升了大文件处理的性能

## [1.2.0] - 2024-01-01

### 新增
- 添加了批量文件操作支持
- 新增了文件压缩功能

### 废弃
- `old_method()` 已废弃，请使用 `new_method()`
```

### 3. 监控和日志

#### 结构化日志
```rust
use tracing::{info, warn, error, debug, instrument};

impl FileOperationsPlugin {
    #[instrument(skip(self), fields(path = %path))]
    pub async fn read_file(&self, path: &str) -> Result<String, AiStudioError> {
        debug!("开始读取文件");
        
        let validated_path = self.validate_path(path)?;
        
        match tokio::fs::read_to_string(&validated_path).await {
            Ok(content) => {
                info!(
                    size = content.len(),
                    "文件读取成功"
                );
                Ok(content)
            },
            Err(e) => {
                error!(
                    error = %e,
                    "文件读取失败"
                );
                Err(AiStudioError::io(e.to_string()))
            }
        }
    }
}
```

#### 指标收集
```rust
use prometheus::{Counter, Histogram, register_counter, register_histogram};

lazy_static! {
    static ref FILE_OPERATIONS_TOTAL: Counter = register_counter!(
        "file_operations_total",
        "Total number of file operations"
    ).unwrap();
    
    static ref FILE_OPERATION_DURATION: Histogram = register_histogram!(
        "file_operation_duration_seconds",
        "Duration of file operations"
    ).unwrap();
}

impl FileOperationsPlugin {
    pub async fn read_file(&self, path: &str) -> Result<String, AiStudioError> {
        let _timer = FILE_OPERATION_DURATION.start_timer();
        FILE_OPERATIONS_TOTAL.inc();
        
        // 实际操作...
        
        Ok(content)
    }
}
```

## 部署和分发

### 1. 容器化

#### Dockerfile 示例
```dockerfile
FROM rust:1.70 as builder

WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/my-plugin /usr/local/bin/

EXPOSE 8080
CMD ["my-plugin"]
```

### 2. CI/CD 流水线

#### GitHub Actions 示例
```yaml
name: Plugin CI/CD

on:
  push:
    branches: [ main ]
  pull_request:
    branches: [ main ]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v3
    
    - name: Setup Rust
      uses: actions-rs/toolchain@v1
      with:
        toolchain: stable
        
    - name: Run tests
      run: cargo test --all-features
      
    - name: Run clippy
      run: cargo clippy -- -D warnings
      
    - name: Check formatting
      run: cargo fmt -- --check

  security:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v3
    
    - name: Security audit
      run: cargo audit
      
    - name: Dependency check
      run: cargo deny check

  build:
    needs: [test, security]
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v3
    
    - name: Build plugin
      run: cargo build --release
      
    - name: Upload artifacts
      uses: actions/upload-artifact@v3
      with:
        name: plugin-binary
        path: target/release/my-plugin
```

## 总结

遵循这些最佳实践可以帮助你开发出：

1. **安全可靠**的插件 - 通过输入验证、权限控制和安全编码实践
2. **高性能**的插件 - 通过异步编程、缓存和资源管理
3. **易维护**的插件 - 通过清晰的代码结构、文档和测试
4. **可扩展**的插件 - 通过模块化设计和接口抽象

记住，好的插件不仅要功能完善，还要考虑安全性、性能、可维护性和用户体验。持续改进和学习新的最佳实践是成为优秀插件开发者的关键。