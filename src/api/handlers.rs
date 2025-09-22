// API 处理器
// 定义基础的 API 处理器函数

pub mod auth;
pub mod health;
pub mod monitoring;
pub mod quota;
pub mod rate_limit;
pub mod tenant;
pub mod version;

pub use auth::*;
pub use health::*;
pub use monitoring::*;
pub use quota::*;
pub use rate_limit::*;
pub use tenant::*;
pub use version::*;