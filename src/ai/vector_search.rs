// 向量检索和相似度搜索模块
// 实现基于余弦相似度的文档检索和混合检索功能

use crate::ai::{RigAiClientManager, DocumentChunk};
use crate::errors::AiStudioError;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// 向量搜索引擎特征
#[async_trait]
pub trait VectorSearchEngine: Send + Sync {
    /// 添加文档块到索引
    async fn add_chunks(&mut self, chunks: &[DocumentChunk]) -> Result<(), AiStudioError>;
    
    /// 从索引中移除文档块
    async fn remove_chunks(&mut self, chunk_ids: &[Uuid]) -> Result<(), AiStudioError>;
    
    /// 向量相似度搜索
    async fn vector_search(
        &self,
        query_vector: &[f32],
        limit: usize,
        threshold: f32,
        filters: Option<&SearchFilters>,
    ) -> Result<Vec<SearchResult>, AiStudioError>;
    
    /// 文本查询搜索（先向量化再搜索）
    async fn text_search(
        &self,
        query: &str,
        limit: usize,
        threshold: f32,
        filters: Option<&SearchFilters>,
    ) -> Result<Vec<SearchResult>, AiStudioError>;
    
    /// 混合搜索（向量 + 关键词）
    async fn hybrid_search(
        &self,
        query: &str,
        limit: usize,
        vector_weight: f32,
        keyword_weight: f32,
        filters: Option<&SearchFilters>,
    ) -> Result<Vec<SearchResult>, AiStudioError>;
    
    /// 获取索引统计信息
    async fn get_stats(&self) -> Result<IndexStats, AiStudioError>;
}

/// 搜索结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub chunk: DocumentChunk,
    pub score: f32,
    pub rank: usize,
    pub match_type: MatchType,
    pub highlights: Vec<TextHighlight>,
}

/// 匹配类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MatchType {
    Vector,
    Keyword,
    Hybrid,
    Exact,
}

/// 文本高亮
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextHighlight {
    pub start: usize,
    pub end: usize,
    pub text: String,
    pub score: f32,
}

/// 搜索过滤器
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchFilters {
    pub tenant_id: Option<Uuid>,
    pub document_ids: Option<Vec<Uuid>>,
    pub chunk_types: Option<Vec<String>>,
    pub languages: Option<Vec<String>>,
    pub date_range: Option<DateRange>,
    pub metadata_filters: Option<HashMap<String, String>>,
}

/// 日期范围
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateRange {
    pub start: chrono::DateTime<chrono::Utc>,
    pub end: chrono::DateTime<chrono::Utc>,
}

/// 索引统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexStats {
    pub total_chunks: usize,
    pub total_vectors: usize,
    pub index_size_bytes: u64,
    pub last_updated: chrono::DateTime<chrono::Utc>,
    pub search_count: u64,
    pub average_search_time_ms: f64,
}

/// 内存向量搜索引擎实现
pub struct InMemoryVectorSearch {
    chunks: HashMap<Uuid, DocumentChunk>,
    client_manager: RigAiClientManager,
    stats: IndexStats,
}

impl InMemoryVectorSearch {
    /// 创建新的内存向量搜索引擎
    pub fn new(client_manager: RigAiClientManager) -> Self {
        Self {
            chunks: HashMap::new(),
            client_manager,
            stats: IndexStats {
                total_chunks: 0,
                total_vectors: 0,
                index_size_bytes: 0,
                last_updated: chrono::Utc::now(),
                search_count: 0,
                average_search_time_ms: 0.0,
            },
        }
    }
    
    /// 计算余弦相似度
    fn cosine_similarity(&self, a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() {
            return 0.0;
        }
        
        let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        
        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }
        
        dot_product / (norm_a * norm_b)
    }
    
    /// 关键词搜索评分
    fn keyword_score(&self, query: &str, content: &str) -> f32 {
        let query_lower = query.to_lowercase();
        let content_lower = content.to_lowercase();
        
        // 简单的关键词匹配评分
        let query_words: Vec<&str> = query_lower.split_whitespace().collect();
        let content_words: Vec<&str> = content_lower.split_whitespace().collect();
        
        if query_words.is_empty() || content_words.is_empty() {
            return 0.0;
        }
        
        let mut matches = 0;
        for query_word in &query_words {
            if content_words.iter().any(|&word| word.contains(query_word)) {
                matches += 1;
            }
        }
        
        matches as f32 / query_words.len() as f32
    }
    
    /// 应用搜索过滤器
    fn apply_filters(&self, chunk: &DocumentChunk, filters: Option<&SearchFilters>) -> bool {
        if let Some(filters) = filters {
            // 检查租户 ID（这里需要从 chunk 的元数据中获取）
            if let Some(_tenant_id) = &filters.tenant_id {
                // 实际实现中需要检查 chunk 是否属于指定租户
                // 这里暂时跳过
            }
            
            // 检查文档 ID
            if let Some(_document_ids) = &filters.document_ids {
                // 实际实现中需要检查 chunk 是否属于指定文档
                // 这里暂时跳过
            }
            
            // 检查块类型
            if let Some(chunk_types) = &filters.chunk_types {
                let chunk_type_str = format!("{:?}", chunk.metadata.chunk_type);
                if !chunk_types.contains(&chunk_type_str) {
                    return false;
                }
            }
            
            // 检查语言
            if let Some(languages) = &filters.languages {
                if let Some(chunk_lang) = &chunk.metadata.language {
                    if !languages.contains(chunk_lang) {
                        return false;
                    }
                } else {
                    return false;
                }
            }
        }
        
        true
    }
    
    /// 生成文本高亮
    fn generate_highlights(&self, query: &str, content: &str) -> Vec<TextHighlight> {
        let mut highlights = Vec::new();
        let query_lower = query.to_lowercase();
        let content_lower = content.to_lowercase();
        
        // 简单的高亮实现 - 查找查询词在内容中的位置
        for word in query_lower.split_whitespace() {
            if let Some(pos) = content_lower.find(word) {
                highlights.push(TextHighlight {
                    start: pos,
                    end: pos + word.len(),
                    text: content[pos..pos + word.len()].to_string(),
                    score: 1.0,
                });
            }
        }
        
        highlights
    }
    
    /// 更新统计信息
    fn update_stats(&mut self) {
        self.stats.total_chunks = self.chunks.len();
        self.stats.total_vectors = self.chunks.values()
            .filter(|chunk| chunk.embedding.is_some())
            .count();
        self.stats.last_updated = chrono::Utc::now();
        
        // 估算索引大小
        self.stats.index_size_bytes = self.chunks.values()
            .map(|chunk| {
                chunk.content.len() as u64 + 
                chunk.embedding.as_ref().map_or(0, |emb| emb.len() * 4) as u64
            })
            .sum();
    }
}

#[async_trait]
impl VectorSearchEngine for InMemoryVectorSearch {
    async fn add_chunks(&mut self, chunks: &[DocumentChunk]) -> Result<(), AiStudioError> {
        debug!("向索引添加 {} 个文档块", chunks.len());
        
        for chunk in chunks {
            self.chunks.insert(chunk.id, chunk.clone());
        }
        
        self.update_stats();
        info!("成功添加 {} 个文档块到索引", chunks.len());
        
        Ok(())
    }
    
    async fn remove_chunks(&mut self, chunk_ids: &[Uuid]) -> Result<(), AiStudioError> {
        debug!("从索引移除 {} 个文档块", chunk_ids.len());
        
        let mut removed_count = 0;
        for chunk_id in chunk_ids {
            if self.chunks.remove(chunk_id).is_some() {
                removed_count += 1;
            }
        }
        
        self.update_stats();
        info!("成功从索引移除 {} 个文档块", removed_count);
        
        Ok(())
    }
    
    async fn vector_search(
        &self,
        query_vector: &[f32],
        limit: usize,
        threshold: f32,
        filters: Option<&SearchFilters>,
    ) -> Result<Vec<SearchResult>, AiStudioError> {
        let start_time = std::time::Instant::now();
        debug!("执行向量搜索，查询向量维度: {}, 限制: {}, 阈值: {}", 
               query_vector.len(), limit, threshold);
        
        let mut results = Vec::new();
        
        for chunk in self.chunks.values() {
            // 应用过滤器
            if !self.apply_filters(chunk, filters) {
                continue;
            }
            
            // 检查是否有嵌入向量
            if let Some(embedding) = &chunk.embedding {
                let similarity = self.cosine_similarity(query_vector, embedding);
                
                if similarity >= threshold {
                    results.push(SearchResult {
                        chunk: chunk.clone(),
                        score: similarity,
                        rank: 0, // 将在排序后设置
                        match_type: MatchType::Vector,
                        highlights: Vec::new(),
                    });
                }
            }
        }
        
        // 按相似度排序
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        
        // 限制结果数量并设置排名
        results.truncate(limit);
        for (i, result) in results.iter_mut().enumerate() {
            result.rank = i + 1;
        }
        
        let search_time = start_time.elapsed().as_millis() as f64;
        info!("向量搜索完成，找到 {} 个结果，耗时 {}ms", results.len(), search_time);
        
        Ok(results)
    }
    
    async fn text_search(
        &self,
        query: &str,
        limit: usize,
        threshold: f32,
        filters: Option<&SearchFilters>,
    ) -> Result<Vec<SearchResult>, AiStudioError> {
        debug!("执行文本搜索，查询: {}", query);
        
        // 首先将查询文本向量化
        let embedding_response = self.client_manager.generate_embedding(query).await?;
        
        // 然后执行向量搜索
        self.vector_search(&embedding_response.embedding, limit, threshold, filters).await
    }
    
    async fn hybrid_search(
        &self,
        query: &str,
        limit: usize,
        vector_weight: f32,
        keyword_weight: f32,
        filters: Option<&SearchFilters>,
    ) -> Result<Vec<SearchResult>, AiStudioError> {
        let start_time = std::time::Instant::now();
        debug!("执行混合搜索，查询: {}, 向量权重: {}, 关键词权重: {}", 
               query, vector_weight, keyword_weight);
        
        // 生成查询向量
        let embedding_response = self.client_manager.generate_embedding(query).await?;
        let query_vector = &embedding_response.embedding;
        
        let mut results = Vec::new();
        
        for chunk in self.chunks.values() {
            // 应用过滤器
            if !self.apply_filters(chunk, filters) {
                continue;
            }
            
            let mut total_score = 0.0;
            let mut has_vector_score = false;
            
            // 计算向量相似度得分
            if let Some(embedding) = &chunk.embedding {
                let vector_score = self.cosine_similarity(query_vector, embedding);
                total_score += vector_score * vector_weight;
                has_vector_score = true;
            }
            
            // 计算关键词匹配得分
            let keyword_score = self.keyword_score(query, &chunk.content);
            total_score += keyword_score * keyword_weight;
            
            // 只有当至少有一种得分时才添加结果
            if has_vector_score || keyword_score > 0.0 {
                let highlights = self.generate_highlights(query, &chunk.content);
                
                results.push(SearchResult {
                    chunk: chunk.clone(),
                    score: total_score,
                    rank: 0,
                    match_type: MatchType::Hybrid,
                    highlights,
                });
            }
        }
        
        // 按总得分排序
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        
        // 限制结果数量并设置排名
        results.truncate(limit);
        for (i, result) in results.iter_mut().enumerate() {
            result.rank = i + 1;
        }
        
        let search_time = start_time.elapsed().as_millis() as f64;
        info!("混合搜索完成，找到 {} 个结果，耗时 {}ms", results.len(), search_time);
        
        Ok(results)
    }
    
    async fn get_stats(&self) -> Result<IndexStats, AiStudioError> {
        Ok(self.stats.clone())
    }
}

/// 向量搜索服务
pub struct VectorSearchService {
    engine: Box<dyn VectorSearchEngine>,
}

impl VectorSearchService {
    /// 创建新的向量搜索服务
    pub fn new(engine: Box<dyn VectorSearchEngine>) -> Self {
        Self { engine }
    }
    
    /// 创建默认的内存搜索服务
    pub fn create_in_memory(client_manager: RigAiClientManager) -> Self {
        let engine = Box::new(InMemoryVectorSearch::new(client_manager));
        Self::new(engine)
    }
    
    /// 添加文档块
    pub async fn add_chunks(&mut self, chunks: &[DocumentChunk]) -> Result<(), AiStudioError> {
        self.engine.add_chunks(chunks).await
    }
    
    /// 移除文档块
    pub async fn remove_chunks(&mut self, chunk_ids: &[Uuid]) -> Result<(), AiStudioError> {
        self.engine.remove_chunks(chunk_ids).await
    }
    
    /// 搜索相似文档
    pub async fn search(
        &self,
        query: &str,
        options: SearchOptions,
    ) -> Result<SearchResponse, AiStudioError> {
        let start_time = std::time::Instant::now();
        
        let results = match options.search_type {
            SearchType::Vector => {
                // 需要先将查询向量化
                self.engine.text_search(
                    query,
                    options.limit,
                    options.threshold,
                    options.filters.as_ref(),
                ).await?
            }
            SearchType::Keyword => {
                // 纯关键词搜索（这里简化实现）
                self.engine.hybrid_search(
                    query,
                    options.limit,
                    0.0, // 不使用向量权重
                    1.0, // 只使用关键词权重
                    options.filters.as_ref(),
                ).await?
            }
            SearchType::Hybrid => {
                self.engine.hybrid_search(
                    query,
                    options.limit,
                    options.vector_weight.unwrap_or(0.7),
                    options.keyword_weight.unwrap_or(0.3),
                    options.filters.as_ref(),
                ).await?
            }
        };
        
        let search_time = start_time.elapsed().as_millis() as u64;
        
        let total_found = results.len();
        
        Ok(SearchResponse {
            results,
            total_found,
            search_time_ms: search_time,
            query: query.to_string(),
            search_type: options.search_type,
        })
    }
    
    /// 获取索引统计信息
    pub async fn get_stats(&self) -> Result<IndexStats, AiStudioError> {
        self.engine.get_stats().await
    }
}

/// 搜索选项
#[derive(Debug, Clone)]
pub struct SearchOptions {
    pub search_type: SearchType,
    pub limit: usize,
    pub threshold: f32,
    pub vector_weight: Option<f32>,
    pub keyword_weight: Option<f32>,
    pub filters: Option<SearchFilters>,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            search_type: SearchType::Hybrid,
            limit: 10,
            threshold: 0.7,
            vector_weight: Some(0.7),
            keyword_weight: Some(0.3),
            filters: None,
        }
    }
}

/// 搜索类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SearchType {
    Vector,
    Keyword,
    Hybrid,
}

/// 搜索响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    pub results: Vec<SearchResult>,
    pub total_found: usize,
    pub search_time_ms: u64,
    pub query: String,
    pub search_type: SearchType,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::{ChunkMetadata, ChunkPosition, ChunkType};
    use crate::config::AiConfig;
    use std::collections::HashMap;
    
    fn create_test_chunk(id: Uuid, content: &str, embedding: Option<Vec<f32>>) -> DocumentChunk {
        DocumentChunk {
            id,
            content: content.to_string(),
            metadata: ChunkMetadata {
                chunk_index: 0,
                total_chunks: 1,
                word_count: content.split_whitespace().count() as u32,
                character_count: content.len() as u32,
                language: Some("zh-CN".to_string()),
                chunk_type: ChunkType::Text,
                source_page: None,
                overlap_with_previous: false,
                overlap_with_next: false,
                custom_properties: HashMap::new(),
            },
            embedding,
            position: ChunkPosition {
                start_char: 0,
                end_char: content.len(),
                start_line: None,
                end_line: None,
            },
        }
    }
    
    #[tokio::test]
    async fn test_cosine_similarity() {
        let config = AiConfig {
            model_endpoint: "mock://test".to_string(),
            api_key: "test".to_string(),
            max_tokens: 1000,
            temperature: 0.7,
            timeout: 30,
            retry_attempts: 3,
        };
        
        // 注意：在测试环境中可能会失败，因为没有真实的 AI 服务
        let client_manager = match RigAiClientManager::new(config).await {
            Ok(manager) => manager,
            Err(_) => return, // 跳过测试如果无法创建客户端
        };
        let search_engine = InMemoryVectorSearch::new(client_manager);
        
        let vec1 = vec![1.0, 0.0, 0.0];
        let vec2 = vec![1.0, 0.0, 0.0];
        let vec3 = vec![0.0, 1.0, 0.0];
        
        // 相同向量的相似度应该是 1.0
        assert!((search_engine.cosine_similarity(&vec1, &vec2) - 1.0).abs() < 0.001);
        
        // 正交向量的相似度应该是 0.0
        assert!((search_engine.cosine_similarity(&vec1, &vec3) - 0.0).abs() < 0.001);
    }
    
    #[tokio::test]
    async fn test_keyword_score() {
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
        let search_engine = InMemoryVectorSearch::new(client_manager);
        
        let query = "测试 文档";
        let content1 = "这是一个测试文档";
        let content2 = "这是另一个内容";
        
        let score1 = search_engine.keyword_score(query, content1);
        let score2 = search_engine.keyword_score(query, content2);
        
        // content1 应该有更高的关键词匹配分数
        assert!(score1 > score2);
        assert!(score1 > 0.0);
    }
    
    #[tokio::test]
    async fn test_add_and_search_chunks() {
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
        let mut search_engine = InMemoryVectorSearch::new(client_manager);
        
        // 创建测试文档块
        let chunks = vec![
            create_test_chunk(
                Uuid::new_v4(),
                "这是关于人工智能的文档",
                Some(vec![1.0, 0.0, 0.0, 0.0]),
            ),
            create_test_chunk(
                Uuid::new_v4(),
                "这是关于机器学习的文档",
                Some(vec![0.8, 0.6, 0.0, 0.0]),
            ),
            create_test_chunk(
                Uuid::new_v4(),
                "这是关于深度学习的文档",
                Some(vec![0.6, 0.8, 0.0, 0.0]),
            ),
        ];
        
        // 添加文档块到索引
        search_engine.add_chunks(&chunks).await.unwrap();
        
        // 执行向量搜索
        let query_vector = vec![1.0, 0.0, 0.0, 0.0];
        let results = search_engine.vector_search(&query_vector, 10, 0.5, None).await.unwrap();
        
        assert!(!results.is_empty());
        assert!(results[0].score >= 0.5);
        
        // 检查结果是否按分数排序
        for i in 1..results.len() {
            assert!(results[i-1].score >= results[i].score);
        }
    }
    
    #[tokio::test]
    async fn test_vector_search_service() {
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
        let mut service = VectorSearchService::create_in_memory(client_manager);
        
        // 添加测试文档块
        let chunks = vec![
            create_test_chunk(
                Uuid::new_v4(),
                "人工智能是计算机科学的一个分支",
                Some(vec![1.0, 0.0, 0.0]),
            ),
        ];
        
        service.add_chunks(&chunks).await.unwrap();
        
        // 执行搜索
        let options = SearchOptions {
            search_type: SearchType::Hybrid,
            limit: 5,
            threshold: 0.1,
            vector_weight: Some(0.7),
            keyword_weight: Some(0.3),
            filters: None,
        };
        
        let response = service.search("人工智能", options).await.unwrap();
        
        assert!(!response.results.is_empty());
        assert_eq!(response.query, "人工智能");
        assert_eq!(response.search_type, SearchType::Hybrid);
        assert!(response.search_time_ms > 0);
    }
    
    #[tokio::test]
    async fn test_search_filters() {
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
        let search_engine = InMemoryVectorSearch::new(client_manager);
        
        let chunk = create_test_chunk(
            Uuid::new_v4(),
            "测试内容",
            Some(vec![1.0, 0.0, 0.0]),
        );
        
        // 测试语言过滤器
        let filters = SearchFilters {
            tenant_id: None,
            document_ids: None,
            chunk_types: None,
            languages: Some(vec!["zh-CN".to_string()]),
            date_range: None,
            metadata_filters: None,
        };
        
        assert!(search_engine.apply_filters(&chunk, Some(&filters)));
        
        // 测试不匹配的语言过滤器
        let filters = SearchFilters {
            tenant_id: None,
            document_ids: None,
            chunk_types: None,
            languages: Some(vec!["en".to_string()]),
            date_range: None,
            metadata_filters: None,
        };
        
        assert!(!search_engine.apply_filters(&chunk, Some(&filters)));
    }
}