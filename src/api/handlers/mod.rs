// API 处理器模块
// 包含所有 API 端点的处理逻辑

pub mod agent;
pub mod auth;
pub mod document;
pub mod health;
pub mod knowledge_base;
pub mod monitoring;
pub mod plugin;
pub mod qa;
pub mod quota;
pub mod rate_limit;
pub mod tenant;
pub mod tool;
pub mod version;
pub mod workflow;

// 重新导出常用的处理器
pub use agent::*;
pub use auth::*;
pub use document::*;
pub use health::*;
pub use knowledge_base::*;
pub use monitoring::*;
pub use plugin::*;
pub use qa::*;
pub use quota::*;
pub use rate_limit::*;
pub use tenant::*;
pub use tool::*;
pub use version::*;
pub use workflow::*;