// 配额管理服务
// 处理租户配额检查、使用统计和限制管理

use sea_orm::{DatabaseConnection, EntityTrait, ColumnTrait, Set, ActiveModelTrait};
use uuid::Uuid;
use chrono::{Utc, Duration, DateTime};
use chrono::Datelike;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument, debug};
use utoipa::ToSchema;

use crate::db::entities::{tenant, prelude::*};
use crate::errors::AiStudioError;

/// 配额类型枚举
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum QuotaType {
    /// 用户数量
    Users,
    /// 知识库数量
    KnowledgeBases,
    /// 文档数量
    Documents,
    /// 存储空间（字节）
    Storage,
    /// 月度 API 调用
    MonthlyApiCalls,
    /// 每日 AI 查询
    DailyAiQueries,
}

/// 配额使用情况
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct QuotaUsage {
    /// 配额类型
    pub quota_type: QuotaType,
    /// 当前使用量
    pub current_usage: u64,
    /// 配额限制
    pub limit: u64,
    /// 使用百分比
    pub usage_percentage: f64,
    /// 是否超限
    pub is_exceeded: bool,
    /// 剩余配额
    pub remaining: u64,
    /// 重置时间（对于时间相关的配额）
    pub reset_time: Option<DateTime<Utc>>,
}

/// 配额检查结果
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct QuotaCheckResult {
    /// 租户 ID
    pub tenant_id: Uuid,
    /// 配额类型
    pub quota_type: QuotaType,
    /// 是否允许操作
    pub allowed: bool,
    /// 当前使用情况
    pub usage: QuotaUsage,
    /// 拒绝原因（如果不允许）
    pub rejection_reason: Option<String>,
}

/// 配额更新请求
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct QuotaUpdateRequest {
    /// 配额类型
    pub quota_type: QuotaType,
    /// 增量（可以为负数表示减少）
    pub delta: i64,
    /// 操作描述
    pub operation: String,
    /// 相关资源 ID
    pub resource_id: Option<Uuid>,
}

/// 配额统计响应
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct QuotaStatsResponse {
    /// 租户 ID
    pub tenant_id: Uuid,
    /// 所有配额使用情况
    pub quotas: Vec<QuotaUsage>,
    /// 总体健康状态
    pub overall_health: QuotaHealth,
    /// 警告信息
    pub warnings: Vec<String>,
    /// 最后更新时间
    pub last_updated: DateTime<Utc>,
}

/// 配额健康状态
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum QuotaHealth {
    /// 健康（使用率 < 80%）
    Healthy,
    /// 警告（使用率 80-95%）
    Warning,
    /// 危险（使用率 95-100%）
    Critical,
    /// 超限（使用率 > 100%）
    Exceeded,
}

/// 配额管理服务
pub struct QuotaService {
    db: DatabaseConnection,
}

impl QuotaService {
    /// 创建新的配额服务实例
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// 检查配额是否允许操作
    #[instrument(skip(self))]
    pub async fn check_quota(
        &self,
        tenant_id: Uuid,
        quota_type: QuotaType,
        requested_amount: u64,
    ) -> Result<QuotaCheckResult, AiStudioError> {
        debug!(
            tenant_id = %tenant_id,
            quota_type = ?quota_type,
            requested_amount = requested_amount,
            "检查配额"
        );

        let tenant = self.get_tenant(tenant_id).await?;
        let usage = self.get_quota_usage(&tenant, &quota_type).await?;

        let allowed = !usage.is_exceeded && (usage.current_usage + requested_amount) <= usage.limit;
        let rejection_reason = if !allowed {
            Some(format!(
                "配额超限：当前使用 {} + 请求 {} > 限制 {}",
                usage.current_usage, requested_amount, usage.limit
            ))
        } else {
            None
        };

        Ok(QuotaCheckResult {
            tenant_id,
            quota_type,
            allowed,
            usage,
            rejection_reason,
        })
    }

    /// 更新配额使用量
    #[instrument(skip(self))]
    pub async fn update_quota_usage(
        &self,
        tenant_id: Uuid,
        request: QuotaUpdateRequest,
    ) -> Result<QuotaUsage, AiStudioError> {
        info!(
            tenant_id = %tenant_id,
            quota_type = ?request.quota_type,
            delta = request.delta,
            operation = %request.operation,
            "更新配额使用量"
        );

        let tenant = self.get_tenant(tenant_id).await?;
        let mut usage_stats = tenant.get_usage_stats()
            .map_err(|e| AiStudioError::internal(format!("解析使用统计失败: {}", e)))?;

        // 更新对应的使用量
        match request.quota_type {
            QuotaType::Users => {
                usage_stats.current_users = (usage_stats.current_users as i64 + request.delta).max(0) as u32;
            }
            QuotaType::KnowledgeBases => {
                usage_stats.current_knowledge_bases = (usage_stats.current_knowledge_bases as i64 + request.delta).max(0) as u32;
            }
            QuotaType::Documents => {
                usage_stats.current_documents = (usage_stats.current_documents as i64 + request.delta).max(0) as u32;
            }
            QuotaType::Storage => {
                usage_stats.current_storage_bytes = (usage_stats.current_storage_bytes as i64 + request.delta).max(0) as u64;
            }
            QuotaType::MonthlyApiCalls => {
                usage_stats.monthly_api_calls = (usage_stats.monthly_api_calls as i64 + request.delta).max(0) as u32;
            }
            QuotaType::DailyAiQueries => {
                usage_stats.daily_ai_queries = (usage_stats.daily_ai_queries as i64 + request.delta).max(0) as u32;
            }
        }

        usage_stats.last_updated = Utc::now().into();

        // 更新数据库
        let mut active_tenant: tenant::ActiveModel = tenant.into();
        active_tenant.usage_stats = Set(serde_json::to_value(&usage_stats)
            .map_err(|e| AiStudioError::internal(format!("序列化使用统计失败: {}", e)))?);
        active_tenant.updated_at = Set(Utc::now().into());

        let updated_tenant = active_tenant.update(&self.db).await?;

        // 返回更新后的配额使用情况
        self.get_quota_usage(&updated_tenant, &request.quota_type).await
    }

    /// 获取租户所有配额统计
    #[instrument(skip(self))]
    pub async fn get_quota_stats(&self, tenant_id: Uuid) -> Result<QuotaStatsResponse, AiStudioError> {
        let tenant = self.get_tenant(tenant_id).await?;
        
        let quota_types = vec![
            QuotaType::Users,
            QuotaType::KnowledgeBases,
            QuotaType::Documents,
            QuotaType::Storage,
            QuotaType::MonthlyApiCalls,
            QuotaType::DailyAiQueries,
        ];

        let mut quotas = Vec::new();
        let mut warnings = Vec::new();
        let mut max_usage_percentage: f32 = 0.0;

        for quota_type in quota_types {
            let usage = self.get_quota_usage(&tenant, &quota_type).await?;
            
            // 检查是否需要警告
            if usage.usage_percentage >= 80.0 && usage.usage_percentage < 95.0 {
                warnings.push(format!("{:?} 使用率已达 {:.1}%", quota_type, usage.usage_percentage));
            } else if usage.usage_percentage >= 95.0 {
                warnings.push(format!("{:?} 使用率危险：{:.1}%", quota_type, usage.usage_percentage));
            }

            max_usage_percentage = max_usage_percentage.max(usage.usage_percentage as f32);
            quotas.push(usage);
        }

        let overall_health = if max_usage_percentage > 100.0 {
            QuotaHealth::Exceeded
        } else if max_usage_percentage >= 95.0 {
            QuotaHealth::Critical
        } else if max_usage_percentage >= 80.0 {
            QuotaHealth::Warning
        } else {
            QuotaHealth::Healthy
        };

        let usage_stats = tenant.get_usage_stats()
            .map_err(|e| AiStudioError::internal(format!("解析使用统计失败: {}", e)))?;

        Ok(QuotaStatsResponse {
            tenant_id,
            quotas,
            overall_health,
            warnings,
            last_updated: usage_stats.last_updated.into(),
        })
    }

    /// 重置时间相关的配额
    #[instrument(skip(self))]
    pub async fn reset_time_based_quotas(&self, tenant_id: Uuid) -> Result<(), AiStudioError> {
        info!(tenant_id = %tenant_id, "重置时间相关配额");

        let tenant = self.get_tenant(tenant_id).await?;
        let mut usage_stats = tenant.get_usage_stats()
            .map_err(|e| AiStudioError::internal(format!("解析使用统计失败: {}", e)))?;

        let now = Utc::now();
        
        // 检查是否需要重置月度 API 调用
        let last_updated: DateTime<Utc> = usage_stats.last_updated.into();
        if now.month() != last_updated.month() || now.year() != last_updated.year() {
            usage_stats.monthly_api_calls = 0;
            info!(tenant_id = %tenant_id, "重置月度 API 调用配额");
        }

        // 检查是否需要重置每日 AI 查询
        if now.date_naive() != last_updated.date_naive() {
            usage_stats.daily_ai_queries = 0;
            info!(tenant_id = %tenant_id, "重置每日 AI 查询配额");
        }

        usage_stats.last_updated = now.into();

        // 更新数据库
        let mut active_tenant: tenant::ActiveModel = tenant.into();
        active_tenant.usage_stats = Set(serde_json::to_value(&usage_stats)
            .map_err(|e| AiStudioError::internal(format!("序列化使用统计失败: {}", e)))?);
        active_tenant.updated_at = Set(now.into());

        active_tenant.update(&self.db).await?;
        Ok(())
    }

    /// 批量检查多个配额
    #[instrument(skip(self))]
    pub async fn batch_check_quotas(
        &self,
        tenant_id: Uuid,
        checks: Vec<(QuotaType, u64)>,
    ) -> Result<Vec<QuotaCheckResult>, AiStudioError> {
        let mut results = Vec::new();
        
        for (quota_type, requested_amount) in checks {
            let result = self.check_quota(tenant_id, quota_type, requested_amount).await?;
            results.push(result);
        }

        Ok(results)
    }

    /// 获取配额使用趋势（需要历史数据支持）
    #[instrument(skip(self))]
    pub async fn get_quota_trends(
        &self,
        tenant_id: Uuid,
        quota_type: QuotaType,
        days: u32,
    ) -> Result<Vec<(DateTime<Utc>, u64)>, AiStudioError> {
        // 这里应该从历史数据表中查询趋势数据
        // 为了简化，返回当前数据点
        let tenant = self.get_tenant(tenant_id).await?;
        let usage = self.get_quota_usage(&tenant, &quota_type).await?;
        
        Ok(vec![(Utc::now(), usage.current_usage)])
    }

    // 私有辅助方法

    /// 获取租户信息
    async fn get_tenant(&self, tenant_id: Uuid) -> Result<tenant::Model, AiStudioError> {
        Tenant::find_by_id(tenant_id)
            .one(&self.db)
            .await?
            .ok_or_else(|| AiStudioError::not_found("租户"))
    }

    /// 获取特定配额的使用情况
    async fn get_quota_usage(
        &self,
        tenant: &tenant::Model,
        quota_type: &QuotaType,
    ) -> Result<QuotaUsage, AiStudioError> {
        let limits = tenant.get_quota_limits()
            .map_err(|e| AiStudioError::internal(format!("解析配额限制失败: {}", e)))?;
        let stats = tenant.get_usage_stats()
            .map_err(|e| AiStudioError::internal(format!("解析使用统计失败: {}", e)))?;

        let (current_usage, limit, reset_time) = match quota_type {
            QuotaType::Users => (stats.current_users as u64, limits.max_users as u64, None),
            QuotaType::KnowledgeBases => (stats.current_knowledge_bases as u64, limits.max_knowledge_bases as u64, None),
            QuotaType::Documents => (stats.current_documents as u64, limits.max_documents as u64, None),
            QuotaType::Storage => (stats.current_storage_bytes, limits.max_storage_bytes, None),
            QuotaType::MonthlyApiCalls => {
                let next_month = Utc::now().with_day(1).unwrap() + Duration::days(32);
                let reset_time = next_month.with_day(1).unwrap();
                (stats.monthly_api_calls as u64, limits.monthly_api_calls as u64, Some(reset_time))
            },
            QuotaType::DailyAiQueries => {
                let tomorrow = Utc::now().date_naive() + chrono::Duration::days(1);
                let reset_time = tomorrow.and_hms_opt(0, 0, 0).unwrap().and_utc();
                (stats.daily_ai_queries as u64, limits.daily_ai_queries as u64, Some(reset_time))
            },
        };

        let usage_percentage = if limit > 0 {
            (current_usage as f64 / limit as f64) * 100.0
        } else {
            0.0
        };

        let is_exceeded = current_usage > limit;
        let remaining = if current_usage < limit {
            limit - current_usage
        } else {
            0
        };

        Ok(QuotaUsage {
            quota_type: quota_type.clone(),
            current_usage,
            limit,
            usage_percentage,
            is_exceeded,
            remaining,
            reset_time,
        })
    }
}

/// 配额服务工厂
pub struct QuotaServiceFactory;

impl QuotaServiceFactory {
    /// 创建配额服务实例
    pub fn create(db: DatabaseConnection) -> QuotaService {
        QuotaService::new(db)
    }
}