// 问答质量评估和反馈服务
// 实现答案质量评估、用户反馈收集和答案缓存优化

use std::sync::Arc;
use std::collections::HashMap;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{info, warn, error, debug};
use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait, QueryOrder, ActiveModelTrait};

use crate::errors::AiStudioError;
use crate::ai::rag_engine::{RagQueryResponse, RetrievedChunk, SourceDocument};

/// 答案质量评估器
pub struct QualityAssessment {
    /// 置信度分数 (0.0-1.0)
    pub confidence_score: f32,
    /// 质量分数 (0.0-1.0)
    pub quality_score: f32,
    /// 相关性分数 (0.0-1.0)
    pub relevance_score: f32,
    /// 完整性分数 (0.0-1.0)
    pub completeness_score: f32,
    /// 准确性分数 (0.0-1.0)
    pub accuracy_score: f32,
    /// 评估详情
    pub assessment_details: AssessmentDetails,
}

/// 评估详情
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssessmentDetails {
    /// 来源文档数量
    pub source_count: u32,
    /// 使用的文档块数量
    pub chunk_count: u32,
    /// 平均相似度分数
    pub avg_similarity: f32,
    /// 答案长度
    pub answer_length: usize,
    /// 关键词匹配度
    pub keyword_match_ratio: f32,
    /// 不确定性指标
    pub uncertainty_indicators: Vec<String>,
    /// 质量标签
    pub quality_tags: Vec<String>,
}

/// 用户反馈数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserFeedback {
    /// 反馈 ID
    pub feedback_id: Uuid,
    /// 查询 ID
    pub query_id: String,
    /// 租户 ID
    pub tenant_id: Uuid,
    /// 用户 ID
    pub user_id: Uuid,
    /// 反馈类型
    pub feedback_type: FeedbackType,
    /// 评分 (1-5)
    pub rating: Option<u8>,
    /// 反馈内容
    pub comment: Option<String>,
    /// 是否有用
    pub helpful: Option<bool>,
    /// 反馈时间
    pub created_at: DateTime<Utc>,
    /// 元数据
    pub metadata: serde_json::Value,
}

/// 反馈类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FeedbackType {
    Helpful,
    NotHelpful,
    Incorrect,
    Incomplete,
    Irrelevant,
    Other,
}

/// 答案缓存项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedAnswer {
    /// 缓存 ID
    pub cache_id: String,
    /// 问题哈希
    pub question_hash: String,
    /// 知识库 ID
    pub knowledge_base_id: Option<Uuid>,
    /// 租户 ID
    pub tenant_id: Uuid,
    /// 缓存的答案
    pub answer: String,
    /// 置信度分数
    pub confidence_score: f32,
    /// 来源信息
    pub sources: Vec<CachedSource>,
    /// 缓存时间
    pub cached_at: DateTime<Utc>,
    /// 过期时间
    pub expires_at: DateTime<Utc>,
    /// 访问次数
    pub access_count: u32,
    /// 最后访问时间
    pub last_accessed: DateTime<Utc>,
    /// 用户反馈统计
    pub feedback_stats: FeedbackStats,
}

/// 缓存的来源信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedSource {
    /// 文档 ID
    pub document_id: Uuid,
    /// 文档标题
    pub title: String,
    /// 相关性分数
    pub relevance_score: f32,
}

/// 反馈统计
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FeedbackStats {
    /// 总反馈数
    pub total_feedback: u32,
    /// 正面反馈数
    pub positive_feedback: u32,
    /// 负面反馈数
    pub negative_feedback: u32,
    /// 平均评分
    pub average_rating: f32,
    /// 有用率
    pub helpfulness_ratio: f32,
}

/// 质量评估服务
pub struct QaQualityService {
    /// 数据库连接
    db: Arc<DatabaseConnection>,
    /// 缓存存储 (简单内存缓存，实际应用中可使用 Redis)
    cache: Arc<tokio::sync::RwLock<HashMap<String, CachedAnswer>>>,
    /// 反馈存储
    feedback_store: Arc<tokio::sync::RwLock<HashMap<String, Vec<UserFeedback>>>>,
}

impl QaQualityService {
    /// 创建新的质量评估服务
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self {
            db,
            cache: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            feedback_store: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }
}    ///
 评估答案质量
    pub async fn assess_answer_quality(
        &self,
        question: &str,
        rag_response: &RagQueryResponse,
    ) -> Result<QualityAssessment, AiStudioError> {
        debug!("评估答案质量: query_id={}", rag_response.query_id);
        
        let assessment_details = self.analyze_response_details(question, rag_response).await?;
        
        // 计算各项质量分数
        let confidence_score = rag_response.confidence_score;
        let relevance_score = self.calculate_relevance_score(question, rag_response);
        let completeness_score = self.calculate_completeness_score(rag_response);
        let accuracy_score = self.calculate_accuracy_score(rag_response);
        
        // 计算综合质量分数
        let quality_score = (confidence_score + relevance_score + completeness_score + accuracy_score) / 4.0;
        
        let assessment = QualityAssessment {
            confidence_score,
            quality_score,
            relevance_score,
            completeness_score,
            accuracy_score,
            assessment_details,
        };
        
        debug!("质量评估完成: 综合分数={:.2}", quality_score);
        Ok(assessment)
    }
    
    /// 分析响应详情
    async fn analyze_response_details(
        &self,
        question: &str,
        rag_response: &RagQueryResponse,
    ) -> Result<AssessmentDetails, AiStudioError> {
        let source_count = rag_response.source_documents.len() as u32;
        let chunk_count = rag_response.retrieved_chunks.len() as u32;
        
        // 计算平均相似度
        let avg_similarity = if !rag_response.retrieved_chunks.is_empty() {
            rag_response.retrieved_chunks.iter()
                .map(|chunk| chunk.similarity_score)
                .sum::<f32>() / chunk_count as f32
        } else {
            0.0
        };
        
        let answer_length = rag_response.answer.len();
        let keyword_match_ratio = self.calculate_keyword_match_ratio(question, &rag_response.answer);
        let uncertainty_indicators = self.detect_uncertainty_indicators(&rag_response.answer);
        let quality_tags = self.generate_quality_tags(rag_response);
        
        Ok(AssessmentDetails {
            source_count,
            chunk_count,
            avg_similarity,
            answer_length,
            keyword_match_ratio,
            uncertainty_indicators,
            quality_tags,
        })
    }
    
    /// 计算相关性分数
    fn calculate_relevance_score(&self, question: &str, rag_response: &RagQueryResponse) -> f32 {
        // 基于检索到的文档块的相似度分数
        if rag_response.retrieved_chunks.is_empty() {
            return 0.0;
        }
        
        let max_similarity = rag_response.retrieved_chunks.iter()
            .map(|chunk| chunk.similarity_score)
            .fold(0.0f32, |acc, score| acc.max(score));
        
        // 结合关键词匹配度
        let keyword_match = self.calculate_keyword_match_ratio(question, &rag_response.answer);
        
        (max_similarity + keyword_match) / 2.0
    }
    
    /// 计算完整性分数
    fn calculate_completeness_score(&self, rag_response: &RagQueryResponse) -> f32 {
        let answer_length = rag_response.answer.len();
        let source_count = rag_response.source_documents.len();
        
        // 基于答案长度和来源数量的完整性评估
        let length_score = if answer_length > 100 {
            1.0
        } else if answer_length > 50 {
            0.7
        } else if answer_length > 20 {
            0.5
        } else {
            0.2
        };
        
        let source_score = if source_count >= 3 {
            1.0
        } else if source_count >= 2 {
            0.8
        } else if source_count >= 1 {
            0.6
        } else {
            0.0
        };
        
        (length_score + source_score) / 2.0
    }
    
    /// 计算准确性分数
    fn calculate_accuracy_score(&self, rag_response: &RagQueryResponse) -> f32 {
        // 基于不确定性指标和来源质量
        let uncertainty_indicators = self.detect_uncertainty_indicators(&rag_response.answer);
        let uncertainty_penalty = uncertainty_indicators.len() as f32 * 0.1;
        
        let base_score = rag_response.confidence_score;
        (base_score - uncertainty_penalty).max(0.0)
    }
    
    /// 计算关键词匹配率
    fn calculate_keyword_match_ratio(&self, question: &str, answer: &str) -> f32 {
        let question_words: Vec<&str> = question.split_whitespace()
            .filter(|word| word.len() > 2) // 过滤短词
            .collect();
        
        if question_words.is_empty() {
            return 0.0;
        }
        
        let answer_lower = answer.to_lowercase();
        let matched_count = question_words.iter()
            .filter(|word| answer_lower.contains(&word.to_lowercase()))
            .count();
        
        matched_count as f32 / question_words.len() as f32
    }
    
    /// 检测不确定性指标
    fn detect_uncertainty_indicators(&self, answer: &str) -> Vec<String> {
        let mut indicators = Vec::new();
        let answer_lower = answer.to_lowercase();
        
        let uncertainty_phrases = vec![
            "不确定", "可能", "也许", "大概", "似乎", "好像",
            "我不知道", "无法确定", "不清楚", "难以确定",
            "uncertain", "maybe", "perhaps", "possibly", "might",
            "i don't know", "not sure", "unclear"
        ];
        
        for phrase in uncertainty_phrases {
            if answer_lower.contains(phrase) {
                indicators.push(phrase.to_string());
            }
        }
        
        indicators
    }
    
    /// 生成质量标签
    fn generate_quality_tags(&self, rag_response: &RagQueryResponse) -> Vec<String> {
        let mut tags = Vec::new();
        
        if rag_response.confidence_score >= 0.9 {
            tags.push("高置信度".to_string());
        } else if rag_response.confidence_score >= 0.7 {
            tags.push("中等置信度".to_string());
        } else {
            tags.push("低置信度".to_string());
        }
        
        if rag_response.source_documents.len() >= 3 {
            tags.push("多来源支持".to_string());
        } else if rag_response.source_documents.len() >= 1 {
            tags.push("有来源支持".to_string());
        } else {
            tags.push("无来源支持".to_string());
        }
        
        if rag_response.answer.len() > 200 {
            tags.push("详细回答".to_string());
        } else if rag_response.answer.len() > 50 {
            tags.push("简洁回答".to_string());
        } else {
            tags.push("简短回答".to_string());
        }
        
        tags
    }
    
    /// 收集用户反馈
    pub async fn collect_feedback(
        &self,
        query_id: String,
        tenant_id: Uuid,
        user_id: Uuid,
        feedback_type: FeedbackType,
        rating: Option<u8>,
        comment: Option<String>,
        helpful: Option<bool>,
    ) -> Result<Uuid, AiStudioError> {
        let feedback_id = Uuid::new_v4();
        
        let feedback = UserFeedback {
            feedback_id,
            query_id: query_id.clone(),
            tenant_id,
            user_id,
            feedback_type,
            rating,
            comment,
            helpful,
            created_at: Utc::now(),
            metadata: serde_json::json!({}),
        };
        
        // 存储反馈
        {
            let mut store = self.feedback_store.write().await;
            store.entry(query_id.clone())
                .or_insert_with(Vec::new)
                .push(feedback);
        }
        
        // 更新缓存中的反馈统计
        self.update_cache_feedback_stats(&query_id).await?;
        
        info!("用户反馈已收集: feedback_id={}, query_id={}", feedback_id, query_id);
        Ok(feedback_id)
    }
    
    /// 更新缓存的反馈统计
    async fn update_cache_feedback_stats(&self, query_id: &str) -> Result<(), AiStudioError> {
        let feedbacks = {
            let store = self.feedback_store.read().await;
            store.get(query_id).cloned().unwrap_or_default()
        };
        
        if feedbacks.is_empty() {
            return Ok(());
        }
        
        let total_feedback = feedbacks.len() as u32;
        let positive_feedback = feedbacks.iter()
            .filter(|f| matches!(f.feedback_type, FeedbackType::Helpful) || f.helpful == Some(true))
            .count() as u32;
        let negative_feedback = feedbacks.iter()
            .filter(|f| matches!(f.feedback_type, FeedbackType::NotHelpful | FeedbackType::Incorrect | FeedbackType::Incomplete | FeedbackType::Irrelevant) || f.helpful == Some(false))
            .count() as u32;
        
        let average_rating = if !feedbacks.is_empty() {
            let total_rating: u32 = feedbacks.iter()
                .filter_map(|f| f.rating)
                .map(|r| r as u32)
                .sum();
            let rating_count = feedbacks.iter().filter(|f| f.rating.is_some()).count();
            if rating_count > 0 {
                total_rating as f32 / rating_count as f32
            } else {
                0.0
            }
        } else {
            0.0
        };
        
        let helpfulness_ratio = if total_feedback > 0 {
            positive_feedback as f32 / total_feedback as f32
        } else {
            0.0
        };
        
        let stats = FeedbackStats {
            total_feedback,
            positive_feedback,
            negative_feedback,
            average_rating,
            helpfulness_ratio,
        };
        
        // 更新缓存中的统计信息
        {
            let mut cache = self.cache.write().await;
            for cached_answer in cache.values_mut() {
                if cached_answer.cache_id == *query_id {
                    cached_answer.feedback_stats = stats.clone();
                    break;
                }
            }
        }
        
        Ok(())
    }
}    /// 缓存
答案
    pub async fn cache_answer(
        &self,
        question: &str,
        knowledge_base_id: Option<Uuid>,
        tenant_id: Uuid,
        rag_response: &RagQueryResponse,
        quality_assessment: &QualityAssessment,
    ) -> Result<String, AiStudioError> {
        // 只缓存高质量的答案
        if quality_assessment.quality_score < 0.7 {
            debug!("答案质量不足，不进行缓存: score={:.2}", quality_assessment.quality_score);
            return Ok("not_cached".to_string());
        }
        
        let question_hash = format!("{:x}", md5::compute(question));
        let cache_id = format!("cache_{}_{}", tenant_id, question_hash);
        
        let sources: Vec<CachedSource> = rag_response.source_documents.iter()
            .map(|doc| CachedSource {
                document_id: doc.document_id,
                title: doc.title.clone(),
                relevance_score: doc.relevance_score,
            })
            .collect();
        
        let cached_answer = CachedAnswer {
            cache_id: cache_id.clone(),
            question_hash,
            knowledge_base_id,
            tenant_id,
            answer: rag_response.answer.clone(),
            confidence_score: rag_response.confidence_score,
            sources,
            cached_at: Utc::now(),
            expires_at: Utc::now() + chrono::Duration::hours(24), // 24小时过期
            access_count: 0,
            last_accessed: Utc::now(),
            feedback_stats: FeedbackStats::default(),
        };
        
        // 存储到缓存
        {
            let mut cache = self.cache.write().await;
            cache.insert(cache_id.clone(), cached_answer);
        }
        
        info!("答案已缓存: cache_id={}, 质量分数={:.2}", cache_id, quality_assessment.quality_score);
        Ok(cache_id)
    }
    
    /// 从缓存获取答案
    pub async fn get_cached_answer(
        &self,
        question: &str,
        knowledge_base_id: Option<Uuid>,
        tenant_id: Uuid,
    ) -> Option<CachedAnswer> {
        let question_hash = format!("{:x}", md5::compute(question));
        let cache_id = format!("cache_{}_{}", tenant_id, question_hash);
        
        let mut cache = self.cache.write().await;
        if let Some(cached_answer) = cache.get_mut(&cache_id) {
            // 检查是否过期
            if cached_answer.expires_at < Utc::now() {
                cache.remove(&cache_id);
                debug!("缓存已过期: cache_id={}", cache_id);
                return None;
            }
            
            // 检查知识库匹配
            if cached_answer.knowledge_base_id != knowledge_base_id {
                return None;
            }
            
            // 更新访问统计
            cached_answer.access_count += 1;
            cached_answer.last_accessed = Utc::now();
            
            debug!("命中缓存: cache_id={}, 访问次数={}", cache_id, cached_answer.access_count);
            Some(cached_answer.clone())
        } else {
            None
        }
    }
    
    /// 获取反馈统计
    pub async fn get_feedback_stats(&self, query_id: &str) -> FeedbackStats {
        let store = self.feedback_store.read().await;
        if let Some(feedbacks) = store.get(query_id) {
            self.calculate_feedback_stats(feedbacks)
        } else {
            FeedbackStats::default()
        }
    }
    
    /// 计算反馈统计
    fn calculate_feedback_stats(&self, feedbacks: &[UserFeedback]) -> FeedbackStats {
        if feedbacks.is_empty() {
            return FeedbackStats::default();
        }
        
        let total_feedback = feedbacks.len() as u32;
        let positive_feedback = feedbacks.iter()
            .filter(|f| matches!(f.feedback_type, FeedbackType::Helpful) || f.helpful == Some(true))
            .count() as u32;
        let negative_feedback = feedbacks.iter()
            .filter(|f| matches!(f.feedback_type, FeedbackType::NotHelpful | FeedbackType::Incorrect | FeedbackType::Incomplete | FeedbackType::Irrelevant) || f.helpful == Some(false))
            .count() as u32;
        
        let average_rating = {
            let ratings: Vec<u8> = feedbacks.iter().filter_map(|f| f.rating).collect();
            if !ratings.is_empty() {
                ratings.iter().map(|&r| r as f32).sum::<f32>() / ratings.len() as f32
            } else {
                0.0
            }
        };
        
        let helpfulness_ratio = positive_feedback as f32 / total_feedback as f32;
        
        FeedbackStats {
            total_feedback,
            positive_feedback,
            negative_feedback,
            average_rating,
            helpfulness_ratio,
        }
    }
    
    /// 清理过期缓存
    pub async fn cleanup_expired_cache(&self) -> u32 {
        let now = Utc::now();
        let mut cache = self.cache.write().await;
        let initial_count = cache.len();
        
        cache.retain(|_, cached_answer| cached_answer.expires_at > now);
        
        let removed_count = initial_count - cache.len();
        if removed_count > 0 {
            info!("清理了 {} 个过期缓存", removed_count);
        }
        
        removed_count as u32
    }
    
    /// 获取缓存统计
    pub async fn get_cache_stats(&self) -> CacheStats {
        let cache = self.cache.read().await;
        let total_cached = cache.len() as u32;
        let total_access_count = cache.values().map(|c| c.access_count).sum();
        
        let now = Utc::now();
        let expired_count = cache.values()
            .filter(|c| c.expires_at < now)
            .count() as u32;
        
        CacheStats {
            total_cached,
            expired_count,
            total_access_count,
            hit_ratio: 0.0, // 需要额外统计命中率
        }
    }
    
    /// 启动定期清理任务
    pub async fn start_cleanup_scheduler(&self) {
        let cache = self.cache.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(3600)); // 每小时清理一次
            
            loop {
                interval.tick().await;
                
                let now = Utc::now();
                let mut cache_guard = cache.write().await;
                let initial_count = cache_guard.len();
                
                cache_guard.retain(|_, cached_answer| cached_answer.expires_at > now);
                
                let removed_count = initial_count - cache_guard.len();
                if removed_count > 0 {
                    info!("定期清理了 {} 个过期缓存", removed_count);
                }
            }
        });
    }
}

/// 缓存统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    /// 总缓存数量
    pub total_cached: u32,
    /// 过期缓存数量
    pub expired_count: u32,
    /// 总访问次数
    pub total_access_count: u32,
    /// 命中率
    pub hit_ratio: f32,
}

/// 质量评估服务工厂
pub struct QaQualityServiceFactory;

impl QaQualityServiceFactory {
    /// 创建质量评估服务实例
    pub async fn create(db: Arc<DatabaseConnection>) -> Arc<QaQualityService> {
        let service = Arc::new(QaQualityService::new(db));
        
        // 启动清理调度器
        service.start_cleanup_scheduler().await;
        
        service
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_keyword_match_ratio() {
        let service = QaQualityService::new(Arc::new(todo!()));
        
        let question = "什么是人工智能";
        let answer = "人工智能是计算机科学的一个分支";
        
        let ratio = service.calculate_keyword_match_ratio(question, answer);
        assert!(ratio > 0.0);
    }
    
    #[test]
    fn test_uncertainty_detection() {
        let service = QaQualityService::new(Arc::new(todo!()));
        
        let answer = "我不确定这个答案是否正确，可能需要更多信息";
        let indicators = service.detect_uncertainty_indicators(answer);
        
        assert!(!indicators.is_empty());
        assert!(indicators.contains(&"不确定".to_string()));
        assert!(indicators.contains(&"可能".to_string()));
    }
    
    #[test]
    fn test_quality_tags_generation() {
        let service = QaQualityService::new(Arc::new(todo!()));
        
        let rag_response = RagQueryResponse {
            query_id: "test".to_string(),
            answer: "这是一个详细的答案，包含了很多有用的信息...".to_string(),
            confidence_score: 0.9,
            retrieved_chunks: Vec::new(),
            source_documents: vec![
                SourceDocument {
                    document_id: Uuid::new_v4(),
                    title: "测试文档".to_string(),
                    doc_type: "text".to_string(),
                    relevance_score: 0.8,
                    chunk_count: 1,
                }
            ],
            query_stats: todo!(),
            generated_at: Utc::now(),
        };
        
        let tags = service.generate_quality_tags(&rag_response);
        assert!(tags.contains(&"高置信度".to_string()));
        assert!(tags.contains(&"有来源支持".to_string()));
    }
}