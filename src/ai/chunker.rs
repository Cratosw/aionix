// 文档分块模块
// 实现智能文档分块算法

use crate::ai::{RigAiClientManager, ExtractedText};
use crate::errors::AiStudioError;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// 文档分块器特征
#[async_trait]
pub trait DocumentChunker: Send + Sync {
    /// 将文档分块
    async fn chunk_document(&self, text: &ExtractedText) -> Result<Vec<DocumentChunk>, AiStudioError>;
    
    /// 获取分块器配置
    fn get_config(&self) -> &ChunkerConfig;
}

/// 文档块
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentChunk {
    pub id: Uuid,
    pub content: String,
    pub metadata: ChunkMetadata,
    pub embedding: Option<Vec<f32>>,
    pub position: ChunkPosition,
}

/// 块元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkMetadata {
    pub chunk_index: usize,
    pub total_chunks: usize,
    pub word_count: u32,
    pub character_count: u32,
    pub language: Option<String>,
    pub chunk_type: ChunkType,
    pub source_page: Option<u32>,
    pub overlap_with_previous: bool,
    pub overlap_with_next: bool,
    pub custom_properties: HashMap<String, String>,
}

/// 块位置信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkPosition {
    pub start_char: usize,
    pub end_char: usize,
    pub start_line: Option<usize>,
    pub end_line: Option<usize>,
}

/// 块类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ChunkType {
    Text,
    Heading,
    List,
    Table,
    Code,
    Quote,
    Other,
}

/// 分块器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkerConfig {
    pub max_chunk_size: usize,
    pub min_chunk_size: usize,
    pub overlap_size: usize,
    pub preserve_sentences: bool,
    pub preserve_paragraphs: bool,
    pub split_on_headers: bool,
    pub chunk_type: ChunkerType,
    pub language: Option<String>,
}

/// 分块器类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ChunkerType {
    Fixed,      // 固定大小分块
    Semantic,   // 语义分块
    Sentence,   // 句子分块
    Paragraph,  // 段落分块
    Hybrid,     // 混合分块
}

impl Default for ChunkerConfig {
    fn default() -> Self {
        Self {
            max_chunk_size: 1000,
            min_chunk_size: 100,
            overlap_size: 100,
            preserve_sentences: true,
            preserve_paragraphs: true,
            split_on_headers: true,
            chunk_type: ChunkerType::Hybrid,
            language: Some("zh-CN".to_string()),
        }
    }
}

/// 文档向量化器
#[async_trait]
pub trait DocumentVectorizer: Send + Sync {
    /// 为文档块生成向量
    async fn vectorize_chunks(&self, chunks: &mut [DocumentChunk]) -> Result<(), AiStudioError>;
    
    /// 批量向量化
    async fn batch_vectorize(&self, chunks: &mut [DocumentChunk], batch_size: usize) -> Result<(), AiStudioError>;
}

/// 文档处理管道
pub struct DocumentProcessingPipeline {
    chunker: Box<dyn DocumentChunker>,
    vectorizer: Box<dyn DocumentVectorizer>,
}

impl DocumentProcessingPipeline {
    /// 创建新的处理管道
    pub fn new(
        chunker: Box<dyn DocumentChunker>,
        vectorizer: Box<dyn DocumentVectorizer>,
    ) -> Self {
        Self { chunker, vectorizer }
    }
    
    /// 处理文档
    pub async fn process(&self, text: &ExtractedText) -> Result<Vec<DocumentChunk>, AiStudioError> {
        info!("开始处理文档，内容长度: {} 字符", text.content.len());
        
        // 1. 分块
        let mut chunks = self.chunker.chunk_document(text).await?;
        info!("文档分块完成，共 {} 个块", chunks.len());
        
        // 2. 向量化
        self.vectorizer.vectorize_chunks(&mut chunks).await?;
        info!("文档向量化完成");
        
        Ok(chunks)
    }
    
    /// 批量处理文档
    pub async fn batch_process(&self, texts: &[ExtractedText]) -> Result<Vec<Vec<DocumentChunk>>, AiStudioError> {
        let mut results = Vec::new();
        
        for text in texts {
            let chunks = self.process(text).await?;
            results.push(chunks);
        }
        
        Ok(results)
    }
}

/// 混合分块器实现
pub struct HybridChunker {
    config: ChunkerConfig,
}

impl HybridChunker {
    pub fn new(config: ChunkerConfig) -> Self {
        Self { config }
    }
    
    pub fn with_default_config() -> Self {
        Self::new(ChunkerConfig::default())
    }
}

#[async_trait]
impl DocumentChunker for HybridChunker {
    async fn chunk_document(&self, text: &ExtractedText) -> Result<Vec<DocumentChunk>, AiStudioError> {
        debug!("使用混合分块器处理文档，配置: {:?}", self.config);
        
        let content = &text.content;
        let mut chunks = Vec::new();
        
        // 首先尝试按段落分割
        let paragraphs = self.split_by_paragraphs(content);
        
        let mut current_chunk = String::new();
        let mut chunk_start = 0;
        let mut chunk_index = 0;
        
        for paragraph in paragraphs {
            let paragraph_trimmed = paragraph.trim();
            if paragraph_trimmed.is_empty() {
                continue;
            }
            
            // 检查是否是标题
            let chunk_type = self.detect_chunk_type(paragraph_trimmed);
            
            // 如果当前块加上新段落会超过最大大小，先保存当前块
            if !current_chunk.is_empty() && 
               (current_chunk.len() + paragraph_trimmed.len() > self.config.max_chunk_size ||
                (self.config.split_on_headers && chunk_type == ChunkType::Heading)) {
                
                let chunk = self.create_chunk(
                    &current_chunk,
                    chunk_index,
                    chunk_start,
                    chunk_start + current_chunk.len(),
                    ChunkType::Text,
                )?;
                chunks.push(chunk);
                
                chunk_index += 1;
                chunk_start += current_chunk.len();
                current_chunk.clear();
            }
            
            // 添加段落到当前块
            if !current_chunk.is_empty() {
                current_chunk.push('\n');
            }
            current_chunk.push_str(paragraph_trimmed);
        }
        
        // 处理最后一个块
        if !current_chunk.is_empty() {
            let chunk = self.create_chunk(
                &current_chunk,
                chunk_index,
                chunk_start,
                chunk_start + current_chunk.len(),
                ChunkType::Text,
            )?;
            chunks.push(chunk);
        }
        
        // 如果没有生成任何块，创建一个包含全部内容的块
        if chunks.is_empty() && !content.is_empty() {
            let chunk = self.create_chunk(content, 0, 0, content.len(), ChunkType::Text)?;
            chunks.push(chunk);
        }
        
        // 更新总块数信息
        let total_chunks = chunks.len();
        for chunk in &mut chunks {
            chunk.metadata.total_chunks = total_chunks;
        }
        
        // 添加重叠信息
        self.add_overlap_info(&mut chunks);
        
        info!("混合分块完成，生成 {} 个块", chunks.len());
        Ok(chunks)
    }
    
    fn get_config(&self) -> &ChunkerConfig {
        &self.config
    }
}

impl HybridChunker {
    fn split_by_paragraphs<'a>(&self, content: &'a str) -> Vec<&'a str> {
        content.split("\n\n")
            .filter(|p| !p.trim().is_empty())
            .collect()
    }
    
    fn detect_chunk_type(&self, content: &str) -> ChunkType {
        let trimmed = content.trim();
        
        // 检查是否是标题
        if trimmed.starts_with('#') || 
           (trimmed.len() < 100 && trimmed.lines().count() == 1 && 
            !trimmed.ends_with('.') && !trimmed.ends_with('。')) {
            return ChunkType::Heading;
        }
        
        // 检查是否是列表
        if trimmed.starts_with('-') || trimmed.starts_with('*') || 
           trimmed.starts_with("1.") || trimmed.starts_with("•") {
            return ChunkType::List;
        }
        
        // 检查是否是代码块
        if trimmed.starts_with("```") || trimmed.starts_with("    ") {
            return ChunkType::Code;
        }
        
        // 检查是否是引用
        if trimmed.starts_with('>') || trimmed.starts_with('"') {
            return ChunkType::Quote;
        }
        
        ChunkType::Text
    }
    
    fn create_chunk(
        &self,
        content: &str,
        index: usize,
        start_char: usize,
        end_char: usize,
        chunk_type: ChunkType,
    ) -> Result<DocumentChunk, AiStudioError> {
        let content = content.trim().to_string();
        let word_count = content.split_whitespace().count() as u32;
        let character_count = content.len() as u32;
        
        Ok(DocumentChunk {
            id: Uuid::new_v4(),
            content,
            metadata: ChunkMetadata {
                chunk_index: index,
                total_chunks: 0, // 将在后面更新
                word_count,
                character_count,
                language: self.config.language.clone(),
                chunk_type,
                source_page: None,
                overlap_with_previous: false,
                overlap_with_next: false,
                custom_properties: HashMap::new(),
            },
            embedding: None,
            position: ChunkPosition {
                start_char,
                end_char,
                start_line: None,
                end_line: None,
            },
        })
    }
    
    fn add_overlap_info(&self, chunks: &mut [DocumentChunk]) {
        for i in 0..chunks.len() {
            if i > 0 {
                chunks[i].metadata.overlap_with_previous = true;
            }
            if i < chunks.len() - 1 {
                chunks[i].metadata.overlap_with_next = true;
            }
        }
    }
}

/// AI 向量化器实现
pub struct AiVectorizer {
    client_manager: RigAiClientManager,
    batch_size: usize,
}

impl AiVectorizer {
    pub fn new(client_manager: RigAiClientManager) -> Self {
        Self {
            client_manager,
            batch_size: 10,
        }
    }
    
    pub fn with_batch_size(mut self, batch_size: usize) -> Self {
        self.batch_size = batch_size;
        self
    }
}

#[async_trait]
impl DocumentVectorizer for AiVectorizer {
    async fn vectorize_chunks(&self, chunks: &mut [DocumentChunk]) -> Result<(), AiStudioError> {
        debug!("开始向量化 {} 个文档块", chunks.len());
        
        self.batch_vectorize(chunks, self.batch_size).await
    }
    
    async fn batch_vectorize(&self, chunks: &mut [DocumentChunk], batch_size: usize) -> Result<(), AiStudioError> {
        for chunk_batch in chunks.chunks_mut(batch_size) {
            let texts: Vec<String> = chunk_batch.iter()
                .map(|chunk| chunk.content.clone())
                .collect();
            
            debug!("向量化批次，包含 {} 个文档块", texts.len());
            
            let embeddings = self.client_manager.generate_embeddings(&texts).await?;
            
            if embeddings.len() != chunk_batch.len() {
                return Err(AiStudioError::ai("嵌入向量数量与文档块数量不匹配"));
            }
            
            for (chunk, embedding_response) in chunk_batch.iter_mut().zip(embeddings.iter()) {
                chunk.embedding = Some(embedding_response.embedding.clone());
            }
        }
        
        info!("批量向量化完成，处理了 {} 个文档块", chunks.len());
        Ok(())
    }
}

/// 文档处理工厂
pub struct DocumentProcessingFactory;

impl DocumentProcessingFactory {
    /// 创建默认的处理管道
    pub fn create_default_pipeline(client_manager: RigAiClientManager) -> DocumentProcessingPipeline {
        let chunker = Box::new(HybridChunker::with_default_config());
        let vectorizer = Box::new(AiVectorizer::new(client_manager));
        
        DocumentProcessingPipeline::new(chunker, vectorizer)
    }
    
    /// 创建自定义配置的处理管道
    pub fn create_custom_pipeline(
        chunker_config: ChunkerConfig,
        client_manager: RigAiClientManager,
        batch_size: usize,
    ) -> DocumentProcessingPipeline {
        let chunker = Box::new(HybridChunker::new(chunker_config));
        let vectorizer = Box::new(AiVectorizer::new(client_manager).with_batch_size(batch_size));
        
        DocumentProcessingPipeline::new(chunker, vectorizer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::{DocumentMetadata, ProcessingInfo};
    use crate::config::AiConfig;
    use std::collections::HashMap;
    
    fn create_test_extracted_text() -> ExtractedText {
        ExtractedText {
            content: "这是第一段内容。包含一些文本。\n\n这是第二段内容。包含更多的文本内容，用于测试分块功能。\n\n# 这是一个标题\n\n这是标题下的内容。".to_string(),
            metadata: DocumentMetadata {
                title: Some("测试文档".to_string()),
                author: None,
                subject: None,
                keywords: None,
                created_date: None,
                modified_date: None,
                page_count: Some(1),
                word_count: Some(20),
                language: Some("zh-CN".to_string()),
                format: "text/plain".to_string(),
                file_size: 100,
                custom_properties: HashMap::new(),
            },
            pages: None,
            processing_info: ProcessingInfo {
                processor_type: "test".to_string(),
                processing_time_ms: 0,
                success: true,
                warnings: Vec::new(),
                errors: Vec::new(),
            },
        }
    }
    
    #[tokio::test]
    async fn test_hybrid_chunker() {
        let config = ChunkerConfig {
            max_chunk_size: 50,
            min_chunk_size: 10,
            overlap_size: 5,
            preserve_sentences: true,
            preserve_paragraphs: true,
            split_on_headers: true,
            chunk_type: ChunkerType::Hybrid,
            language: Some("zh-CN".to_string()),
        };
        
        let chunker = HybridChunker::new(config);
        let text = create_test_extracted_text();
        
        let chunks = chunker.chunk_document(&text).await.unwrap();
        
        assert!(!chunks.is_empty());
        assert!(chunks.len() > 1); // 应该被分成多个块
        
        // 检查每个块的基本属性
        for (i, chunk) in chunks.iter().enumerate() {
            assert_eq!(chunk.metadata.chunk_index, i);
            assert_eq!(chunk.metadata.total_chunks, chunks.len());
            assert!(!chunk.content.is_empty());
            assert!(chunk.metadata.word_count > 0);
            assert!(chunk.metadata.character_count > 0);
        }
    }
    
    #[tokio::test]
    async fn test_chunk_type_detection() {
        let chunker = HybridChunker::with_default_config();
        
        assert_eq!(chunker.detect_chunk_type("# 这是标题"), ChunkType::Heading);
        assert_eq!(chunker.detect_chunk_type("- 这是列表项"), ChunkType::List);
        assert_eq!(chunker.detect_chunk_type("```代码块```"), ChunkType::Code);
        assert_eq!(chunker.detect_chunk_type("> 这是引用"), ChunkType::Quote);
        assert_eq!(chunker.detect_chunk_type("这是普通文本。"), ChunkType::Text);
    }
    
    #[tokio::test]
    async fn test_ai_vectorizer() {
        let config = AiConfig {
            model_endpoint: "mock://test".to_string(),
            api_key: "test".to_string(),
            max_tokens: 1000,
            temperature: 0.7,
            timeout: 30,
            retry_attempts: 3,
        };
        
        let client_manager = match RigAiClientManager::new(config).await {
            Ok(manager) => manager,
            Err(_) => return,
        };
        let vectorizer = AiVectorizer::new(client_manager);
        
        let mut chunks = vec![
            DocumentChunk {
                id: Uuid::new_v4(),
                content: "测试文本1".to_string(),
                metadata: ChunkMetadata {
                    chunk_index: 0,
                    total_chunks: 2,
                    word_count: 3,
                    character_count: 5,
                    language: Some("zh-CN".to_string()),
                    chunk_type: ChunkType::Text,
                    source_page: None,
                    overlap_with_previous: false,
                    overlap_with_next: true,
                    custom_properties: HashMap::new(),
                },
                embedding: None,
                position: ChunkPosition {
                    start_char: 0,
                    end_char: 5,
                    start_line: None,
                    end_line: None,
                },
            },
            DocumentChunk {
                id: Uuid::new_v4(),
                content: "测试文本2".to_string(),
                metadata: ChunkMetadata {
                    chunk_index: 1,
                    total_chunks: 2,
                    word_count: 3,
                    character_count: 5,
                    language: Some("zh-CN".to_string()),
                    chunk_type: ChunkType::Text,
                    source_page: None,
                    overlap_with_previous: true,
                    overlap_with_next: false,
                    custom_properties: HashMap::new(),
                },
                embedding: None,
                position: ChunkPosition {
                    start_char: 5,
                    end_char: 10,
                    start_line: None,
                    end_line: None,
                },
            },
        ];
        
        let result = vectorizer.vectorize_chunks(&mut chunks).await;
        assert!(result.is_ok());
        
        // 检查向量是否已生成
        for chunk in &chunks {
            assert!(chunk.embedding.is_some());
            assert!(!chunk.embedding.as_ref().unwrap().is_empty());
        }
    }
    
    #[tokio::test]
    async fn test_document_processing_pipeline() {
        let config = AiConfig {
            model_endpoint: "mock://test".to_string(),
            api_key: "test".to_string(),
            max_tokens: 1000,
            temperature: 0.7,
            timeout: 30,
            retry_attempts: 3,
        };
        
        let client_manager = match RigAiClientManager::new(config).await {
            Ok(manager) => manager,
            Err(_) => return,
        };
        let pipeline = DocumentProcessingFactory::create_default_pipeline(client_manager);
        
        let text = create_test_extracted_text();
        let chunks = pipeline.process(&text).await.unwrap();
        
        assert!(!chunks.is_empty());
        
        // 检查所有块都有向量
        for chunk in &chunks {
            assert!(chunk.embedding.is_some());
            assert!(!chunk.embedding.as_ref().unwrap().is_empty());
        }
    }
}