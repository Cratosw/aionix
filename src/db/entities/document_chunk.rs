// 文档块实体定义

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 文档块状态枚举
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "chunk_status")]
pub enum ChunkStatus {
    #[sea_orm(string_value = "pending")]
    Pending,
    #[sea_orm(string_value = "processing")]
    Processing,
    #[sea_orm(string_value = "completed")]
    Completed,
    #[sea_orm(string_value = "failed")]
    Failed,
}

/// 文档块实体
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "document_chunks")]
pub struct Model {
    /// 文档块 ID
    #[sea_orm(primary_key)]
    pub id: Uuid,
    
    /// 文档 ID
    pub document_id: Uuid,
    
    /// 知识库 ID（冗余字段，便于查询）
    pub knowledge_base_id: Uuid,
    
    /// 块序号（在文档中的位置）
    pub chunk_index: i32,
    
    /// 块内容
    #[sea_orm(column_type = "Text")]
    pub content: String,
    
    /// 块标题（如果有）
    #[sea_orm(column_type = "String(Some(500))", nullable)]
    pub title: Option<String>,
    
    /// 块摘要
    #[sea_orm(column_type = "Text", nullable)]
    pub summary: Option<String>,
    
    /// 块状态
    pub status: ChunkStatus,
    
    /// 内容长度（字符数）
    pub content_length: i32,
    
    /// 词数
    pub word_count: i32,
    
    /// 内容哈希
    #[sea_orm(column_type = "String(Some(64))")]
    pub content_hash: String,
    
    /// 块元数据（JSON 格式）
    #[sea_orm(column_type = "Json")]
    pub metadata: Json,
    
    /// 在原文档中的位置信息
    #[sea_orm(column_type = "Json")]
    pub position_info: Json,
    
    /// 处理开始时间
    #[sea_orm(nullable)]
    pub processing_started_at: Option<DateTimeWithTimeZone>,
    
    /// 处理完成时间
    #[sea_orm(nullable)]
    pub processing_completed_at: Option<DateTimeWithTimeZone>,
    
    /// 错误信息
    #[sea_orm(column_type = "Text", nullable)]
    pub error_message: Option<String>,
    
    /// 创建时间
    pub created_at: DateTimeWithTimeZone,
    
    /// 更新时间
    pub updated_at: DateTimeWithTimeZone,
}

/// 文档块关联关系
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 多对一：文档块 -> 文档
    #[sea_orm(
        belongs_to = "super::document::Entity",
        from = "Column::DocumentId",
        to = "super::document::Column::Id"
    )]
    Document,
    
    /// 多对一：文档块 -> 知识库
    #[sea_orm(
        belongs_to = "super::knowledge_base::Entity",
        from = "Column::KnowledgeBaseId",
        to = "super::knowledge_base::Column::Id"
    )]
    KnowledgeBase,
    
    /// 一对多：文档块 -> 向量嵌入
    #[sea_orm(has_many = "super::embedding::Entity")]
    Embeddings,
}

/// 实现与文档的关联
impl Related<super::document::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Document.def()
    }
}

/// 实现与知识库的关联
impl Related<super::knowledge_base::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::KnowledgeBase.def()
    }
}

/// 实现与向量嵌入的关联
impl Related<super::embedding::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Embeddings.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

/// 文档块元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkMetadata {
    /// 章节标题
    pub section_title: Option<String>,
    /// 页码（对于 PDF）
    pub page_number: Option<i32>,
    /// 段落编号
    pub paragraph_number: Option<i32>,
    /// 标签
    pub tags: Vec<String>,
    /// 关键词
    pub keywords: Vec<String>,
    /// 语言
    pub language: String,
    /// 置信度分数
    pub confidence_score: Option<f32>,
    /// 重要性分数
    pub importance_score: Option<f32>,
    /// 自定义字段
    pub custom_fields: std::collections::HashMap<String, serde_json::Value>,
}

/// 位置信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionInfo {
    /// 开始位置（字符偏移）
    pub start_offset: u32,
    /// 结束位置（字符偏移）
    pub end_offset: u32,
    /// 开始行号
    pub start_line: Option<u32>,
    /// 结束行号
    pub end_line: Option<u32>,
    /// 页码范围
    pub page_range: Option<PageRange>,
    /// 章节路径
    pub section_path: Vec<String>,
}

/// 页码范围
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageRange {
    /// 开始页码
    pub start_page: u32,
    /// 结束页码
    pub end_page: u32,
}

impl Default for ChunkMetadata {
    fn default() -> Self {
        Self {
            section_title: None,
            page_number: None,
            paragraph_number: None,
            tags: Vec::new(),
            keywords: Vec::new(),
            language: "zh-CN".to_string(),
            confidence_score: None,
            importance_score: None,
            custom_fields: std::collections::HashMap::new(),
        }
    }
}

impl Default for PositionInfo {
    fn default() -> Self {
        Self {
            start_offset: 0,
            end_offset: 0,
            start_line: None,
            end_line: None,
            page_range: None,
            section_path: Vec::new(),
        }
    }
}

/// 文档块实用方法
impl Model {
    /// 检查块是否处理完成
    pub fn is_completed(&self) -> bool {
        self.status == ChunkStatus::Completed
    }
    
    /// 检查块是否正在处理
    pub fn is_processing(&self) -> bool {
        self.status == ChunkStatus::Processing
    }
    
    /// 检查块是否处理失败
    pub fn has_failed(&self) -> bool {
        self.status == ChunkStatus::Failed
    }
    
    /// 检查块是否待处理
    pub fn is_pending(&self) -> bool {
        self.status == ChunkStatus::Pending
    }
    
    /// 获取块元数据
    pub fn get_metadata(&self) -> Result<ChunkMetadata, serde_json::Error> {
        serde_json::from_value(self.metadata.clone())
    }
    
    /// 获取位置信息
    pub fn get_position_info(&self) -> Result<PositionInfo, serde_json::Error> {
        serde_json::from_value(self.position_info.clone())
    }
    
    /// 计算处理耗时
    pub fn processing_duration(&self) -> Option<chrono::Duration> {
        if let (Some(start), Some(end)) = (self.processing_started_at, self.processing_completed_at) {
            let start_utc = start.with_timezone(&chrono::Utc);
            let end_utc = end.with_timezone(&chrono::Utc);
            Some(end_utc - start_utc)
        } else {
            None
        }
    }
    
    /// 获取内容预览（前100个字符）
    pub fn content_preview(&self) -> String {
        if self.content.len() > 100 {
            format!("{}...", &self.content[..100])
        } else {
            self.content.clone()
        }
    }
    
    /// 计算内容密度（词数/字符数）
    pub fn content_density(&self) -> f32 {
        if self.content_length > 0 {
            self.word_count as f32 / self.content_length as f32
        } else {
            0.0
        }
    }
    
    /// 检查是否为标题块
    pub fn is_title_chunk(&self) -> bool {
        self.title.is_some() && self.content_length < 200
    }
    
    /// 检查是否为长内容块
    pub fn is_long_content(&self) -> bool {
        self.content_length > 1000
    }
    
    /// 获取块的重要性分数
    pub fn importance_score(&self) -> f32 {
        if let Ok(metadata) = self.get_metadata() {
            metadata.importance_score.unwrap_or(0.5)
        } else {
            0.5
        }
    }
    
    /// 检查块是否包含关键词
    pub fn contains_keywords(&self, keywords: &[String]) -> bool {
        let content_lower = self.content.to_lowercase();
        keywords.iter().any(|keyword| content_lower.contains(&keyword.to_lowercase()))
    }
    
    /// 获取块在文档中的相对位置（百分比）
    pub fn relative_position(&self, total_chunks: i32) -> f32 {
        if total_chunks > 0 {
            (self.chunk_index as f32 / total_chunks as f32) * 100.0
        } else {
            0.0
        }
    }
    
    /// 检查内容是否发生变化
    pub fn content_changed(&self, new_content_hash: &str) -> bool {
        self.content_hash != new_content_hash
    }
    
    /// 获取块的唯一标识符（用于缓存等）
    pub fn cache_key(&self) -> String {
        format!("chunk:{}:{}", self.id, self.content_hash)
    }
}