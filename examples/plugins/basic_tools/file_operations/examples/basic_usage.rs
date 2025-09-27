// 文件操作插件基本使用示例

use std::collections::HashMap;
use uuid::Uuid;
use chrono::Utc;
use file_operations_plugin::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::init();
    
    println!("=== 文件操作插件基本使用示例 ===\n");
    
    // 创建插件实例
    let mut plugin = FileOperationsPlugin::new();
    println!("1. 创建插件实例: {:?}", plugin.metadata().name);
    
    // 配置插件
    let config = PluginConfig {
        plugin_id: "file-operations-demo".to_string(),
        parameters: HashMap::new(),
        environment: HashMap::new(),
        resource_limits: Default::default(),
        security_settings: Default::default(),
    };
    
    // 初始化和启动插件
    plugin.initialize(config).await?;
    plugin.start().await?;
    println!("2. 插件状态: {:?}", plugin.status());
    
    // 创建插件上下文
    let context = PluginContext {
        tenant_id: Uuid::new_v4(),
        user_id: Some(Uuid::new_v4()),
        session_id: None,
        request_id: Uuid::new_v4(),
        variables: HashMap::new(),
        timestamp: Utc::now(),
    };
    
    // 示例 1: 创建目录
    println!("\n3. 创建目录示例:");
    let mut create_dir_params = HashMap::new();
    create_dir_params.insert("path".to_string(), serde_json::Value::String("demo_dir".to_string()));
    
    match plugin.handle_call("create_directory", create_dir_params, &context).await {
        Ok(result) => println!("   创建目录成功: {}", result),
        Err(e) => println!("   创建目录失败: {}", e),
    }
    
    // 示例 2: 写入文件
    println!("\n4. 写入文件示例:");
    let mut write_params = HashMap::new();
    write_params.insert("path".to_string(), serde_json::Value::String("demo_dir/hello.txt".to_string()));
    write_params.insert("content".to_string(), serde_json::Value::String("Hello, Aionix AI Studio!".to_string()));
    
    match plugin.handle_call("write_file", write_params, &context).await {
        Ok(result) => println!("   写入文件成功: {}", result),
        Err(e) => println!("   写入文件失败: {}", e),
    }
    
    // 示例 3: 读取文件
    println!("\n5. 读取文件示例:");
    let mut read_params = HashMap::new();
    read_params.insert("path".to_string(), serde_json::Value::String("demo_dir/hello.txt".to_string()));
    
    match plugin.handle_call("read_file", read_params, &context).await {
        Ok(result) => {
            println!("   读取文件成功:");
            println!("   内容: {}", result["content"]);
            println!("   大小: {} 字节", result["size"]);
        },
        Err(e) => println!("   读取文件失败: {}", e),
    }
    
    // 示例 4: 获取文件信息
    println!("\n6. 获取文件信息示例:");
    let mut info_params = HashMap::new();
    info_params.insert("path".to_string(), serde_json::Value::String("demo_dir/hello.txt".to_string()));
    
    match plugin.handle_call("get_file_info", info_params, &context).await {
        Ok(result) => {
            println!("   文件信息:");
            println!("   路径: {}", result["path"]);
            println!("   是文件: {}", result["is_file"]);
            println!("   是目录: {}", result["is_dir"]);
            println!("   大小: {} 字节", result["size"]);
            println!("   只读: {}", result["readonly"]);
        },
        Err(e) => println!("   获取文件信息失败: {}", e),
    }
    
    // 示例 5: 列出目录内容
    println!("\n7. 列出目录内容示例:");
    let mut list_params = HashMap::new();
    list_params.insert("path".to_string(), serde_json::Value::String("demo_dir".to_string()));
    
    match plugin.handle_call("list_directory", list_params, &context).await {
        Ok(result) => {
            println!("   目录内容:");
            if let Some(entries) = result["entries"].as_array() {
                for entry in entries {
                    println!("   - {} ({})", 
                        entry["name"], 
                        if entry["is_dir"].as_bool().unwrap_or(false) { "目录" } else { "文件" }
                    );
                }
            }
            println!("   总计: {} 项", result["count"]);
        },
        Err(e) => println!("   列出目录失败: {}", e),
    }
    
    // 示例 6: 复制文件
    println!("\n8. 复制文件示例:");
    let mut copy_params = HashMap::new();
    copy_params.insert("source".to_string(), serde_json::Value::String("demo_dir/hello.txt".to_string()));
    copy_params.insert("destination".to_string(), serde_json::Value::String("demo_dir/hello_copy.txt".to_string()));
    
    match plugin.handle_call("copy_file", copy_params, &context).await {
        Ok(result) => println!("   复制文件成功: {}", result),
        Err(e) => println!("   复制文件失败: {}", e),
    }
    
    // 示例 7: 健康检查
    println!("\n9. 健康检查示例:");
    match plugin.health_check().await {
        Ok(health) => {
            println!("   健康状态: {}", if health.healthy { "正常" } else { "异常" });
            println!("   消息: {}", health.message);
            println!("   响应时间: {} ms", health.response_time_ms);
        },
        Err(e) => println!("   健康检查失败: {}", e),
    }
    
    // 清理: 删除演示文件和目录
    println!("\n10. 清理演示文件:");
    
    // 删除文件
    let mut delete_file_params = HashMap::new();
    delete_file_params.insert("path".to_string(), serde_json::Value::String("demo_dir/hello.txt".to_string()));
    let _ = plugin.handle_call("delete_file", delete_file_params, &context).await;
    
    let mut delete_copy_params = HashMap::new();
    delete_copy_params.insert("path".to_string(), serde_json::Value::String("demo_dir/hello_copy.txt".to_string()));
    let _ = plugin.handle_call("delete_file", delete_copy_params, &context).await;
    
    // 删除目录
    let mut delete_dir_params = HashMap::new();
    delete_dir_params.insert("path".to_string(), serde_json::Value::String("demo_dir".to_string()));
    delete_dir_params.insert("recursive".to_string(), serde_json::Value::Bool(true));
    
    match plugin.handle_call("delete_file", delete_dir_params, &context).await {
        Ok(_) => println!("   清理完成"),
        Err(e) => println!("   清理失败: {}", e),
    }
    
    // 停止插件
    plugin.stop().await?;
    plugin.shutdown().await?;
    println!("\n11. 插件已关闭");
    
    println!("\n=== 示例完成 ===");
    
    Ok(())
}