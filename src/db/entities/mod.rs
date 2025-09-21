// 数据库实体模块
// 包含所有 SeaORM 实体定义

pub mod tenant;
pub mod user;
pub mod session;

// 知识库相关实体
pub mod knowledge_base;
pub mod document;
pub mod document_chunk;
pub mod embedding;

// Agent 相关实体（将在后续任务中实现）
// pub mod agent;
// pub mod workflow;

pub mod prelude;
pub use prelude::*;