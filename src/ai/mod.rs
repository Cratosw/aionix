// AI 模块
// 包含 AI 相关功能和 Rig 框架集成

pub mod client;
pub mod models;
pub mod health;
pub mod document_processor;
pub mod chunker;
pub mod vector_search;
pub mod rig_client;

#[cfg(test)]
mod tests;

// 将在后续任务中实现的模块
// pub mod rag;
// pub mod agent;

pub use client::*;
pub use models::*;
pub use health::*;
pub use document_processor::*;
pub use chunker::*;
pub use vector_search::*;
pub use rig_client::*;