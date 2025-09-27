// 任务队列服务
// 用于处理异步批量操作和长时间运行的任务

use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::{RwLock, mpsc};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{info, warn, error, debug};

use crate::errors::AiStudioError;

/// 任务状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// 任务类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskType {
    BatchDocumentDelete,
    BatchDocumentUpdate,
    BatchDocumentReprocess,
    BatchDocumentImport,
    BatchDocumentExport,
    DocumentProcessing,
    KnowledgeBaseReindex,
}

/// 任务信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskInfo {
    /// 任务 ID
    pub id: Uuid,
    /// 任务类型
    pub task_type: TaskType,
    /// 租户 ID
    pub tenant_id: Uuid,
    /// 任务状态
    pub status: TaskStatus,
    /// 任务参数
    pub parameters: serde_json::Value,
    /// 进度百分比 (0-100)
    pub progress: u8,
    /// 总数量
    pub total_count: Option<u32>,
    /// 成功数量
    pub success_count: u32,
    /// 失败数量
    pub error_count: u32,
    /// 错误信息
    pub error_message: Option<String>,
    /// 结果数据
    pub result: Option<serde_json::Value>,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 开始时间
    pub started_at: Option<DateTime<Utc>>,
    /// 完成时间
    pub completed_at: Option<DateTime<Utc>>,
    /// 过期时间
    pub expires_at: DateTime<Utc>,
}

/// 任务执行器接口
#[async_trait::async_trait]
pub trait TaskExecutor: Send + Sync {
    /// 执行任务
    async fn execute(&self, task: &mut TaskInfo) -> Result<(), AiStudioError>;
    
    /// 获取支持的任务类型
    fn supported_task_types(&self) -> Vec<TaskType>;
}

/// 任务队列服务
pub struct TaskQueueService {
    /// 任务存储
    tasks: Arc<RwLock<HashMap<Uuid, TaskInfo>>>,
    /// 任务发送器
    task_sender: mpsc::UnboundedSender<Uuid>,
    /// 任务执行器
    executors: Arc<RwLock<HashMap<TaskType, Arc<dyn TaskExecutor>>>>,
}

impl TaskQueueService {
    /// 创建新的任务队列服务
    pub fn new() -> Self {
        let tasks = Arc::new(RwLock::new(HashMap::new()));
        let (task_sender, task_receiver) = mpsc::unbounded_channel();
        let executors = Arc::new(RwLock::new(HashMap::new()));
        
        let service = Self {
            tasks: tasks.clone(),
            task_sender,
            executors: executors.clone(),
        };
        
        // 启动任务处理器
        tokio::spawn(Self::task_processor(tasks, task_receiver, executors));
        
        service
    }
    
    /// 注册任务执行器
    pub async fn register_executor(&self, executor: Arc<dyn TaskExecutor>) {
        let mut executors = self.executors.write().await;
        for task_type in executor.supported_task_types() {
            executors.insert(task_type, executor.clone());
        }
    }
    
    /// 提交任务
    pub async fn submit_task(
        &self,
        task_type: TaskType,
        tenant_id: Uuid,
        parameters: serde_json::Value,
        total_count: Option<u32>,
    ) -> Result<Uuid, AiStudioError> {
        let task_id = Uuid::new_v4();
        let now = Utc::now();
        
        let task = TaskInfo {
            id: task_id,
            task_type,
            tenant_id,
            status: TaskStatus::Pending,
            parameters,
            progress: 0,
            total_count,
            success_count: 0,
            error_count: 0,
            error_message: None,
            result: None,
            created_at: now,
            started_at: None,
            completed_at: None,
            expires_at: now + chrono::Duration::hours(24), // 24小时后过期
        };
        
        // 存储任务
        {
            let mut tasks = self.tasks.write().await;
            tasks.insert(task_id, task);
        }
        
        // 发送任务到处理队列
        self.task_sender.send(task_id).map_err(|e| {
            error!("发送任务到队列失败: {}", e);
            AiStudioError::internal("任务队列错误")
        })?;
        
        info!("任务已提交: id={}, type={:?}", task_id, task_type);
        Ok(task_id)
    }
    
    /// 获取任务状态
    pub async fn get_task_status(&self, task_id: Uuid) -> Option<TaskInfo> {
        let tasks = self.tasks.read().await;
        tasks.get(&task_id).cloned()
    }
    
    /// 获取租户的任务列表
    pub async fn get_tenant_tasks(&self, tenant_id: Uuid) -> Vec<TaskInfo> {
        let tasks = self.tasks.read().await;
        tasks.values()
            .filter(|task| task.tenant_id == tenant_id)
            .cloned()
            .collect()
    }
    
    /// 取消任务
    pub async fn cancel_task(&self, task_id: Uuid) -> Result<bool, AiStudioError> {
        let mut tasks = self.tasks.write().await;
        if let Some(task) = tasks.get_mut(&task_id) {
            if task.status == TaskStatus::Pending || task.status == TaskStatus::Running {
                task.status = TaskStatus::Cancelled;
                task.completed_at = Some(Utc::now());
                info!("任务已取消: id={}", task_id);
                Ok(true)
            } else {
                Ok(false)
            }
        } else {
            Ok(false)
        }
    }
    
    /// 清理过期任务
    pub async fn cleanup_expired_tasks(&self) -> u32 {
        let now = Utc::now();
        let mut tasks = self.tasks.write().await;
        let initial_count = tasks.len();
        
        tasks.retain(|_, task| task.expires_at > now);
        
        let removed_count = initial_count - tasks.len();
        if removed_count > 0 {
            info!("清理了 {} 个过期任务", removed_count);
        }
        
        removed_count as u32
    }
    
    /// 任务处理器
    async fn task_processor(
        tasks: Arc<RwLock<HashMap<Uuid, TaskInfo>>>,
        mut task_receiver: mpsc::UnboundedReceiver<Uuid>,
        executors: Arc<RwLock<HashMap<TaskType, Arc<dyn TaskExecutor>>>>,
    ) {
        info!("任务处理器已启动");
        
        while let Some(task_id) = task_receiver.recv().await {
            // 获取任务
            let mut task = {
                let mut tasks_guard = tasks.write().await;
                if let Some(task) = tasks_guard.get_mut(&task_id) {
                    if task.status != TaskStatus::Pending {
                        continue; // 跳过非待处理任务
                    }
                    task.status = TaskStatus::Running;
                    task.started_at = Some(Utc::now());
                    task.clone()
                } else {
                    warn!("任务不存在: id={}", task_id);
                    continue;
                }
            };
            
            // 查找执行器
            let executor = {
                let executors_guard = executors.read().await;
                executors_guard.get(&task.task_type).cloned()
            };
            
            if let Some(executor) = executor {
                info!("开始执行任务: id={}, type={:?}", task_id, task.task_type);
                
                // 执行任务
                let result = executor.execute(&mut task).await;
                
                // 更新任务状态
                {
                    let mut tasks_guard = tasks.write().await;
                    if let Some(stored_task) = tasks_guard.get_mut(&task_id) {
                        *stored_task = task.clone();
                        stored_task.completed_at = Some(Utc::now());
                        
                        match result {
                            Ok(_) => {
                                stored_task.status = TaskStatus::Completed;
                                stored_task.progress = 100;
                                info!("任务执行成功: id={}", task_id);
                            }
                            Err(e) => {
                                stored_task.status = TaskStatus::Failed;
                                stored_task.error_message = Some(e.to_string());
                                error!("任务执行失败: id={}, error={}", task_id, e);
                            }
                        }
                    }
                }
            } else {
                error!("未找到任务执行器: type={:?}", task.task_type);
                
                // 标记任务失败
                let mut tasks_guard = tasks.write().await;
                if let Some(stored_task) = tasks_guard.get_mut(&task_id) {
                    stored_task.status = TaskStatus::Failed;
                    stored_task.error_message = Some("未找到任务执行器".to_string());
                    stored_task.completed_at = Some(Utc::now());
                }
            }
        }
        
        info!("任务处理器已停止");
    }
    
    /// 启动定期清理任务
    pub async fn start_cleanup_scheduler(&self) {
        let tasks = self.tasks.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(3600)); // 每小时清理一次
            
            loop {
                interval.tick().await;
                
                let now = Utc::now();
                let mut tasks_guard = tasks.write().await;
                let initial_count = tasks_guard.len();
                
                tasks_guard.retain(|_, task| task.expires_at > now);
                
                let removed_count = initial_count - tasks_guard.len();
                if removed_count > 0 {
                    info!("定期清理了 {} 个过期任务", removed_count);
                }
            }
        });
    }
}

/// 默认任务执行器（示例实现）
pub struct DefaultTaskExecutor;

#[async_trait::async_trait]
impl TaskExecutor for DefaultTaskExecutor {
    async fn execute(&self, task: &mut TaskInfo) -> Result<(), AiStudioError> {
        debug!("执行默认任务: id={}, type={:?}", task.id, task.task_type);
        
        // 模拟任务执行
        for i in 1..=10 {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            task.progress = (i * 10) as u8;
            
            if task.status == TaskStatus::Cancelled {
                return Err(AiStudioError::cancelled("任务已取消"));
            }
        }
        
        task.success_count = task.total_count.unwrap_or(1);
        Ok(())
    }
    
    fn supported_task_types(&self) -> Vec<TaskType> {
        vec![
            TaskType::BatchDocumentDelete,
            TaskType::BatchDocumentUpdate,
            TaskType::BatchDocumentReprocess,
            TaskType::BatchDocumentImport,
            TaskType::BatchDocumentExport,
            TaskType::DocumentProcessing,
            TaskType::KnowledgeBaseReindex,
        ]
    }
}

/// 任务队列服务工厂
pub struct TaskQueueServiceFactory;

impl TaskQueueServiceFactory {
    /// 创建任务队列服务实例
    pub async fn create() -> Arc<TaskQueueService> {
        let service = Arc::new(TaskQueueService::new());
        
        // 注册默认执行器
        let default_executor = Arc::new(DefaultTaskExecutor);
        service.register_executor(default_executor).await;
        
        // 启动清理调度器
        service.start_cleanup_scheduler().await;
        
        service
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_task_queue_basic_operations() {
        let service = TaskQueueService::new();
        
        // 提交任务
        let task_id = service.submit_task(
            TaskType::BatchDocumentDelete,
            Uuid::new_v4(),
            serde_json::json!({"test": "data"}),
            Some(10),
        ).await.unwrap();
        
        // 获取任务状态
        let task = service.get_task_status(task_id).await;
        assert!(task.is_some());
        
        let task = task.unwrap();
        assert_eq!(task.id, task_id);
        assert_eq!(task.task_type, TaskType::BatchDocumentDelete);
    }
    
    #[tokio::test]
    async fn test_task_cancellation() {
        let service = TaskQueueService::new();
        
        let task_id = service.submit_task(
            TaskType::BatchDocumentUpdate,
            Uuid::new_v4(),
            serde_json::json!({}),
            None,
        ).await.unwrap();
        
        // 取消任务
        let cancelled = service.cancel_task(task_id).await.unwrap();
        assert!(cancelled);
        
        // 检查任务状态
        let task = service.get_task_status(task_id).await.unwrap();
        assert_eq!(task.status, TaskStatus::Cancelled);
    }
}