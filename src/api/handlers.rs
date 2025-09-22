// API 处理器
// 定义基础的 API 处理器函数

pub mod auth;
pub mod health;
pub mod tenant;
pub mod version;

pub use auth::*;
pub use health::*;
pub use tenant::*;
pub use version::*;