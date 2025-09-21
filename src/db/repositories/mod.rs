// 数据库仓储模块
// 提供数据访问层的抽象

pub mod tenant;
pub mod user;
pub mod session;

// 知识库相关仓储
pub mod knowledge_base;
pub mod document;
pub mod document_chunk;
pub mod embedding;

// Agent 相关仓储
pub mod agent;
pub mod workflow;

pub use tenant::TenantRepository;
pub use user::UserRepository;
pub use session::SessionRepository;

// 知识库相关仓储导出
pub use knowledge_base::KnowledgeBaseRepository;
pub use document::DocumentRepository;
pub use document_chunk::DocumentChunkRepository;
pub use embedding::EmbeddingRepository;

// Agent 相关仓储导出
pub use agent::AgentRepository;
pub use workflow::WorkflowRepository;