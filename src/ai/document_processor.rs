// 文档处理模块
// 实现多格式文档解析和文本提取

use crate::errors::AiStudioError;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// 文档处理器特征
#[async_trait]
pub trait DocumentProcessor: Send + Sync {
    /// 提取文档文本
    async fn extract_text(&self, file_path: &str) -> Result<ExtractedText, AiStudioError>;
    
    /// 检查是否支持该文件格式
    fn supports_format(&self, file_extension: &str) -> bool;
    
    /// 获取支持的格式列表
    fn supported_formats(&self) -> Vec<String>;
}

/// 提取的文本内容
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedText {
    pub content: String,
    pub metadata: DocumentMetadata,
    pub pages: Option<Vec<PageContent>>,
    pub processing_info: ProcessingInfo,
}

/// 文档元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentMetadata {
    pub title: Option<String>,
    pub author: Option<String>,
    pub subject: Option<String>,
    pub keywords: Option<Vec<String>>,
    pub created_date: Option<chrono::DateTime<chrono::Utc>>,
    pub modified_date: Option<chrono::DateTime<chrono::Utc>>,
    pub page_count: Option<u32>,
    pub word_count: Option<u32>,
    pub language: Option<String>,
    pub format: String,
    pub file_size: u64,
    pub custom_properties: HashMap<String, String>,
}

/// 页面内容
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageContent {
    pub page_number: u32,
    pub content: String,
    pub images: Vec<ImageInfo>,
    pub tables: Vec<TableInfo>,
}

/// 图片信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageInfo {
    pub id: String,
    pub alt_text: Option<String>,
    pub caption: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

/// 表格信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableInfo {
    pub id: String,
    pub caption: Option<String>,
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

/// 处理信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingInfo {
    pub processor_type: String,
    pub processing_time_ms: u64,
    pub success: bool,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

/// 文档处理状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProcessingStatus {
    Pending,
    Processing,
    Completed,
    Failed,
    Cancelled,
}

/// 文档处理管理器
pub struct DocumentProcessorManager {
    processors: HashMap<String, Box<dyn DocumentProcessor>>,
}

impl DocumentProcessorManager {
    /// 创建新的文档处理管理器
    pub fn new() -> Self {
        let mut manager = Self {
            processors: HashMap::new(),
        };
        
        // 注册默认处理器
        manager.register_default_processors();
        
        manager
    }
    
    /// 注册默认处理器
    fn register_default_processors(&mut self) {
        // 注册文本文件处理器
        self.register_processor("txt", Box::new(TextProcessor::new()));
        self.register_processor("md", Box::new(MarkdownProcessor::new()));
        self.register_processor("markdown", Box::new(MarkdownProcessor::new()));
        
        // 注册 PDF 处理器（模拟实现）
        self.register_processor("pdf", Box::new(PdfProcessor::new()));
        
        // 注册 Word 处理器（模拟实现）
        self.register_processor("doc", Box::new(WordProcessor::new()));
        self.register_processor("docx", Box::new(WordProcessor::new()));
        
        // 注册 HTML 处理器
        self.register_processor("html", Box::new(HtmlProcessor::new()));
        self.register_processor("htm", Box::new(HtmlProcessor::new()));
    }
    
    /// 注册处理器
    pub fn register_processor(&mut self, format: &str, processor: Box<dyn DocumentProcessor>) {
        self.processors.insert(format.to_lowercase(), processor);
    }
    
    /// 处理文档
    pub async fn process_document(&self, file_path: &str) -> Result<ExtractedText, AiStudioError> {
        let path = Path::new(file_path);
        let extension = path.extension()
            .and_then(|ext| ext.to_str())
            .ok_or_else(|| AiStudioError::file_processing("无法确定文件扩展名"))?
            .to_lowercase();
        
        let processor = self.processors.get(&extension)
            .ok_or_else(|| AiStudioError::file_processing(
                format!("不支持的文件格式: {}", extension)
            ))?;
        
        debug!("使用 {} 处理器处理文件: {}", extension, file_path);
        
        let start_time = std::time::Instant::now();
        let result = processor.extract_text(file_path).await;
        let processing_time = start_time.elapsed().as_millis() as u64;
        
        match result {
            Ok(mut extracted) => {
                extracted.processing_info.processing_time_ms = processing_time;
                info!("文档处理完成: {} ({}ms)", file_path, processing_time);
                Ok(extracted)
            }
            Err(e) => {
                warn!("文档处理失败: {} - {}", file_path, e);
                Err(e)
            }
        }
    }
    
    /// 检查是否支持文件格式
    pub fn supports_format(&self, file_extension: &str) -> bool {
        self.processors.contains_key(&file_extension.to_lowercase())
    }
    
    /// 获取支持的格式列表
    pub fn get_supported_formats(&self) -> Vec<String> {
        self.processors.keys().cloned().collect()
    }
    
    /// 批量处理文档
    pub async fn batch_process(&self, file_paths: &[String]) -> Vec<(String, Result<ExtractedText, AiStudioError>)> {
        let mut results = Vec::new();
        
        for file_path in file_paths {
            let result = self.process_document(file_path).await;
            results.push((file_path.clone(), result));
        }
        
        results
    }
}

impl Default for DocumentProcessorManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 文本文件处理器
pub struct TextProcessor;

impl TextProcessor {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl DocumentProcessor for TextProcessor {
    async fn extract_text(&self, file_path: &str) -> Result<ExtractedText, AiStudioError> {
        let content = tokio::fs::read_to_string(file_path).await
            .map_err(|e| AiStudioError::file_processing_with_name(
                format!("读取文本文件失败: {}", e),
                file_path
            ))?;
        
        let metadata = self.extract_metadata(file_path, &content).await?;
        
        Ok(ExtractedText {
            content: content.clone(),
            metadata,
            pages: None,
            processing_info: ProcessingInfo {
                processor_type: "text".to_string(),
                processing_time_ms: 0, // 将由管理器设置
                success: true,
                warnings: Vec::new(),
                errors: Vec::new(),
            },
        })
    }
    
    fn supports_format(&self, file_extension: &str) -> bool {
        matches!(file_extension.to_lowercase().as_str(), "txt")
    }
    
    fn supported_formats(&self) -> Vec<String> {
        vec!["txt".to_string()]
    }
}

impl TextProcessor {
    async fn extract_metadata(&self, file_path: &str, content: &str) -> Result<DocumentMetadata, AiStudioError> {
        let file_metadata = tokio::fs::metadata(file_path).await
            .map_err(|e| AiStudioError::file_processing(format!("获取文件元数据失败: {}", e)))?;
        
        let word_count = content.split_whitespace().count() as u32;
        
        Ok(DocumentMetadata {
            title: Path::new(file_path).file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.to_string()),
            author: None,
            subject: None,
            keywords: None,
            created_date: file_metadata.created().ok()
                .map(|t| chrono::DateTime::from(t)),
            modified_date: file_metadata.modified().ok()
                .map(|t| chrono::DateTime::from(t)),
            page_count: Some(1),
            word_count: Some(word_count),
            language: None,
            format: "text/plain".to_string(),
            file_size: file_metadata.len(),
            custom_properties: HashMap::new(),
        })
    }
}

/// Markdown 文件处理器
pub struct MarkdownProcessor;

impl MarkdownProcessor {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl DocumentProcessor for MarkdownProcessor {
    async fn extract_text(&self, file_path: &str) -> Result<ExtractedText, AiStudioError> {
        let content = tokio::fs::read_to_string(file_path).await
            .map_err(|e| AiStudioError::file_processing_with_name(
                format!("读取 Markdown 文件失败: {}", e),
                file_path
            ))?;
        
        // 简单的 Markdown 处理 - 移除标记符号
        let plain_text = self.markdown_to_text(&content);
        let metadata = self.extract_metadata(file_path, &content).await?;
        
        Ok(ExtractedText {
            content: plain_text,
            metadata,
            pages: None,
            processing_info: ProcessingInfo {
                processor_type: "markdown".to_string(),
                processing_time_ms: 0,
                success: true,
                warnings: Vec::new(),
                errors: Vec::new(),
            },
        })
    }
    
    fn supports_format(&self, file_extension: &str) -> bool {
        matches!(file_extension.to_lowercase().as_str(), "md" | "markdown")
    }
    
    fn supported_formats(&self) -> Vec<String> {
        vec!["md".to_string(), "markdown".to_string()]
    }
}

impl MarkdownProcessor {
    fn markdown_to_text(&self, markdown: &str) -> String {
        // 简单的 Markdown 到纯文本转换
        let mut text = markdown.to_string();
        
        // 移除标题标记
        text = regex::Regex::new(r"^#{1,6}\s+").unwrap()
            .replace_all(&text, "").to_string();
        
        // 移除粗体和斜体标记
        text = regex::Regex::new(r"\*\*([^*]+)\*\*").unwrap()
            .replace_all(&text, "$1").to_string();
        text = regex::Regex::new(r"\*([^*]+)\*").unwrap()
            .replace_all(&text, "$1").to_string();
        
        // 移除链接标记
        text = regex::Regex::new(r"\[([^\]]+)\]\([^)]+\)").unwrap()
            .replace_all(&text, "$1").to_string();
        
        // 移除代码块标记
        text = regex::Regex::new(r"```[^`]*```").unwrap()
            .replace_all(&text, "").to_string();
        text = regex::Regex::new(r"`([^`]+)`").unwrap()
            .replace_all(&text, "$1").to_string();
        
        text
    }
    
    async fn extract_metadata(&self, file_path: &str, content: &str) -> Result<DocumentMetadata, AiStudioError> {
        let file_metadata = tokio::fs::metadata(file_path).await
            .map_err(|e| AiStudioError::file_processing(format!("获取文件元数据失败: {}", e)))?;
        
        let word_count = content.split_whitespace().count() as u32;
        
        // 尝试从内容中提取标题
        let title = content.lines()
            .find(|line| line.starts_with('#'))
            .map(|line| line.trim_start_matches('#').trim().to_string())
            .or_else(|| {
                Path::new(file_path).file_stem()
                    .and_then(|s| s.to_str())
                    .map(|s| s.to_string())
            });
        
        Ok(DocumentMetadata {
            title,
            author: None,
            subject: None,
            keywords: None,
            created_date: file_metadata.created().ok()
                .map(|t| chrono::DateTime::from(t)),
            modified_date: file_metadata.modified().ok()
                .map(|t| chrono::DateTime::from(t)),
            page_count: Some(1),
            word_count: Some(word_count),
            language: None,
            format: "text/markdown".to_string(),
            file_size: file_metadata.len(),
            custom_properties: HashMap::new(),
        })
    }
}

/// PDF 文件处理器（模拟实现）
pub struct PdfProcessor;

impl PdfProcessor {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl DocumentProcessor for PdfProcessor {
    async fn extract_text(&self, file_path: &str) -> Result<ExtractedText, AiStudioError> {
        // 注意：这是一个模拟实现
        // 在实际项目中，你需要使用如 pdf-extract 或 poppler 等库
        
        debug!("模拟 PDF 文本提取: {}", file_path);
        
        let file_metadata = tokio::fs::metadata(file_path).await
            .map_err(|e| AiStudioError::file_processing_with_name(
                format!("读取 PDF 文件失败: {}", e),
                file_path
            ))?;
        
        // 模拟提取的文本内容
        let content = format!("这是从 PDF 文件 {} 提取的模拟文本内容。\n\n在实际实现中，这里会包含真实的 PDF 文本内容。", file_path);
        
        let metadata = DocumentMetadata {
            title: Path::new(file_path).file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.to_string()),
            author: Some("未知作者".to_string()),
            subject: None,
            keywords: None,
            created_date: file_metadata.created().ok()
                .map(|t| chrono::DateTime::from(t)),
            modified_date: file_metadata.modified().ok()
                .map(|t| chrono::DateTime::from(t)),
            page_count: Some(1), // 模拟页数
            word_count: Some(content.split_whitespace().count() as u32),
            language: Some("zh-CN".to_string()),
            format: "application/pdf".to_string(),
            file_size: file_metadata.len(),
            custom_properties: HashMap::new(),
        };
        
        // 模拟页面内容
        let pages = vec![PageContent {
            page_number: 1,
            content: content.clone(),
            images: Vec::new(),
            tables: Vec::new(),
        }];
        
        Ok(ExtractedText {
            content,
            metadata,
            pages: Some(pages),
            processing_info: ProcessingInfo {
                processor_type: "pdf".to_string(),
                processing_time_ms: 0,
                success: true,
                warnings: vec!["这是模拟的 PDF 处理器".to_string()],
                errors: Vec::new(),
            },
        })
    }
    
    fn supports_format(&self, file_extension: &str) -> bool {
        matches!(file_extension.to_lowercase().as_str(), "pdf")
    }
    
    fn supported_formats(&self) -> Vec<String> {
        vec!["pdf".to_string()]
    }
}

/// Word 文档处理器（模拟实现）
pub struct WordProcessor;

impl WordProcessor {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl DocumentProcessor for WordProcessor {
    async fn extract_text(&self, file_path: &str) -> Result<ExtractedText, AiStudioError> {
        // 注意：这是一个模拟实现
        // 在实际项目中，你需要使用如 docx-rs 或调用外部工具
        
        debug!("模拟 Word 文档文本提取: {}", file_path);
        
        let file_metadata = tokio::fs::metadata(file_path).await
            .map_err(|e| AiStudioError::file_processing_with_name(
                format!("读取 Word 文档失败: {}", e),
                file_path
            ))?;
        
        // 模拟提取的文本内容
        let content = format!("这是从 Word 文档 {} 提取的模拟文本内容。\n\n在实际实现中，这里会包含真实的 Word 文档文本内容，包括段落、表格和其他格式化内容。", file_path);
        
        let metadata = DocumentMetadata {
            title: Path::new(file_path).file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.to_string()),
            author: Some("文档作者".to_string()),
            subject: Some("Word 文档".to_string()),
            keywords: Some(vec!["word".to_string(), "document".to_string()]),
            created_date: file_metadata.created().ok()
                .map(|t| chrono::DateTime::from(t)),
            modified_date: file_metadata.modified().ok()
                .map(|t| chrono::DateTime::from(t)),
            page_count: Some(1),
            word_count: Some(content.split_whitespace().count() as u32),
            language: Some("zh-CN".to_string()),
            format: if file_path.ends_with(".docx") {
                "application/vnd.openxmlformats-officedocument.wordprocessingml.document".to_string()
            } else {
                "application/msword".to_string()
            },
            file_size: file_metadata.len(),
            custom_properties: HashMap::new(),
        };
        
        Ok(ExtractedText {
            content,
            metadata,
            pages: None,
            processing_info: ProcessingInfo {
                processor_type: "word".to_string(),
                processing_time_ms: 0,
                success: true,
                warnings: vec!["这是模拟的 Word 处理器".to_string()],
                errors: Vec::new(),
            },
        })
    }
    
    fn supports_format(&self, file_extension: &str) -> bool {
        matches!(file_extension.to_lowercase().as_str(), "doc" | "docx")
    }
    
    fn supported_formats(&self) -> Vec<String> {
        vec!["doc".to_string(), "docx".to_string()]
    }
}

/// HTML 文件处理器
pub struct HtmlProcessor;

impl HtmlProcessor {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl DocumentProcessor for HtmlProcessor {
    async fn extract_text(&self, file_path: &str) -> Result<ExtractedText, AiStudioError> {
        let content = tokio::fs::read_to_string(file_path).await
            .map_err(|e| AiStudioError::file_processing_with_name(
                format!("读取 HTML 文件失败: {}", e),
                file_path
            ))?;
        
        // 简单的 HTML 标签移除
        let plain_text = self.html_to_text(&content);
        let metadata = self.extract_metadata(file_path, &content).await?;
        
        Ok(ExtractedText {
            content: plain_text,
            metadata,
            pages: None,
            processing_info: ProcessingInfo {
                processor_type: "html".to_string(),
                processing_time_ms: 0,
                success: true,
                warnings: Vec::new(),
                errors: Vec::new(),
            },
        })
    }
    
    fn supports_format(&self, file_extension: &str) -> bool {
        matches!(file_extension.to_lowercase().as_str(), "html" | "htm")
    }
    
    fn supported_formats(&self) -> Vec<String> {
        vec!["html".to_string(), "htm".to_string()]
    }
}

impl HtmlProcessor {
    fn html_to_text(&self, html: &str) -> String {
        // 简单的 HTML 标签移除
        let tag_regex = regex::Regex::new(r"<[^>]*>").unwrap();
        let text = tag_regex.replace_all(html, " ");
        
        // 清理多余的空白字符
        let whitespace_regex = regex::Regex::new(r"\s+").unwrap();
        whitespace_regex.replace_all(&text, " ").trim().to_string()
    }
    
    async fn extract_metadata(&self, file_path: &str, content: &str) -> Result<DocumentMetadata, AiStudioError> {
        let file_metadata = tokio::fs::metadata(file_path).await
            .map_err(|e| AiStudioError::file_processing(format!("获取文件元数据失败: {}", e)))?;
        
        // 尝试从 HTML 中提取标题
        let title_regex = regex::Regex::new(r"<title[^>]*>([^<]*)</title>").unwrap();
        let title = title_regex.captures(content)
            .and_then(|caps| caps.get(1))
            .map(|m| m.as_str().trim().to_string())
            .or_else(|| {
                Path::new(file_path).file_stem()
                    .and_then(|s| s.to_str())
                    .map(|s| s.to_string())
            });
        
        let plain_text = self.html_to_text(content);
        let word_count = plain_text.split_whitespace().count() as u32;
        
        Ok(DocumentMetadata {
            title,
            author: None,
            subject: None,
            keywords: None,
            created_date: file_metadata.created().ok()
                .map(|t| chrono::DateTime::from(t)),
            modified_date: file_metadata.modified().ok()
                .map(|t| chrono::DateTime::from(t)),
            page_count: Some(1),
            word_count: Some(word_count),
            language: None,
            format: "text/html".to_string(),
            file_size: file_metadata.len(),
            custom_properties: HashMap::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;
    
    #[tokio::test]
    async fn test_text_processor() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "这是一个测试文本文件。\n包含多行内容。").unwrap();
        
        let processor = TextProcessor::new();
        let result = processor.extract_text(temp_file.path().to_str().unwrap()).await;
        
        assert!(result.is_ok());
        let extracted = result.unwrap();
        assert!(extracted.content.contains("测试文本文件"));
        assert_eq!(extracted.metadata.format, "text/plain");
        assert!(extracted.metadata.word_count.unwrap() > 0);
    }
    
    #[tokio::test]
    async fn test_markdown_processor() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "# 标题\n\n这是 **粗体** 文本和 *斜体* 文本。\n\n[链接](http://example.com)").unwrap();
        
        let processor = MarkdownProcessor::new();
        let result = processor.extract_text(temp_file.path().to_str().unwrap()).await;
        
        assert!(result.is_ok());
        let extracted = result.unwrap();
        assert!(extracted.content.contains("标题"));
        assert!(extracted.content.contains("粗体"));
        assert!(!extracted.content.contains("**")); // 标记应该被移除
        assert_eq!(extracted.metadata.format, "text/markdown");
    }
    
    #[tokio::test]
    async fn test_html_processor() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "<html><head><title>测试页面</title></head><body><h1>标题</h1><p>段落内容</p></body></html>").unwrap();
        
        let processor = HtmlProcessor::new();
        let result = processor.extract_text(temp_file.path().to_str().unwrap()).await;
        
        assert!(result.is_ok());
        let extracted = result.unwrap();
        assert!(extracted.content.contains("标题"));
        assert!(extracted.content.contains("段落内容"));
        assert!(!extracted.content.contains("<h1>")); // HTML 标签应该被移除
        assert_eq!(extracted.metadata.format, "text/html");
        assert_eq!(extracted.metadata.title, Some("测试页面".to_string()));
    }
    
    #[tokio::test]
    async fn test_document_processor_manager() {
        let manager = DocumentProcessorManager::new();
        
        // 测试支持的格式
        assert!(manager.supports_format("txt"));
        assert!(manager.supports_format("md"));
        assert!(manager.supports_format("pdf"));
        assert!(manager.supports_format("docx"));
        assert!(!manager.supports_format("unknown"));
        
        // 测试获取支持的格式
        let formats = manager.get_supported_formats();
        assert!(formats.contains(&"txt".to_string()));
        assert!(formats.contains(&"md".to_string()));
        assert!(formats.contains(&"pdf".to_string()));
    }
    
    #[tokio::test]
    async fn test_unsupported_format() {
        let manager = DocumentProcessorManager::new();
        let result = manager.process_document("test.unknown").await;
        
        assert!(result.is_err());
        if let Err(AiStudioError::FileProcessing { message, .. }) = result {
            assert!(message.contains("不支持的文件格式"));
        }
    }
}