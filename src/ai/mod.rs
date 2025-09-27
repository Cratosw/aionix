// AI 模块
// 包含 AI 相关功能和 Rig 框架集成

pub mod client;
pub mod models;
pub mod health;

#[cfg(test)]
mod tests;

// 将在后续任务中实现的模块
// pub mod rag;
// pub mod agent;
// pub mod document_processor;
// pub mod vector_search;

pub use client::*;
pub use models::*;
pub use health::*;