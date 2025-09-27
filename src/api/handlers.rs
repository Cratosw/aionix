// API 处理器
// 定义基础的 API 处理器函数

pub mod agent;
pub mod auth;
pub mod document;
pub mod health;
pub mod knowledge_base;
pub mod monitoring;
pub mod qa;
pub mod quota;
pub mod rate_limit;
pub mod tenant;
pub mod tool;
pub mod version;

pub use agent::*;
pub use auth::*;
pub use document::*;
pub use health::*;
pub use knowledge_base::*;
pub use monitoring::*;
pub use qa::*;
pub use quota::*;
pub use rate_limit::*;
pub use tenant::*;
pub use tool::*;
pub use version::*;