// 插件测试示例
// 演示如何使用测试框架测试插件

use std::collections::HashMap;
use std::time::Duration;
use serde_json::json;

// 导入测试框架
mod test_framework;
use test_framework::*;

// 示例插件实现
struct ExamplePlugin {
    status: PluginStatus,
    config: Option<PluginConfig>,
}

impl ExamplePlugin {
    fn new() -> Self {
        Self {
            status: PluginStatus::Uninitialized,
            config: None,
        }
    }
}

#[async_trait::async_trait]
impl Plugin for ExamplePlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: "example-plugin".to_string(),
            name: "示例插件".to_string(),
            version: "1.0.0".to_string(),
            description: "用于测试的示例插件".to_string(),
            author: "Test Author".to_string(),
            license: "MIT".to_string(),
            homepage: None,
            repository: None,
            plugin_type: PluginType::Tool,
            api_version: "1.0".to_string(),
            min_system_version: "1.0.0".to_string(),
            dependencies: Vec::new(),
            permissions: vec![PluginPermission::FileSystem],
            tags: vec!["test".to_string(), "example".to_string()],
            icon: Some("🧪".to_string()),
            created_at: chrono::Utc::now(),
        }
    }
    
    async fn initialize(&mut self, config: PluginConfig) -> Result<(), AiStudioError> {
        self.config = Some(config);
        self.status = PluginStatus::Initialized;
        Ok(())
    }
    
    async fn start(&mut self) -> Result<(), AiStudioError> {
        self.status = PluginStatus::Running;
        Ok(())
    }
    
    async fn stop(&mut self) -> Result<(), AiStudioError> {
        self.status = PluginStatus::Stopped;
        Ok(())
    }
    
    async fn shutdown(&mut self) -> Result<(), AiStudioError> {
        self.config = None;
        self.status = PluginStatus::Unloaded;
        Ok(())
    }
    
    fn status(&self) -> PluginStatus {
        self.status.clone()
    }
    
    async fn handle_call(
        &self,
        method: &str,
        params: HashMap<String, serde_json::Value>,
        _context: &PluginContext,
    ) -> Result<serde_json::Value, AiStudioError> {
        match method {
            "echo" => {
                let message = params.get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Hello, World!");
                
                Ok(json!({
                    "echo": message,
                    "timestamp": chrono::Utc::now()
                }))
            },
            "add" => {
                let a = params.get("a")
                    .and_then(|v| v.as_f64())
                    .ok_or_else(|| AiStudioError::validation("缺少参数 a"))?;
                
                let b = params.get("b")
                    .and_then(|v| v.as_f64())
                    .ok_or_else(|| AiStudioError::validation("缺少参数 b"))?;
                
                Ok(json!({
                    "result": a + b,
                    "operation": "add"
                }))
            },
            "slow_operation" => {
                // 模拟慢操作
                tokio::time::sleep(Duration::from_millis(100)).await;
                Ok(json!({
                    "message": "操作完成",
                    "duration_ms": 100
                }))
            },
            "error_operation" => {
                Err(AiStudioError::test_error("这是一个测试错误"))
            },
            _ => Err(AiStudioError::validation(&format!("未知方法: {}", method)))
        }
    }
    
    async fn health_check(&self) -> Result<PluginHealth, AiStudioError> {
        let start_time = std::time::Instant::now();
        
        let healthy = self.status == PluginStatus::Running;
        let mut details = HashMap::new();
        details.insert("status".to_string(), json!(self.status));
        
        let response_time = start_time.elapsed().as_millis() as u64;
        
        Ok(PluginHealth {
            healthy,
            message: if healthy {
                "插件运行正常".to_string()
            } else {
                format!("插件状态异常: {:?}", self.status)
            },
            details,
            checked_at: chrono::Utc::now(),
            response_time_ms: response_time,
        })
    }
    
    fn config_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "timeout_seconds": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": 300,
                    "default": 30
                },
                "debug_mode": {
                    "type": "boolean",
                    "default": false
                }
            }
        })
    }
    
    fn validate_config(&self, config: &PluginConfig) -> Result<(), AiStudioError> {
        if let Some(timeout) = config.parameters.get("timeout_seconds") {
            if let Some(timeout_val) = timeout.as_u64() {
                if timeout_val == 0 || timeout_val > 300 {
                    return Err(AiStudioError::validation("timeout_seconds 必须在 1-300 之间"));
                }
            }
        }
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== 插件测试框架示例 ===\n");
    
    let mut framework = PluginTestFramework::new();
    
    // 1. 生命周期测试
    println!("1. 运行生命周期测试...");
    let plugin = ExamplePlugin::new();
    let config = PluginConfig {
        plugin_id: "test".to_string(),
        parameters: HashMap::new(),
        environment: HashMap::new(),
        resource_limits: Default::default(),
        security_settings: Default::default(),
    };
    
    let lifecycle_suite = framework.test_plugin_lifecycle(plugin, config.clone()).await;
    framework.add_suite(lifecycle_suite);
    
    // 2. 方法调用测试
    println!("2. 运行方法调用测试...");
    let mut plugin = ExamplePlugin::new();
    plugin.initialize(config.clone()).await?;
    plugin.start().await?;
    
    let method_tests = vec![
        MethodTestCase {
            name: "Echo Test".to_string(),
            method: "echo".to_string(),
            params: {
                let mut params = HashMap::new();
                params.insert("message".to_string(), json!("Hello, Test!"));
                params
            },
            expect_error: false,
            validator: Some(Box::new(|response| {
                if let Some(echo) = response.get("echo") {
                    if echo.as_str() == Some("Hello, Test!") {
                        Ok(())
                    } else {
                        Err("Echo message mismatch".to_string())
                    }
                } else {
                    Err("Missing echo field".to_string())
                }
            })),
        },
        MethodTestCase {
            name: "Add Test".to_string(),
            method: "add".to_string(),
            params: {
                let mut params = HashMap::new();
                params.insert("a".to_string(), json!(5));
                params.insert("b".to_string(), json!(3));
                params
            },
            expect_error: false,
            validator: Some(Box::new(|response| {
                if let Some(result) = response.get("result") {
                    if result.as_f64() == Some(8.0) {
                        Ok(())
                    } else {
                        Err(format!("Expected 8, got {:?}", result))
                    }
                } else {
                    Err("Missing result field".to_string())
                }
            })),
        },
        MethodTestCase {
            name: "Error Test".to_string(),
            method: "error_operation".to_string(),
            params: HashMap::new(),
            expect_error: true,
            validator: None,
        },
        MethodTestCase {
            name: "Invalid Method Test".to_string(),
            method: "invalid_method".to_string(),
            params: HashMap::new(),
            expect_error: true,
            validator: None,
        },
    ];
    
    let method_suite = framework.test_plugin_methods(&plugin, method_tests).await;
    framework.add_suite(method_suite);
    
    // 3. 配置测试
    println!("3. 运行配置测试...");
    let config_tests = vec![
        ConfigTestCase {
            name: "Valid Config".to_string(),
            config: PluginConfig {
                plugin_id: "test".to_string(),
                parameters: {
                    let mut params = HashMap::new();
                    params.insert("timeout_seconds".to_string(), json!(30));
                    params.insert("debug_mode".to_string(), json!(true));
                    params
                },
                environment: HashMap::new(),
                resource_limits: Default::default(),
                security_settings: Default::default(),
            },
            expect_valid: true,
        },
        ConfigTestCase {
            name: "Invalid Timeout".to_string(),
            config: PluginConfig {
                plugin_id: "test".to_string(),
                parameters: {
                    let mut params = HashMap::new();
                    params.insert("timeout_seconds".to_string(), json!(0)); // 无效值
                    params
                },
                environment: HashMap::new(),
                resource_limits: Default::default(),
                security_settings: Default::default(),
            },
            expect_valid: false,
        },
        ConfigTestCase {
            name: "Empty Config".to_string(),
            config: PluginConfig {
                plugin_id: "test".to_string(),
                parameters: HashMap::new(),
                environment: HashMap::new(),
                resource_limits: Default::default(),
                security_settings: Default::default(),
            },
            expect_valid: true,
        },
    ];
    
    let config_suite = framework.test_plugin_config(&plugin, config_tests).await;
    framework.add_suite(config_suite);
    
    // 4. 性能测试
    println!("4. 运行性能测试...");
    let perf_tests = vec![
        PerformanceTestCase {
            name: "Echo Performance".to_string(),
            method: "echo".to_string(),
            params: {
                let mut params = HashMap::new();
                params.insert("message".to_string(), json!("Performance Test"));
                params
            },
            iterations: 100,
            max_avg_duration: Duration::from_millis(10),
        },
        PerformanceTestCase {
            name: "Add Performance".to_string(),
            method: "add".to_string(),
            params: {
                let mut params = HashMap::new();
                params.insert("a".to_string(), json!(1));
                params.insert("b".to_string(), json!(2));
                params
            },
            iterations: 1000,
            max_avg_duration: Duration::from_millis(5),
        },
        PerformanceTestCase {
            name: "Slow Operation Performance".to_string(),
            method: "slow_operation".to_string(),
            params: HashMap::new(),
            iterations: 10,
            max_avg_duration: Duration::from_millis(150), // 允许一些开销
        },
    ];
    
    let perf_suite = framework.test_plugin_performance(&plugin, perf_tests).await;
    framework.add_suite(perf_suite);
    
    // 清理
    plugin.stop().await?;
    plugin.shutdown().await?;
    
    // 5. 生成和显示报告
    println!("5. 生成测试报告...\n");
    let report = framework.generate_report();
    report.print();
    
    // 6. 检查测试结果
    if report.all_passed() {
        println!("🎉 所有测试都通过了！");
        std::process::exit(0);
    } else {
        println!("❌ 有测试失败，请检查上面的报告。");
        std::process::exit(1);
    }
}