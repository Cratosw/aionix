// Agent 工具模块
// 实现基础工具和工具接口

pub mod search_tool;
pub mod calculator_tool;
pub mod file_tool;
pub mod http_tool;

pub use search_tool::*;
pub use calculator_tool::*;
pub use file_tool::*;
pub use http_tool::*;

use std::collections::HashMap;
use serde_json;
use crate::ai::agent_runtime::{Tool, ToolResult, ToolMetadata, ExecutionContext, ToolEnum};
use crate::errors::AiStudioError;

/// 工具工厂
pub struct ToolFactory;

impl ToolFactory {
    /// 创建所有基础工具
    pub fn create_basic_tools() -> Vec<ToolEnum> {
        vec![
            ToolEnum::SearchTool(SearchTool::new()),
            ToolEnum::CalculatorTool(CalculatorTool::new()),
            ToolEnum::FileTool(FileTool::new()),
            ToolEnum::HttpTool(HttpTool::new()),
        ]
    }
    
    /// 根据名称创建工具
    pub fn create_tool(tool_name: &str) -> Option<ToolEnum> {
        match tool_name {
            "search" => Some(ToolEnum::SearchTool(SearchTool::new())),
            "calculator" => Some(ToolEnum::CalculatorTool(CalculatorTool::new())),
            "file" => Some(ToolEnum::FileTool(FileTool::new())),
            "http" => Some(ToolEnum::HttpTool(HttpTool::new())),
            _ => None,
        }
    }
}