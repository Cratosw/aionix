// 计算器工具实现

use std::collections::HashMap;
use serde_json;
use tracing::{debug, error};

use crate::ai::agent_runtime::{Tool, ToolResult, ToolMetadata, ExecutionContext};
use crate::errors::AiStudioError;

/// 计算器工具
#[derive(Debug, Clone)]
pub struct CalculatorTool {
    /// 支持的操作
    supported_operations: Vec<String>,
}

impl CalculatorTool {
    /// 创建新的计算器工具
    pub fn new() -> Self {
        Self {
            supported_operations: vec![
                "add".to_string(),
                "subtract".to_string(),
                "multiply".to_string(),
                "divide".to_string(),
                "power".to_string(),
                "sqrt".to_string(),
                "abs".to_string(),
                "round".to_string(),
            ],
        }
    }
}

impl Tool for CalculatorTool {
    async fn execute(
        &self,
        parameters: HashMap<String, serde_json::Value>,
        _context: &ExecutionContext,
    ) -> Result<ToolResult, AiStudioError> {
        debug!("执行计算器工具");
        
        // 提取操作类型
        let operation = parameters.get("operation")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AiStudioError::validation("operation".to_string(), "缺少必需参数: operation".to_string()))?;
        
        if !self.supported_operations.contains(&operation.to_string()) {
            return Err(AiStudioError::validation("operation".to_string(), &format!("不支持的操作: {}", operation)));
        }
        
        debug!("计算操作: {}", operation);
        
        let start_time = std::time::Instant::now();
        
        // 执行计算
        let result = match operation {
            "add" => self.add(&parameters)?,
            "subtract" => self.subtract(&parameters)?,
            "multiply" => self.multiply(&parameters)?,
            "divide" => self.divide(&parameters)?,
            "power" => self.power(&parameters)?,
            "sqrt" => self.sqrt(&parameters)?,
            "abs" => self.abs(&parameters)?,
            "round" => self.round(&parameters)?,
            _ => return Err(AiStudioError::validation("operation".to_string(), &format!("未实现的操作: {}", operation))),
        };
        
        let execution_time = start_time.elapsed().as_millis() as u64;
        
        Ok(ToolResult {
            success: true,
            data: serde_json::json!({
                "operation": operation,
                "result": result,
                "parameters": parameters
            }),
            error: None,
            execution_time_ms: execution_time,
            message: Some(format!("计算完成: {} = {}", operation, result)),
        })
    }
    
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            name: "calculator".to_string(),
            description: "执行数学计算操作".to_string(),
            parameters_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "operation": {
                        "type": "string",
                        "description": "计算操作类型",
                        "enum": self.supported_operations
                    },
                    "a": {
                        "type": "number",
                        "description": "第一个操作数"
                    },
                    "b": {
                        "type": "number",
                        "description": "第二个操作数（某些操作需要）"
                    },
                    "precision": {
                        "type": "integer",
                        "description": "结果精度（小数位数）",
                        "minimum": 0,
                        "maximum": 10,
                        "default": 2
                    }
                },
                "required": ["operation", "a"]
            }),
            category: "math".to_string(),
            requires_permission: false,
            version: "1.0.0".to_string(),
        }
    }
    
    fn validate_parameters(
        &self,
        parameters: &HashMap<String, serde_json::Value>,
    ) -> Result<(), AiStudioError> {
        // 验证操作参数
        let operation = parameters.get("operation")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AiStudioError::validation("operation".to_string(), "缺少必需参数: operation".to_string()))?;
        
        if !self.supported_operations.contains(&operation.to_string()) {
            return Err(AiStudioError::validation("operation".to_string(), &format!("不支持的操作: {}", operation)));
        }
        
        // 验证第一个操作数
        if !parameters.contains_key("a") {
            return Err(AiStudioError::validation("缺少必需参数: a"));
        }
        
        if !parameters.get("a").unwrap().is_number() {
            return Err(AiStudioError::validation("参数 a 必须是数字"));
        }
        
        // 验证第二个操作数（如果需要）
        let requires_b = matches!(operation, "add" | "subtract" | "multiply" | "divide" | "power");
        if requires_b {
            if !parameters.contains_key("b") {
                return Err(AiStudioError::validation(&format!("操作 {} 需要参数 b", operation)));
            }
            
            if !parameters.get("b").unwrap().is_number() {
                return Err(AiStudioError::validation("参数 b 必须是数字"));
            }
            
            // 检查除零
            if operation == "divide" {
                let b = parameters.get("b").unwrap().as_f64().unwrap();
                if b == 0.0 {
                    return Err(AiStudioError::validation("除数不能为零"));
                }
            }
        }
        
        // 验证精度参数
        if let Some(precision) = parameters.get("precision") {
            if let Some(p) = precision.as_u64() {
                if p > 10 {
                    return Err(AiStudioError::validation("精度不能超过 10"));
                }
            } else {
                return Err(AiStudioError::validation("precision 必须是非负整数"));
            }
        }
        
        Ok(())
    }
}

impl CalculatorTool {
    /// 加法
    fn add(&self, parameters: &HashMap<String, serde_json::Value>) -> Result<f64, AiStudioError> {
        let a = self.get_number(parameters, "a")?;
        let b = self.get_number(parameters, "b")?;
        Ok(a + b)
    }
    
    /// 减法
    fn subtract(&self, parameters: &HashMap<String, serde_json::Value>) -> Result<f64, AiStudioError> {
        let a = self.get_number(parameters, "a")?;
        let b = self.get_number(parameters, "b")?;
        Ok(a - b)
    }
    
    /// 乘法
    fn multiply(&self, parameters: &HashMap<String, serde_json::Value>) -> Result<f64, AiStudioError> {
        let a = self.get_number(parameters, "a")?;
        let b = self.get_number(parameters, "b")?;
        Ok(a * b)
    }
    
    /// 除法
    fn divide(&self, parameters: &HashMap<String, serde_json::Value>) -> Result<f64, AiStudioError> {
        let a = self.get_number(parameters, "a")?;
        let b = self.get_number(parameters, "b")?;
        
        if b == 0.0 {
            return Err(AiStudioError::validation("除数不能为零"));
        }
        
        Ok(a / b)
    }
    
    /// 幂运算
    fn power(&self, parameters: &HashMap<String, serde_json::Value>) -> Result<f64, AiStudioError> {
        let a = self.get_number(parameters, "a")?;
        let b = self.get_number(parameters, "b")?;
        Ok(a.powf(b))
    }
    
    /// 平方根
    fn sqrt(&self, parameters: &HashMap<String, serde_json::Value>) -> Result<f64, AiStudioError> {
        let a = self.get_number(parameters, "a")?;
        
        if a < 0.0 {
            return Err(AiStudioError::validation("不能计算负数的平方根"));
        }
        
        Ok(a.sqrt())
    }
    
    /// 绝对值
    fn abs(&self, parameters: &HashMap<String, serde_json::Value>) -> Result<f64, AiStudioError> {
        let a = self.get_number(parameters, "a")?;
        Ok(a.abs())
    }
    
    /// 四舍五入
    fn round(&self, parameters: &HashMap<String, serde_json::Value>) -> Result<f64, AiStudioError> {
        let a = self.get_number(parameters, "a")?;
        let precision = parameters.get("precision")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32;
        
        let multiplier = 10_f64.powi(precision as i32);
        Ok((a * multiplier).round() / multiplier)
    }
    
    /// 获取数字参数
    fn get_number(
        &self,
        parameters: &HashMap<String, serde_json::Value>,
        key: &str,
    ) -> Result<f64, AiStudioError> {
        parameters.get(key)
            .and_then(|v| v.as_f64())
            .ok_or_else(|| AiStudioError::validation(&format!("无效的数字参数: {}", key)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_calculator_add() {
        let tool = CalculatorTool::new();
        let mut parameters = HashMap::new();
        parameters.insert("operation".to_string(), serde_json::Value::String("add".to_string()));
        parameters.insert("a".to_string(), serde_json::Value::Number(serde_json::Number::from(5)));
        parameters.insert("b".to_string(), serde_json::Value::Number(serde_json::Number::from(3)));
        
        let context = ExecutionContext {
            current_task: None,
            execution_history: Vec::new(),
            context_variables: HashMap::new(),
            session_id: None,
            user_id: None,
        };
        
        let result = tool.execute(parameters, &context).await.unwrap();
        assert!(result.success);
        assert_eq!(result.data.get("result").unwrap().as_f64().unwrap(), 8.0);
    }
    
    #[tokio::test]
    async fn test_calculator_divide_by_zero() {
        let tool = CalculatorTool::new();
        let mut parameters = HashMap::new();
        parameters.insert("operation".to_string(), serde_json::Value::String("divide".to_string()));
        parameters.insert("a".to_string(), serde_json::Value::Number(serde_json::Number::from(5)));
        parameters.insert("b".to_string(), serde_json::Value::Number(serde_json::Number::from(0)));
        
        let context = ExecutionContext {
            current_task: None,
            execution_history: Vec::new(),
            context_variables: HashMap::new(),
            session_id: None,
            user_id: None,
        };
        
        let result = tool.execute(parameters, &context).await;
        assert!(result.is_err());
    }
    
    #[test]
    fn test_calculator_validation() {
        let tool = CalculatorTool::new();
        
        // 测试有效参数
        let mut valid_params = HashMap::new();
        valid_params.insert("operation".to_string(), serde_json::Value::String("add".to_string()));
        valid_params.insert("a".to_string(), serde_json::Value::Number(serde_json::Number::from(5)));
        valid_params.insert("b".to_string(), serde_json::Value::Number(serde_json::Number::from(3)));
        assert!(tool.validate_parameters(&valid_params).is_ok());
        
        // 测试缺少操作参数
        let mut invalid_params = HashMap::new();
        invalid_params.insert("a".to_string(), serde_json::Value::Number(serde_json::Number::from(5)));
        assert!(tool.validate_parameters(&invalid_params).is_err());
    }
}