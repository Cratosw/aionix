// 文档实体定义

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 文档状态枚举
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "document_status")]
pub enum DocumentStatus {
    #[sea_orm(string_value = "pending")]
    Pending,
    #[sea_orm(string_value = "processing")]
    Processing,
    #[sea_orm(string_value = "completed")]
    Completed,
    #[sea_orm(string_value = "failed")]
    Failed,
    #[sea_orm(string_value = "archived")]
    Archived,
}

/// 文档类型枚举
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "document_type")]
pub enum DocumentType {
    #[sea_orm(string_value = "text")]
    Text,
    #[sea_orm(string_value = "pdf")]
    Pdf,
    #[sea_orm(string_value = "word")]
    Word,
    #[sea_orm(string_value = "markdown")]
    Markdown,
    #[sea_orm(string_value = "html")]
    Html,
    #[sea_orm(string_value = "csv")]
    Csv,
    #[sea_orm(string_value = "json")]
    Json,
    #[sea_orm(string_value = "xml")]
    Xml,
}

/// 文档实体
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "documents")]
pub struct Model {
    /// 文档 ID
    #[sea_orm(primary_key)]
    pub id: Uuid,
    
    /// 知识库 ID
    pub knowledge_base_id: Uuid,
    
    /// 文档标题
    #[sea_orm(column_type = "String(Some(500))")]
    pub title: String,
    
    /// 文档内容
    #[sea_orm(column_type = "Text")]
    pub content: String,
    
    /// 原始内容（未处理的）
    #[sea_orm(column_type = "Text", nullable)]
    pub raw_content: Option<String>,
    
    /// 文档摘要
    #[sea_orm(column_type = "Text", nullable)]
    pub summary: Option<String>,
    
    /// 文档类型
    pub doc_type: DocumentType,
    
    /// 文档状态
    pub status: DocumentStatus,
    
    /// 文件路径
    #[sea_orm(column_type = "String(Some(1000))", nullable)]
    pub file_path: Option<String>,
    
    /// 文件名
    #[sea_orm(column_type = "String(Some(255))", nullable)]
    pub file_name: Option<String>,
    
    /// 文件大小（字节）
    pub file_size: i64,
    
    /// MIME 类型
    #[sea_orm(column_type = "String(Some(100))", nullable)]
    pub mime_type: Option<String>,
    
    /// 文档哈希值（用于去重）
    #[sea_orm(column_type = "String(Some(64))", nullable)]
    pub content_hash: Option<String>,
    
    /// 文档元数据（JSON 格式）
    #[sea_orm(column_type = "Json")]
    pub metadata: Json,
    
    /// 处理配置（JSON 格式）
    #[sea_orm(column_type = "Json")]
    pub processing_config: Json,
    
    /// 文档块数量
    pub chunk_count: i32,
    
    /// 处理开始时间
    #[sea_orm(nullable)]
    pub processing_started_at: Option<DateTimeWithTimeZone>,
    
    /// 处理完成时间
    #[sea_orm(nullable)]
    pub processing_completed_at: Option<DateTimeWithTimeZone>,
    
    /// 错误信息
    #[sea_orm(column_type = "Text", nullable)]
    pub error_message: Option<String>,
    
    /// 版本号
    pub version: i32,
    
    /// 创建时间
    pub created_at: DateTimeWithTimeZone,
    
    /// 更新时间
    pub updated_at: DateTimeWithTimeZone,
}

/// 文档关联关系
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 多对一：文档 -> 知识库
    #[sea_orm(
        belongs_to = "super::knowledge_base::Entity",
        from = "Column::KnowledgeBaseId",
        to = "super::knowledge_base::Column::Id"
    )]
    KnowledgeBase,
    
    /// 一对多：文档 -> 文档块
    #[sea_orm(has_many = "super::document_chunk::Entity")]
    DocumentChunks,
}

/// 实现与知识库的关联
impl Related<super::knowledge_base::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::KnowledgeBase.def()
    }
}

/// 实现与文档块的关联
impl Related<super::document_chunk::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::DocumentChunks.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

/// 文档元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentMetadata {
    /// 作者
    pub author: Option<String>,
    /// 创建日期
    pub created_date: Option<chrono::NaiveDateTime>,
    /// 修改日期
    pub modified_date: Option<chrono::NaiveDateTime>,
    /// 标签
    pub tags: Vec<String>,
    /// 分类
    pub category: Option<String>,
    /// 语言
    pub language: String,
    /// 来源 URL
    pub source_url: Option<String>,
    /// 页数（对于 PDF 等）
    pub page_count: Option<i32>,
    /// 字数
    pub word_count: Option<i32>,
    /// 字符数
    pub char_count: Option<i32>,
    /// 自定义字段
    pub custom_fields: std::collections::HashMap<String, serde_json::Value>,
}

/// 文档处理配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentProcessingConfig {
    /// 是否提取文本
    pub extract_text: bool,
    /// 是否生成摘要
    pub generate_summary: bool,
    /// 是否提取关键词
    pub extract_keywords: bool,
    /// 是否检测语言
    pub detect_language: bool,
    /// 分块配置
    pub chunking_config: ChunkingConfig,
    /// OCR 配置（对于图片和扫描文档）
    pub ocr_config: Option<OcrConfig>,
    /// 自定义处理步骤
    pub custom_processors: Vec<String>,
}

/// 分块配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkingConfig {
    /// 是否启用分块
    pub enabled: bool,
    /// 分块策略
    pub strategy: String,
    /// 块大小
    pub chunk_size: u32,
    /// 重叠大小
    pub overlap_size: u32,
    /// 保留元数据
    pub preserve_metadata: bool,
}

/// OCR 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrConfig {
    /// OCR 引擎
    pub engine: String,
    /// 语言
    pub language: String,
    /// 置信度阈值
    pub confidence_threshold: f32,
    /// 预处理选项
    pub preprocessing: Vec<String>,
}

impl Default for DocumentMetadata {
    fn default() -> Self {
        Self {
            author: None,
            created_date: None,
            modified_date: None,
            tags: Vec::new(),
            category: None,
            language: "zh-CN".to_string(),
            source_url: None,
            page_count: None,
            word_count: None,
            char_count: None,
            custom_fields: std::collections::HashMap::new(),
        }
    }
}

impl Default for DocumentProcessingConfig {
    fn default() -> Self {
        Self {
            extract_text: true,
            generate_summary: false,
            extract_keywords: false,
            detect_language: true,
            chunking_config: ChunkingConfig::default(),
            ocr_config: None,
            custom_processors: Vec::new(),
        }
    }
}

impl Default for ChunkingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            strategy: "fixed_size".to_string(),
            chunk_size: 1000,
            overlap_size: 200,
            preserve_metadata: true,
        }
    }
}

/// 文档实用方法
impl Model {
    /// 检查文档是否处理完成
    pub fn is_completed(&self) -> bool {
        self.status == DocumentStatus::Completed
    }
    
    /// 检查文档是否正在处理
    pub fn is_processing(&self) -> bool {
        self.status == DocumentStatus::Processing
    }
    
    /// 检查文档是否处理失败
    pub fn has_failed(&self) -> bool {
        self.status == DocumentStatus::Failed
    }
    
    /// 检查文档是否待处理
    pub fn is_pending(&self) -> bool {
        self.status == DocumentStatus::Pending
    }
    
    /// 获取文档元数据
    pub fn get_metadata(&self) -> Result<DocumentMetadata, serde_json::Error> {
        serde_json::from_value(self.metadata.clone())
    }
    
    /// 获取处理配置
    pub fn get_processing_config(&self) -> Result<DocumentProcessingConfig, serde_json::Error> {
        serde_json::from_value(self.processing_config.clone())
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
    
    /// 获取文件大小的人类可读格式
    pub fn formatted_file_size(&self) -> String {
        let size = self.file_size as f64;
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
    
    /// 获取文档类型的显示名称
    pub fn type_display_name(&self) -> &'static str {
        match self.doc_type {
            DocumentType::Text => "文本文档",
            DocumentType::Pdf => "PDF 文档",
            DocumentType::Word => "Word 文档",
            DocumentType::Markdown => "Markdown 文档",
            DocumentType::Html => "HTML 文档",
            DocumentType::Csv => "CSV 文件",
            DocumentType::Json => "JSON 文件",
            DocumentType::Xml => "XML 文件",
        }
    }
    
    /// 检查是否支持 OCR
    pub fn supports_ocr(&self) -> bool {
        matches!(self.doc_type, DocumentType::Pdf)
    }
    
    /// 检查是否需要文本提取
    pub fn needs_text_extraction(&self) -> bool {
        !matches!(self.doc_type, DocumentType::Text | DocumentType::Markdown)
    }
    
    /// 获取文档进度百分比
    pub fn progress_percentage(&self) -> f32 {
        match self.status {
            DocumentStatus::Pending => 0.0,
            DocumentStatus::Processing => {
                // 可以根据实际处理步骤计算更精确的进度
                50.0
            }
            DocumentStatus::Completed => 100.0,
            DocumentStatus::Failed => 0.0,
            DocumentStatus::Archived => 100.0,
        }
    }
    
    /// 检查内容是否发生变化
    pub fn content_changed(&self, new_content_hash: &str) -> bool {
        if let Some(current_hash) = &self.content_hash {
            current_hash != new_content_hash
        } else {
            true
        }
    }
}