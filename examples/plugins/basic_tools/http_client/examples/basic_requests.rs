// HTTP 客户端插件基本请求示例

use std::collections::HashMap;
use uuid::Uuid;
use chrono::Utc;
use serde_json::json;
use http_client_plugin::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::init();
    
    println!("=== HTTP 客户端插件基本请求示例 ===\n");
    
    // 创建插件实例
    let mut plugin = HttpClientPlugin::new();
    println!("1. 创建插件实例: {}", plugin.metadata().name);
    
    // 配置插件
    let mut config_params = HashMap::new();
    config_params.insert("default_timeout_seconds".to_string(), json!(30));
    config_params.insert("max_response_size_mb".to_string(), json!(10));
    config_params.insert("verify_ssl".to_string(), json!(true));
    
    let config = PluginConfig {
        plugin_id: "http-client-demo".to_string(),
        parameters: config_params,
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
    
    // 示例 1: GET 请求
    println!("\n3. GET 请求示例:");
    let mut get_params = HashMap::new();
    get_params.insert("url".to_string(), json!("https://httpbin.org/get"));
    get_params.insert("headers".to_string(), json!({
        "User-Agent": "Aionix-AI-Studio-Demo/1.0",
        "Accept": "application/json"
    }));
    
    match plugin.handle_call("get", get_params, &context).await {
        Ok(response) => {
            println!("   GET 请求成功:");
            println!("   状态码: {}", response["status"]);
            println!("   响应时间: {} ms", response["response_time_ms"]);
            println!("   内容类型: {}", response["content_type"].as_str().unwrap_or("未知"));
            
            // 解析响应体（如果是 JSON）
            if let Ok(body_json) = serde_json::from_str::<serde_json::Value>(
                response["body"].as_str().unwrap_or("")
            ) {
                if let Some(headers) = body_json.get("headers") {
                    println!("   服务器收到的请求头: {}", headers);
                }
            }
        },
        Err(e) => println!("   GET 请求失败: {}", e),
    }
    
    // 示例 2: POST 请求（JSON 数据）
    println!("\n4. POST 请求示例（JSON 数据）:");
    let post_data = json!({
        "name": "Aionix AI Studio",
        "version": "1.0.0",
        "features": ["plugins", "ai", "automation"]
    });
    
    let mut post_params = HashMap::new();
    post_params.insert("url".to_string(), json!("https://httpbin.org/post"));
    post_params.insert("headers".to_string(), json!({
        "Content-Type": "application/json",
        "Accept": "application/json"
    }));
    post_params.insert("body".to_string(), json!(post_data.to_string()));
    
    match plugin.handle_call("post", post_params, &context).await {
        Ok(response) => {
            println!("   POST 请求成功:");
            println!("   状态码: {}", response["status"]);
            println!("   响应时间: {} ms", response["response_time_ms"]);
            
            // 解析响应体
            if let Ok(body_json) = serde_json::from_str::<serde_json::Value>(
                response["body"].as_str().unwrap_or("")
            ) {
                if let Some(json_data) = body_json.get("json") {
                    println!("   服务器收到的 JSON 数据: {}", json_data);
                }
            }
        },
        Err(e) => println!("   POST 请求失败: {}", e),
    }
    
    // 示例 3: PUT 请求
    println!("\n5. PUT 请求示例:");
    let put_data = json!({
        "id": 123,
        "title": "Updated Title",
        "body": "Updated content",
        "userId": 1
    });
    
    let mut put_params = HashMap::new();
    put_params.insert("url".to_string(), json!("https://httpbin.org/put"));
    put_params.insert("headers".to_string(), json!({
        "Content-Type": "application/json"
    }));
    put_params.insert("body".to_string(), json!(put_data.to_string()));
    
    match plugin.handle_call("put", put_params, &context).await {
        Ok(response) => {
            println!("   PUT 请求成功:");
            println!("   状态码: {}", response["status"]);
            println!("   响应时间: {} ms", response["response_time_ms"]);
        },
        Err(e) => println!("   PUT 请求失败: {}", e),
    }
    
    // 示例 4: DELETE 请求
    println!("\n6. DELETE 请求示例:");
    let mut delete_params = HashMap::new();
    delete_params.insert("url".to_string(), json!("https://httpbin.org/delete"));
    delete_params.insert("headers".to_string(), json!({
        "Authorization": "Bearer demo-token"
    }));
    
    match plugin.handle_call("delete", delete_params, &context).await {
        Ok(response) => {
            println!("   DELETE 请求成功:");
            println!("   状态码: {}", response["status"]);
            println!("   响应时间: {} ms", response["response_time_ms"]);
        },
        Err(e) => println!("   DELETE 请求失败: {}", e),
    }
    
    // 示例 5: 带查询参数的 GET 请求
    println!("\n7. 带查询参数的 GET 请求示例:");
    let mut query_params = HashMap::new();
    query_params.insert("url".to_string(), json!("https://httpbin.org/get?param1=value1&param2=value2"));
    
    match plugin.handle_call("get", query_params, &context).await {
        Ok(response) => {
            println!("   查询参数请求成功:");
            println!("   状态码: {}", response["status"]);
            
            // 解析响应体查看查询参数
            if let Ok(body_json) = serde_json::from_str::<serde_json::Value>(
                response["body"].as_str().unwrap_or("")
            ) {
                if let Some(args) = body_json.get("args") {
                    println!("   服务器收到的查询参数: {}", args);
                }
            }
        },
        Err(e) => println!("   查询参数请求失败: {}", e),
    }
    
    // 示例 6: 自定义超时的请求
    println!("\n8. 自定义超时的请求示例:");
    let mut timeout_params = HashMap::new();
    timeout_params.insert("url".to_string(), json!("https://httpbin.org/delay/2"));
    timeout_params.insert("timeout".to_string(), json!(5)); // 5 秒超时
    
    match plugin.handle_call("get", timeout_params, &context).await {
        Ok(response) => {
            println!("   超时请求成功:");
            println!("   状态码: {}", response["status"]);
            println!("   响应时间: {} ms", response["response_time_ms"]);
        },
        Err(e) => println!("   超时请求失败: {}", e),
    }
    
    // 示例 7: 错误处理（无效 URL）
    println!("\n9. 错误处理示例（无效 URL）:");
    let mut error_params = HashMap::new();
    error_params.insert("url".to_string(), json!("invalid-url"));
    
    match plugin.handle_call("get", error_params, &context).await {
        Ok(_) => println!("   意外成功（不应该发生）"),
        Err(e) => println!("   预期的错误: {}", e),
    }
    
    // 示例 8: 健康检查
    println!("\n10. 健康检查示例:");
    match plugin.health_check().await {
        Ok(health) => {
            println!("   健康状态: {}", if health.healthy { "正常" } else { "异常" });
            println!("   消息: {}", health.message);
            println!("   响应时间: {} ms", health.response_time_ms);
            println!("   详细信息:");
            for (key, value) in &health.details {
                println!("     {}: {}", key, value);
            }
        },
        Err(e) => println!("   健康检查失败: {}", e),
    }
    
    // 示例 9: 获取配置模式
    println!("\n11. 配置模式示例:");
    let schema = plugin.config_schema();
    println!("   配置模式: {}", serde_json::to_string_pretty(&schema)?);
    
    // 停止插件
    plugin.stop().await?;
    plugin.shutdown().await?;
    println!("\n12. 插件已关闭");
    
    println!("\n=== 示例完成 ===");
    
    Ok(())
}