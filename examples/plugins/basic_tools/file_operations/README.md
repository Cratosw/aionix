# 文件操作插件

这是一个 Aionix AI Studio 的文件操作插件示例，提供基础的文件和目录操作功能。

## 功能特性

### 核心功能
- **文件读写**: 读取和写入文本文件
- **目录操作**: 创建、删除、列出目录内容
- **文件管理**: 复制、删除文件
- **信息查询**: 获取文件和目录的详细信息

### 安全特性
- **路径验证**: 防止路径遍历攻击
- **基础路径限制**: 可配置操作范围
- **权限检查**: 基于插件权限系统
- **参数验证**: 严格的输入参数验证

### 性能特性
- **异步操作**: 基于 tokio 的异步 I/O
- **错误处理**: 完善的错误处理和恢复
- **健康检查**: 实时监控插件状态
- **资源管理**: 可配置的资源限制

## 安装和使用

### 依赖要求

```toml
[dependencies]
tokio = { version = "1.0", features = ["full"] }
async-trait = "0.1"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tracing = "0.1"
uuid = { version = "1.0", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
```

### 基本使用

```rust
use file_operations_plugin::*;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建插件实例
    let mut plugin = FileOperationsPlugin::new();
    
    // 配置插件
    let config = PluginConfig {
        plugin_id: "file-ops".to_string(),
        parameters: HashMap::new(),
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

#### `read_file`
读取文件内容

**参数:**
- `path` (string): 文件路径

**返回:**
```json
{
  "content": "文件内容",
  "size": 1024,
  "modified": 1640995200,
  "is_file": true,
  "is_dir": false
}
```

#### `write_file`
写入文件内容

**参数:**
- `path` (string): 文件路径
- `content` (string): 文件内容

**返回:**
```json
{
  "success": true,
  "path": "example.txt",
  "size": 1024,
  "created": true
}
```

#### `delete_file`
删除文件或目录

**参数:**
- `path` (string): 文件或目录路径
- `recursive` (boolean, 可选): 是否递归删除目录，默认 false

**返回:**
```json
{
  "success": true,
  "path": "example.txt",
  "deleted": true
}
```

#### `list_directory`
列出目录内容

**参数:**
- `path` (string, 可选): 目录路径，默认为当前目录
- `show_hidden` (boolean, 可选): 是否显示隐藏文件，默认 false

**返回:**
```json
{
  "path": ".",
  "entries": [
    {
      "name": "example.txt",
      "path": "./example.txt",
      "is_file": true,
      "is_dir": false,
      "size": 1024,
      "modified": 1640995200
    }
  ],
  "count": 1
}
```

#### `create_directory`
创建目录

**参数:**
- `path` (string): 目录路径
- `recursive` (boolean, 可选): 是否递归创建父目录，默认 true

**返回:**
```json
{
  "success": true,
  "path": "new_dir",
  "created": true
}
```

#### `copy_file`
复制文件

**参数:**
- `source` (string): 源文件路径
- `destination` (string): 目标文件路径

**返回:**
```json
{
  "success": true,
  "source": "source.txt",
  "destination": "dest.txt",
  "copied": true
}
```

#### `get_file_info`
获取文件信息

**参数:**
- `path` (string): 文件或目录路径

**返回:**
```json
{
  "path": "example.txt",
  "exists": true,
  "is_file": true,
  "is_dir": false,
  "size": 1024,
  "readonly": false,
  "modified": 1640995200,
  "created": 1640995100,
  "accessed": 1640995300
}
```

## 配置选项

插件支持以下配置参数：

### `base_path`
- **类型**: string
- **描述**: 基础路径，所有文件操作将限制在此路径下
- **默认值**: null（无限制）

### `max_file_size_mb`
- **类型**: integer
- **描述**: 最大文件大小限制（MB）
- **范围**: 1-1024
- **默认值**: 100

### `allowed_extensions`
- **类型**: array of strings
- **描述**: 允许的文件扩展名列表
- **默认值**: []（无限制）

### `forbidden_paths`
- **类型**: array of strings
- **描述**: 禁止访问的路径列表
- **默认值**: []

### 配置示例

```json
{
  "base_path": "/safe/directory",
  "max_file_size_mb": 50,
  "allowed_extensions": [".txt", ".json", ".md"],
  "forbidden_paths": ["/etc", "/sys", "/proc"]
}
```

## 错误处理

插件使用统一的错误处理机制：

### 错误类型

- **ValidationError**: 参数验证错误
- **NotFoundError**: 文件或目录不存在
- **IoError**: 文件系统操作错误
- **PermissionError**: 权限不足错误

### 错误示例

```json
{
  "error": "ValidationError",
  "message": "缺少参数: path",
  "details": {}
}
```

## 安全考虑

### 路径安全
- 自动检测和阻止路径遍历攻击（`../`）
- 支持基础路径限制，防止访问系统敏感目录
- 路径规范化处理

### 权限控制
- 基于插件权限系统的访问控制
- 可配置的禁止路径列表
- 文件扩展名白名单

### 资源限制
- 文件大小限制
- 操作超时控制
- 内存使用限制

## 测试

### 运行测试

```bash
# 运行所有测试
cargo test

# 运行特定测试
cargo test test_write_and_read_file

# 运行示例
cargo run --example basic_usage
```

### 测试覆盖

- 插件生命周期测试
- 文件操作功能测试
- 错误处理测试
- 安全性测试
- 性能测试

## 示例

### 基本使用示例

```bash
cargo run --example basic_usage
```

### 高级操作示例

```bash
cargo run --example advanced_operations
```

## 开发指南

### 扩展插件

1. **添加新方法**: 在 `FileOperationsPlugin` 中实现新的操作方法
2. **更新路由**: 在 `handle_call` 方法中添加新的路由
3. **添加测试**: 为新功能编写单元测试
4. **更新文档**: 更新 API 文档和示例

### 自定义配置

1. **扩展配置结构**: 在 `config_schema` 中添加新的配置项
2. **实现验证**: 在 `validate_config` 中添加验证逻辑
3. **使用配置**: 在相关方法中使用新的配置参数

### 错误处理

1. **定义错误类型**: 扩展 `AiStudioError` 或创建自定义错误
2. **错误传播**: 使用 `?` 操作符传播错误
3. **错误转换**: 实现 `From` trait 进行错误转换

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