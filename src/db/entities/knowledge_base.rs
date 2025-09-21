// 知识库实体定义

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 知识库状态枚举
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "knowledge_base_status")]
pub enum KnowledgeBaseStatus {
    #[sea_orm(string_value = "active")]
    Active,
    #[sea_orm(string_value = "inactive")]
    Inactive,
    #[sea_orm(string_value = "processing")]
    Processing,
    #[sea_orm(string_value = "error")]
    Error,
}

/// 知识库类型枚举
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "knowledge_base_type")]
pub enum KnowledgeBaseType {
    #[sea_orm(string_value = "general")]
    General,
    #[sea_orm(string_value = "faq")]
    Faq,
    #[sea_orm(string_value = "documentation")]
    Documentation,
    #[sea_orm(string_value = "policy")]
    Policy,
    #[sea_orm(string_value = "product")]
    Product,
}

/// 知识库实体
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "knowledge_bases")]
pub struct Model {
    /// 知识库 ID
    #[sea_orm(primary_key)]
    pub id: Uuid,
    
    /// 租户 ID
    pub tenant_id: Uuid,
    
    /// 知识库名称
    #[sea_orm(column_type = "String(Some(255))")]
    pub name: String,
    
    /// 知识库描述
    #[sea_orm(column_type = "Text", nullable)]
    pub description: Option<String>,
    
    /// 知识库类型
    pub kb_type: KnowledgeBaseType,
    
    /// 知识库状态
    pub status: KnowledgeBaseStatus,
    
    /// 知识库配置（JSON 格式）
    #[sea_orm(column_type = "Json")]
    pub config: Json,
    
    /// 知识库元数据（JSON 格式）
    #[sea_orm(column_type = "Json")]
    pub metadata: Json,
    
    /// 文档数量
    pub document_count: i32,
    
    /// 总文档块数量
    pub chunk_count: i32,
    
    /// 总存储大小（字节）
    pub total_size_bytes: i64,
    
    /// 向量维度
    pub vector_dimension: i32,
    
    /// 嵌入模型名称
    #[sea_orm(column_type = "String(Some(255))")]
    pub embedding_model: String,
    
    /// 最后索引时间
    #[sea_orm(nullable)]
    pub last_indexed_at: Option<DateTimeWithTimeZone>,
    
    /// 创建时间
    pub created_at: DateTimeWithTimeZone,
    
    /// 更新时间
    pub updated_at: DateTimeWithTimeZone,
}

/// 知识库关联关系
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 多对一：知识库 -> 租户
    #[sea_orm(
        belongs_to = "super::tenant::Entity",
        from = "Column::TenantId",
        to = "super::tenant::Column::Id"
    )]
    Tenant,
    
    /// 一对多：知识库 -> 文档
    #[sea_orm(has_many = "super::document::Entity")]
    Documents,
}

/// 实现与租户的关联
impl Related<super::tenant::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Tenant.def()
    }
}

/// 实现与文档的关联
impl Related<super::document::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Documents.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

/// 知识库配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeBaseConfig {
    /// 分块策略
    pub chunking_strategy: ChunkingStrategy,
    /// 向量化设置
    pub vectorization_settings: VectorizationSettings,
    /// 检索设置
    pub retrieval_settings: RetrievalSettings,
    /// 访问控制
    pub access_control: AccessControl,
    /// 自定义设置
    pub custom_settings: serde_json::Value,
}

/// 分块策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkingStrategy {
    /// 分块方法
    pub method: String, // "fixed_size", "semantic", "sentence", "paragraph"
    /// 块大小（字符数）
    pub chunk_size: u32,
    /// 重叠大小（字符数）
    pub overlap_size: u32,
    /// 最小块大小
    pub min_chunk_size: u32,
    /// 最大块大小
    pub max_chunk_size: u32,
    /// 分隔符
    pub separators: Vec<String>,
}

/// 向量化设置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorizationSettings {
    /// 嵌入模型
    pub model_name: String,
    /// 向量维度
    pub dimension: u32,
    /// 批处理大小
    pub batch_size: u32,
    /// 最大重试次数
    pub max_retries: u32,
    /// 超时时间（秒）
    pub timeout_seconds: u32,
}

/// 检索设置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalSettings {
    /// 默认检索数量
    pub default_top_k: u32,
    /// 最大检索数量
    pub max_top_k: u32,
    /// 相似度阈值
    pub similarity_threshold: f32,
    /// 检索方法
    pub retrieval_method: String, // "cosine", "euclidean", "dot_product"
    /// 是否启用重排序
    pub enable_reranking: bool,
    /// 重排序模型
    pub reranking_model: Option<String>,
}

/// 访问控制
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessControl {
    /// 是否公开
    pub is_public: bool,
    /// 允许的用户角色
    pub allowed_roles: Vec<String>,
    /// 允许的用户 ID
    pub allowed_users: Vec<String>,
    /// 访问权限
    pub permissions: Vec<String>, // "read", "write", "admin"
}

/// 知识库元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeBaseMetadata {
    /// 标签
    pub tags: Vec<String>,
    /// 分类
    pub category: Option<String>,
    /// 语言
    pub language: String,
    /// 版本
    pub version: String,
    /// 作者
    pub author: Option<String>,
    /// 来源
    pub source: Option<String>,
    /// 自定义字段
    pub custom_fields: std::collections::HashMap<String, serde_json::Value>,
}

impl Default for KnowledgeBaseConfig {
    fn default() -> Self {
        Self {
            chunking_strategy: ChunkingStrategy::default(),
            vectorization_settings: VectorizationSettings::default(),
            retrieval_settings: RetrievalSettings::default(),
            access_control: AccessControl::default(),
            custom_settings: serde_json::Value::Object(serde_json::Map::new()),
        }
    }
}

impl Default for ChunkingStrategy {
    fn default() -> Self {
        Self {
            method: "fixed_size".to_string(),
            chunk_size: 1000,
            overlap_size: 200,
            min_chunk_size: 100,
            max_chunk_size: 2000,
            separators: vec!["\n\n".to_string(), "\n".to_string(), " ".to_string()],
        }
    }
}

impl Default for VectorizationSettings {
    fn default() -> Self {
        Self {
            model_name: "text-embedding-ada-002".to_string(),
            dimension: 1536,
            batch_size: 100,
            max_retries: 3,
            timeout_seconds: 30,
        }
    }
}

impl Default for RetrievalSettings {
    fn default() -> Self {
        Self {
            default_top_k: 5,
            max_top_k: 20,
            similarity_threshold: 0.7,
            retrieval_method: "cosine".to_string(),
            enable_reranking: false,
            reranking_model: None,
        }
    }
}

impl Default for AccessControl {
    fn default() -> Self {
        Self {
            is_public: false,
            allowed_roles: vec!["admin".to_string(), "manager".to_string(), "user".to_string()],
            allowed_users: Vec::new(),
            permissions: vec!["read".to_string()],
        }
    }
}

impl Default for KnowledgeBaseMetadata {
    fn default() -> Self {
        Self {
            tags: Vec::new(),
            category: None,
            language: "zh-CN".to_string(),
            version: "1.0.0".to_string(),
            author: None,
            source: None,
            custom_fields: std::collections::HashMap::new(),
        }
    }
}

/// 知识库实用方法
impl Model {
    /// 检查知识库是否活跃
    pub fn is_active(&self) -> bool {
        self.status == KnowledgeBaseStatus::Active
    }
    
    /// 检查知识库是否正在处理
    pub fn is_processing(&self) -> bool {
        self.status == KnowledgeBaseStatus::Processing
    }
    
    /// 检查知识库是否有错误
    pub fn has_error(&self) -> bool {
        self.status == KnowledgeBaseStatus::Error
    }
    
    /// 获取知识库配置
    pub fn get_config(&self) -> Result<KnowledgeBaseConfig, serde_json::Error> {
        serde_json::from_value(self.config.clone())
    }
    
    /// 获取知识库元数据
    pub fn get_metadata(&self) -> Result<KnowledgeBaseMetadata, serde_json::Error> {
        serde_json::from_value(self.metadata.clone())
    }
    
    /// 计算平均文档大小
    pub fn average_document_size(&self) -> f64 {
        if self.document_count > 0 {
            self.total_size_bytes as f64 / self.document_count as f64
        } else {
            0.0
        }
    }
    
    /// 计算平均块大小
    pub fn average_chunk_size(&self) -> f64 {
        if self.chunk_count > 0 {
            self.total_size_bytes as f64 / self.chunk_count as f64
        } else {
            0.0
        }
    }
    
    /// 检查是否需要重新索引
    pub fn needs_reindexing(&self) -> bool {
        if let Some(last_indexed) = self.last_indexed_at {
            let now = chrono::Utc::now();
            let last_indexed_utc = last_indexed.with_timezone(&chrono::Utc);
            // 如果超过24小时未索引，则需要重新索引
            (now - last_indexed_utc).num_hours() > 24
        } else {
            true // 从未索引过
        }
    }
    
    /// 获取存储大小的人类可读格式
    pub fn formatted_size(&self) -> String {
        let size = self.total_size_bytes as f64;
        if size < 1024.0 {
            format!("{} B", size)
        } else if size < 1024.0 * 1024.0 {
            format!("{:.2} KB", size / 1024.0)
        } else if size < 1024.0 * 1024.0 * 1024.0 {
            format!("{:.2} MB", size / (1024.0 * 1024.0))
        } else {
            format!("{:.2} GB", size / (1024.0 * 1024.0 * 1024.0))
        }
    }
    
    /// 检查用户是否有访问权限
    pub fn has_access(&self, user_role: &str, user_id: &str) -> Result<bool, serde_json::Error> {
        let config = self.get_config()?;
        let access_control = &config.access_control;
        
        // 检查是否公开
        if access_control.is_public {
            return Ok(true);
        }
        
        // 检查角色权限
        if access_control.allowed_roles.contains(&user_role.to_string()) {
            return Ok(true);
        }
        
        // 检查用户权限
        if access_control.allowed_users.contains(&user_id.to_string()) {
            return Ok(true);
        }
        
        Ok(false)
    }
}