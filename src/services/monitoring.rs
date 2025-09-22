// 监控服务
// 处理资源使用统计、性能监控和告警

use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait, Set, ActiveModelTrait};
use uuid::Uuid;
use chrono::{Utc, Duration, DateTime};
use serde::{Deserialize, Serialize};
use tracing::{info, warn, error, instrument, debug};
use utoipa::ToSchema;
use std::collections::HashMap;

use crate::db::entities::{tenant, prelude::*};
use crate::errors::AiStudioError;
use crate::services::quota::{QuotaService, QuotaType, QuotaHealth};

/// 监控指标类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum MetricType {
    /// API 调用次数
    ApiCalls,
    /// AI 查询次数
    AiQueries,
    /// 存储使用量
    StorageUsage,
    /// 用户活跃度
    UserActivity,
    /// 错误率
    ErrorRate,
    /// 响应时间
    ResponseTime,
    /// 并发连接数
    ConcurrentConnections,
}

/// 监控指标数据点
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MetricDataPoint {
    /// 指标类型
    pub metric_type: MetricType,
    /// 数值
    pub value: f64,
    /// 时间戳
    pub timestamp: DateTime<Utc>,
    /// 标签（用于分组和过滤）
    pub labels: HashMap<String, String>,
}

/// 监控告警规则
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AlertRule {
    /// 规则 ID
    pub id: Uuid,
    /// 规则名称
    pub name: String,
    /// 指标类型
    pub metric_type: MetricType,
    /// 阈值
    pub threshold: f64,
    /// 比较操作符
    pub operator: AlertOperator,
    /// 时间窗口（秒）
    pub window_seconds: u64,
    /// 是否启用
    pub enabled: bool,
    /// 告警级别
    pub severity: AlertSeverity,
    /// 通知渠道
    pub notification_channels: Vec<String>,
}

/// 告警操作符
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum AlertOperator {
    /// 大于
    GreaterThan,
    /// 小于
    LessThan,
    /// 等于
    Equal,
    /// 大于等于
    GreaterThanOrEqual,
    /// 小于等于
    LessThanOrEqual,
}

/// 告警级别
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum AlertSeverity {
    /// 信息
    Info,
    /// 警告
    Warning,
    /// 错误
    Error,
    /// 严重
    Critical,
}

/// 告警事件
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AlertEvent {
    /// 事件 ID
    pub id: Uuid,
    /// 规则 ID
    pub rule_id: Uuid,
    /// 租户 ID
    pub tenant_id: Uuid,
    /// 告警消息
    pub message: String,
    /// 告警级别
    pub severity: AlertSeverity,
    /// 当前值
    pub current_value: f64,
    /// 阈值
    pub threshold: f64,
    /// 触发时间
    pub triggered_at: DateTime<Utc>,
    /// 是否已解决
    pub resolved: bool,
    /// 解决时间
    pub resolved_at: Option<DateTime<Utc>>,
}

/// 系统健康状态
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SystemHealth {
    /// 整体状态
    pub overall_status: HealthStatus,
    /// 各组件状态
    pub components: HashMap<String, ComponentHealth>,
    /// 活跃告警数量
    pub active_alerts: u32,
    /// 最后检查时间
    pub last_check: DateTime<Utc>,
}

/// 健康状态
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum HealthStatus {
    /// 健康
    Healthy,
    /// 警告
    Warning,
    /// 不健康
    Unhealthy,
    /// 未知
    Unknown,
}

/// 组件健康状态
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ComponentHealth {
    /// 状态
    pub status: HealthStatus,
    /// 响应时间（毫秒）
    pub response_time_ms: Option<u64>,
    /// 错误消息
    pub error_message: Option<String>,
    /// 最后检查时间
    pub last_check: DateTime<Utc>,
}

/// 租户使用统计
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TenantUsageStats {
    /// 租户 ID
    pub tenant_id: Uuid,
    /// 统计时间范围
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    /// API 调用统计
    pub api_calls: UsageMetric,
    /// AI 查询统计
    pub ai_queries: UsageMetric,
    /// 存储使用统计
    pub storage_usage: UsageMetric,
    /// 用户活跃统计
    pub active_users: UsageMetric,
    /// 错误率统计
    pub error_rate: UsageMetric,
    /// 平均响应时间
    pub avg_response_time: UsageMetric,
}

/// 使用指标
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UsageMetric {
    /// 当前值
    pub current: f64,
    /// 最大值
    pub max: f64,
    /// 最小值
    pub min: f64,
    /// 平均值
    pub average: f64,
    /// 总计
    pub total: f64,
    /// 变化趋势（百分比）
    pub trend_percentage: f64,
}

/// 监控服务
pub struct MonitoringService {
    db: DatabaseConnection,
    quota_service: QuotaService,
}

impl MonitoringService {
    /// 创建新的监控服务实例
    pub fn new(db: DatabaseConnection) -> Self {
        let quota_service = QuotaService::new(db.clone());
        Self { db, quota_service }
    }

    /// 记录指标数据点
    #[instrument(skip(self))]
    pub async fn record_metric(
        &self,
        tenant_id: Uuid,
        data_point: MetricDataPoint,
    ) -> Result<(), AiStudioError> {
        debug!(
            tenant_id = %tenant_id,
            metric_type = ?data_point.metric_type,
            value = data_point.value,
            "记录监控指标"
        );

        // 这里应该将指标数据存储到时序数据库（如 InfluxDB）或 Redis
        // 为了简化，这里只记录日志
        info!(
            tenant_id = %tenant_id,
            metric_type = ?data_point.metric_type,
            value = data_point.value,
            timestamp = %data_point.timestamp,
            "监控指标已记录"
        );

        // 检查是否触发告警
        self.check_alerts(tenant_id, &data_point).await?;

        Ok(())
    }

    /// 获取租户使用统计
    #[instrument(skip(self))]
    pub async fn get_tenant_usage_stats(
        &self,
        tenant_id: Uuid,
        period_hours: u32,
    ) -> Result<TenantUsageStats, AiStudioError> {
        let period_end = Utc::now();
        let period_start = period_end - Duration::hours(period_hours as i64);

        // 这里应该从时序数据库查询实际的统计数据
        // 为了简化，返回模拟数据
        Ok(TenantUsageStats {
            tenant_id,
            period_start,
            period_end,
            api_calls: UsageMetric {
                current: 150.0,
                max: 200.0,
                min: 50.0,
                average: 125.0,
                total: 3000.0,
                trend_percentage: 15.5,
            },
            ai_queries: UsageMetric {
                current: 45.0,
                max: 60.0,
                min: 20.0,
                average: 40.0,
                total: 960.0,
                trend_percentage: 8.2,
            },
            storage_usage: UsageMetric {
                current: 1024.0 * 1024.0 * 500.0, // 500MB
                max: 1024.0 * 1024.0 * 600.0,
                min: 1024.0 * 1024.0 * 400.0,
                average: 1024.0 * 1024.0 * 480.0,
                total: 1024.0 * 1024.0 * 500.0,
                trend_percentage: 5.2,
            },
            active_users: UsageMetric {
                current: 25.0,
                max: 30.0,
                min: 15.0,
                average: 22.0,
                total: 25.0,
                trend_percentage: 12.5,
            },
            error_rate: UsageMetric {
                current: 2.5,
                max: 5.0,
                min: 1.0,
                average: 2.8,
                total: 2.5,
                trend_percentage: -10.0,
            },
            avg_response_time: UsageMetric {
                current: 250.0,
                max: 400.0,
                min: 150.0,
                average: 220.0,
                total: 250.0,
                trend_percentage: -5.5,
            },
        })
    }

    /// 获取系统健康状态
    #[instrument(skip(self))]
    pub async fn get_system_health(&self) -> Result<SystemHealth, AiStudioError> {
        let mut components = HashMap::new();
        let mut overall_status = HealthStatus::Healthy;
        let mut active_alerts = 0;

        // 检查数据库健康状态
        let db_health = self.check_database_health().await;
        if db_health.status != HealthStatus::Healthy {
            overall_status = HealthStatus::Warning;
        }
        components.insert("database".to_string(), db_health);

        // 检查 Redis 健康状态
        let redis_health = self.check_redis_health().await;
        if redis_health.status != HealthStatus::Healthy && overall_status == HealthStatus::Healthy {
            overall_status = HealthStatus::Warning;
        }
        components.insert("redis".to_string(), redis_health);

        // 检查 AI 服务健康状态
        let ai_health = self.check_ai_service_health().await;
        if ai_health.status != HealthStatus::Healthy && overall_status == HealthStatus::Healthy {
            overall_status = HealthStatus::Warning;
        }
        components.insert("ai_service".to_string(), ai_health);

        // 获取活跃告警数量
        active_alerts = self.get_active_alerts_count().await?;
        if active_alerts > 0 && overall_status == HealthStatus::Healthy {
            overall_status = HealthStatus::Warning;
        }

        Ok(SystemHealth {
            overall_status,
            components,
            active_alerts,
            last_check: Utc::now(),
        })
    }

    /// 创建告警规则
    #[instrument(skip(self))]
    pub async fn create_alert_rule(
        &self,
        tenant_id: Uuid,
        rule: AlertRule,
    ) -> Result<AlertRule, AiStudioError> {
        info!(
            tenant_id = %tenant_id,
            rule_name = %rule.name,
            "创建告警规则"
        );

        // 这里应该将告警规则存储到数据库
        // 为了简化，直接返回规则
        Ok(rule)
    }

    /// 获取活跃告警
    #[instrument(skip(self))]
    pub async fn get_active_alerts(&self, tenant_id: Uuid) -> Result<Vec<AlertEvent>, AiStudioError> {
        // 这里应该从数据库查询活跃的告警事件
        // 为了简化，返回空列表
        Ok(vec![])
    }

    /// 解决告警
    #[instrument(skip(self))]
    pub async fn resolve_alert(&self, alert_id: Uuid) -> Result<(), AiStudioError> {
        info!(alert_id = %alert_id, "解决告警");
        
        // 这里应该更新数据库中的告警状态
        Ok(())
    }

    /// 批量记录指标
    #[instrument(skip(self))]
    pub async fn batch_record_metrics(
        &self,
        tenant_id: Uuid,
        data_points: Vec<MetricDataPoint>,
    ) -> Result<(), AiStudioError> {
        for data_point in data_points {
            self.record_metric(tenant_id, data_point).await?;
        }
        Ok(())
    }

    /// 获取指标趋势
    #[instrument(skip(self))]
    pub async fn get_metric_trends(
        &self,
        tenant_id: Uuid,
        metric_type: MetricType,
        hours: u32,
    ) -> Result<Vec<MetricDataPoint>, AiStudioError> {
        // 这里应该从时序数据库查询趋势数据
        // 为了简化，返回模拟数据
        let mut trends = Vec::new();
        let now = Utc::now();
        
        for i in 0..hours {
            let timestamp = now - Duration::hours(i as i64);
            let value = 100.0 + (i as f64 * 5.0) + (rand::random::<f64>() * 20.0 - 10.0);
            
            trends.push(MetricDataPoint {
                metric_type: metric_type.clone(),
                value,
                timestamp,
                labels: HashMap::new(),
            });
        }

        Ok(trends)
    }

    // 私有辅助方法

    /// 检查告警
    async fn check_alerts(
        &self,
        tenant_id: Uuid,
        data_point: &MetricDataPoint,
    ) -> Result<(), AiStudioError> {
        // 这里应该检查是否有匹配的告警规则被触发
        // 为了简化，只记录日志
        debug!(
            tenant_id = %tenant_id,
            metric_type = ?data_point.metric_type,
            value = data_point.value,
            "检查告警规则"
        );

        Ok(())
    }

    /// 检查数据库健康状态
    async fn check_database_health(&self) -> ComponentHealth {
        let start_time = std::time::Instant::now();
        
        match Tenant::find().limit(1).all(&self.db).await {
            Ok(_) => ComponentHealth {
                status: HealthStatus::Healthy,
                response_time_ms: Some(start_time.elapsed().as_millis() as u64),
                error_message: None,
                last_check: Utc::now(),
            },
            Err(e) => ComponentHealth {
                status: HealthStatus::Unhealthy,
                response_time_ms: Some(start_time.elapsed().as_millis() as u64),
                error_message: Some(e.to_string()),
                last_check: Utc::now(),
            },
        }
    }

    /// 检查 Redis 健康状态
    async fn check_redis_health(&self) -> ComponentHealth {
        // 这里应该检查 Redis 连接
        // 为了简化，返回健康状态
        ComponentHealth {
            status: HealthStatus::Healthy,
            response_time_ms: Some(10),
            error_message: None,
            last_check: Utc::now(),
        }
    }

    /// 检查 AI 服务健康状态
    async fn check_ai_service_health(&self) -> ComponentHealth {
        // 这里应该检查 AI 服务连接
        // 为了简化，返回健康状态
        ComponentHealth {
            status: HealthStatus::Healthy,
            response_time_ms: Some(200),
            error_message: None,
            last_check: Utc::now(),
        }
    }

    /// 获取活跃告警数量
    async fn get_active_alerts_count(&self) -> Result<u32, AiStudioError> {
        // 这里应该从数据库查询活跃告警数量
        Ok(0)
    }
}

/// 监控服务工厂
pub struct MonitoringServiceFactory;

impl MonitoringServiceFactory {
    /// 创建监控服务实例
    pub fn create(db: DatabaseConnection) -> MonitoringService {
        MonitoringService::new(db)
    }
}

/// 监控指标收集器
pub struct MetricsCollector {
    monitoring_service: MonitoringService,
}

impl MetricsCollector {
    /// 创建指标收集器
    pub fn new(monitoring_service: MonitoringService) -> Self {
        Self { monitoring_service }
    }

    /// 收集 API 调用指标
    pub async fn collect_api_call_metric(
        &self,
        tenant_id: Uuid,
        endpoint: &str,
        method: &str,
        status_code: u16,
        response_time_ms: u64,
    ) -> Result<(), AiStudioError> {
        let mut labels = HashMap::new();
        labels.insert("endpoint".to_string(), endpoint.to_string());
        labels.insert("method".to_string(), method.to_string());
        labels.insert("status_code".to_string(), status_code.to_string());

        // 记录 API 调用次数
        let api_call_metric = MetricDataPoint {
            metric_type: MetricType::ApiCalls,
            value: 1.0,
            timestamp: Utc::now(),
            labels: labels.clone(),
        };
        self.monitoring_service.record_metric(tenant_id, api_call_metric).await?;

        // 记录响应时间
        let response_time_metric = MetricDataPoint {
            metric_type: MetricType::ResponseTime,
            value: response_time_ms as f64,
            timestamp: Utc::now(),
            labels,
        };
        self.monitoring_service.record_metric(tenant_id, response_time_metric).await?;

        // 记录错误率（如果是错误状态码）
        if status_code >= 400 {
            let error_metric = MetricDataPoint {
                metric_type: MetricType::ErrorRate,
                value: 1.0,
                timestamp: Utc::now(),
                labels: HashMap::new(),
            };
            self.monitoring_service.record_metric(tenant_id, error_metric).await?;
        }

        Ok(())
    }

    /// 收集 AI 查询指标
    pub async fn collect_ai_query_metric(
        &self,
        tenant_id: Uuid,
        query_type: &str,
        processing_time_ms: u64,
        success: bool,
    ) -> Result<(), AiStudioError> {
        let mut labels = HashMap::new();
        labels.insert("query_type".to_string(), query_type.to_string());
        labels.insert("success".to_string(), success.to_string());

        let metric = MetricDataPoint {
            metric_type: MetricType::AiQueries,
            value: 1.0,
            timestamp: Utc::now(),
            labels,
        };

        self.monitoring_service.record_metric(tenant_id, metric).await?;

        // 记录处理时间
        let processing_time_metric = MetricDataPoint {
            metric_type: MetricType::ResponseTime,
            value: processing_time_ms as f64,
            timestamp: Utc::now(),
            labels: HashMap::new(),
        };
        self.monitoring_service.record_metric(tenant_id, processing_time_metric).await?;

        Ok(())
    }

    /// 收集存储使用指标
    pub async fn collect_storage_metric(
        &self,
        tenant_id: Uuid,
        storage_bytes: u64,
        operation: &str,
    ) -> Result<(), AiStudioError> {
        let mut labels = HashMap::new();
        labels.insert("operation".to_string(), operation.to_string());

        let metric = MetricDataPoint {
            metric_type: MetricType::StorageUsage,
            value: storage_bytes as f64,
            timestamp: Utc::now(),
            labels,
        };

        self.monitoring_service.record_metric(tenant_id, metric).await
    }
}