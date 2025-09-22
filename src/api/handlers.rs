// API 处理器
// 定义基础的 API 处理器函数

pub mod health;
pub mod version;
pub mod tenant;

pub use health::*;
pub use version::*;
pub use tenant::*;