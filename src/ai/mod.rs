// AI 模块
// 包含 AI 相关功能和 Rig 框架集成

pub mod client;
pub mod models;
pub mod health;
pub mod document_processor;
pub mod chunker;
pub mod vector_search;
pub mod rig_client;
pub mod rag_engine;
pub mod quality_assessment;
pub mod answer_cache;
pub mod agent_runtime;
pub mod tools;

#[cfg(test)]
mod tests;

pub use client::*;
pub use models::*;
pub use health::*;
pub use document_processor::*;
pub use chunker::*;
pub use vector_search::*;
pub use rig_client::*;
pub use rag_engine::*;
pub use quality_assessment::*;
pub use answer_cache::*;
pub use agent_runtime::*;
pub use tools::*;