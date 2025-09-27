// 插件系统模块
// 实现插件架构、接口规范和生命周期管理

pub mod plugin_manager;
pub mod plugin_interface;
pub mod plugin_loader;
pub mod plugin_registry;
pub mod lifecycle;

pub use plugin_manager::*;
pub use plugin_interface::*;
pub use plugin_loader::*;
pub use plugin_registry::*;
pub use lifecycle::*;