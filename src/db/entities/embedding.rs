// 向量嵌入实体定义

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 嵌入状态枚举
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "embedding_status")]
pub enum EmbeddingStatus {
    #[sea_orm(string_value = "pending")]
    Pending,
    #[sea_orm(string_value = "processing")]
    Processing,
    #[sea_orm(string_value = "completed")]
    Completed,
    #[sea_orm(string_value = "failed")]
    Failed,
}

/// 嵌入类型枚举
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "embedding_type")]
pub enum EmbeddingType {
    #[sea_orm(string_value = "text")]
    Text,
    #[sea_orm(string_value = "title")]
    Title,
    #[sea_orm(string_value = "summary")]
    Summary,
    #[sea_orm(string_value = "keyword")]
    Keyword,
}

/// 向量嵌入实体
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "embeddings")]
pub struct Model {
    /// 嵌入 ID
    #[sea_orm(primary_key)]
    pub id: Uuid,
    
    /// 文档块 ID
    pub chunk_id: Uuid,
    
    /// 文档 ID（冗余字段，便于查询）
    pub document_id: Uuid,
    
    /// 知识库 ID（冗余字段，便于查询）
    pub knowledge_base_id: Uuid,
    
    /// 嵌入类型
    pub embedding_type: EmbeddingType,
    
    /// 嵌入状态
    pub status: EmbeddingStatus,
    
    /// 向量数据（使用 pgvector 扩展）
    #[sea_orm(column_type = "Text", nullable)]
    pub vector: Option<String>, // TODO: switch to pgvector when integration is ready
    
    /// 向量维度
    pub dimension: i32,
    
    /// 嵌入模型名称
    #[sea_orm(column_type = "String(Some(255))")]
    pub model_name: String,
    
    /// 模型版本
    #[sea_orm(column_type = "String(Some(100))")]
    pub model_version: String,
    
    /// 原始文本（用于生成嵌入的文本）
    #[sea_orm(column_type = "Text")]
    pub source_text: String,
    
    /// 文本哈希（用于去重）
    #[sea_orm(column_type = "String(Some(64))")]
    pub text_hash: String,
    
    /// 嵌入元数据（JSON 格式）
    #[sea_orm(column_type = "Json")]
    pub metadata: Json,
    
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

/// 向量嵌入关联关系
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 多对一：嵌入 -> 文档块
    #[sea_orm(
        belongs_to = "super::document_chunk::Entity",
        from = "Column::ChunkId",
        to = "super::document_chunk::Column::Id"
    )]
    DocumentChunk,
    
    /// 多对一：嵌入 -> 文档
    #[sea_orm(
        belongs_to = "super::document::Entity",
        from = "Column::DocumentId",
        to = "super::document::Column::Id"
    )]
    Document,
    
    /// 多对一：嵌入 -> 知识库
    #[sea_orm(
        belongs_to = "super::knowledge_base::Entity",
        from = "Column::KnowledgeBaseId",
        to = "super::knowledge_base::Column::Id"
    )]
    KnowledgeBase,
}

/// 实现与文档块的关联
impl Related<super::document_chunk::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::DocumentChunk.def()
    }
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

impl ActiveModelBehavior for ActiveModel {}

/// 嵌入元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingMetadata {
    /// 生成参数
    pub generation_params: GenerationParams,
    /// 质量指标
    pub quality_metrics: QualityMetrics,
    /// 处理信息
    pub processing_info: ProcessingInfo,
    /// 自定义字段
    pub custom_fields: std::collections::HashMap<String, serde_json::Value>,
}

/// 生成参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationParams {
    /// 温度参数
    pub temperature: Option<f32>,
    /// 最大长度
    pub max_length: Option<u32>,
    /// 批处理大小
    pub batch_size: Option<u32>,
    /// 其他参数
    pub other_params: std::collections::HashMap<String, serde_json::Value>,
}

/// 质量指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityMetrics {
    /// 置信度分数
    pub confidence_score: Option<f32>,
    /// 相似度分数（与其他嵌入的平均相似度）
    pub similarity_score: Option<f32>,
    /// 异常检测分数
    pub anomaly_score: Option<f32>,
    /// 质量等级
    pub quality_grade: Option<String>,
}

/// 处理信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingInfo {
    /// 处理耗时（毫秒）
    pub processing_time_ms: Option<u64>,
    /// 重试次数
    pub retry_count: u32,
    /// API 调用次数
    pub api_calls: u32,
    /// 使用的 GPU/CPU 信息
    pub compute_info: Option<String>,
}

impl Default for EmbeddingMetadata {
    fn default() -> Self {
        Self {
            generation_params: GenerationParams::default(),
            quality_metrics: QualityMetrics::default(),
            processing_info: ProcessingInfo::default(),
            custom_fields: std::collections::HashMap::new(),
        }
    }
}

impl Default for GenerationParams {
    fn default() -> Self {
        Self {
            temperature: None,
            max_length: None,
            batch_size: None,
            other_params: std::collections::HashMap::new(),
        }
    }
}

impl Default for QualityMetrics {
    fn default() -> Self {
        Self {
            confidence_score: None,
            similarity_score: None,
            anomaly_score: None,
            quality_grade: None,
        }
    }
}

impl Default for ProcessingInfo {
    fn default() -> Self {
        Self {
            processing_time_ms: None,
            retry_count: 0,
            api_calls: 0,
            compute_info: None,
        }
    }
}

/// 向量嵌入实用方法
impl Model {
    /// 检查嵌入是否完成
    pub fn is_completed(&self) -> bool {
        self.status == EmbeddingStatus::Completed
    }
    
    /// 检查嵌入是否正在处理
    pub fn is_processing(&self) -> bool {
        self.status == EmbeddingStatus::Processing
    }
    
    /// 检查嵌入是否失败
    pub fn has_failed(&self) -> bool {
        self.status == EmbeddingStatus::Failed
    }
    
    /// 检查嵌入是否待处理
    pub fn is_pending(&self) -> bool {
        self.status == EmbeddingStatus::Pending
    }
    
    /// 获取嵌入元数据
    pub fn get_metadata(&self) -> Result<EmbeddingMetadata, serde_json::Error> {
        serde_json::from_value(self.metadata.clone())
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
    
    /// 检查向量是否存在
    pub fn has_vector(&self) -> bool {
        self.vector.is_some()
    }
    
    /// 获取向量数组（解析字符串格式的向量）
    pub fn get_vector_array(&self) -> Result<Vec<f32>, String> {
        if let Some(vector_str) = &self.vector {
            // 解析 pgvector 格式的字符串，例如 "[0.1,0.2,0.3]"
            let trimmed = vector_str.trim_start_matches('[').trim_end_matches(']');
            let values: Result<Vec<f32>, _> = trimmed
                .split(',')
                .map(|s| s.trim().parse::<f32>())
                .collect();
            
            values.map_err(|e| format!("Failed to parse vector: {}", e))
        } else {
            Err("No vector data available".to_string())
        }
    }
    
    /// 设置向量数组（转换为字符串格式）
    pub fn set_vector_array(&mut self, vector: Vec<f32>) {
        let vector_str = format!("[{}]", 
            vector.iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(",")
        );
        self.vector = Some(vector_str);
        self.dimension = vector.len() as i32;
    }
    
    /// 计算与另一个向量的余弦相似度
    pub fn cosine_similarity(&self, other: &Model) -> Result<f32, String> {
        let vec1 = self.get_vector_array()?;
        let vec2 = other.get_vector_array()?;
        
        if vec1.len() != vec2.len() {
            return Err("Vector dimensions do not match".to_string());
        }
        
        let dot_product: f32 = vec1.iter().zip(vec2.iter()).map(|(a, b)| a * b).sum();
        let norm1: f32 = vec1.iter().map(|a| a * a).sum::<f32>().sqrt();
        let norm2: f32 = vec2.iter().map(|b| b * b).sum::<f32>().sqrt();
        
        if norm1 == 0.0 || norm2 == 0.0 {
            return Ok(0.0);
        }
        
        Ok(dot_product / (norm1 * norm2))
    }
    
    /// 计算向量的L2范数
    pub fn vector_norm(&self) -> Result<f32, String> {
        let vector = self.get_vector_array()?;
        let norm = vector.iter().map(|v| v * v).sum::<f32>().sqrt();
        Ok(norm)
    }
    
    /// 检查向量是否归一化
    pub fn is_normalized(&self, tolerance: f32) -> Result<bool, String> {
        let norm = self.vector_norm()?;
        Ok((norm - 1.0).abs() < tolerance)
    }
    
    /// 获取嵌入类型的显示名称
    pub fn type_display_name(&self) -> &'static str {
        match self.embedding_type {
            EmbeddingType::Text => "文本嵌入",
            EmbeddingType::Title => "标题嵌入",
            EmbeddingType::Summary => "摘要嵌入",
            EmbeddingType::Keyword => "关键词嵌入",
        }
    }
    
    /// 检查文本是否发生变化
    pub fn text_changed(&self, new_text_hash: &str) -> bool {
        self.text_hash != new_text_hash
    }
    
    /// 获取嵌入的唯一标识符
    pub fn cache_key(&self) -> String {
        format!("embedding:{}:{}:{}", self.model_name, self.model_version, self.text_hash)
    }
    
    /// 检查模型是否兼容
    pub fn is_model_compatible(&self, model_name: &str, model_version: &str) -> bool {
        self.model_name == model_name && self.model_version == model_version
    }
    
    /// 获取质量分数
    pub fn quality_score(&self) -> f32 {
        if let Ok(metadata) = self.get_metadata() {
            metadata.quality_metrics.confidence_score.unwrap_or(0.5)
        } else {
            0.5
        }
    }
}