// 配置验证器
// 提供详细的配置验证逻辑

use crate::config::AppConfig;
use aionix_common::CommonError;
use std::path::Path;
use url::Url;

/// 配置验证器
pub struct ConfigValidator;

impl ConfigValidator {
    /// 验证完整配置
    pub fn validate_all(config: &AppConfig) -> Result<(), Vec<CommonError>> {
        let mut errors = Vec::new();

        // 验证各个模块
        if let Err(e) = Self::validate_server(&config.server) {
            errors.push(e);
        }

        if let Err(e) = Self::validate_database(&config.database) {
            errors.push(e);
        }

        if let Err(e) = Self::validate_ai(&config.ai) {
            errors.push(e);
        }

        #[cfg(feature = "redis")]
        if let Err(e) = Self::validate_redis(&config.redis) {
            errors.push(e);
        }

        if let Err(e) = Self::validate_security(&config.security) {
            errors.push(e);
        }

        if let Err(e) = Self::validate_storage(&config.storage) {
            errors.push(e);
        }

        if let Err(e) = Self::validate_logging(&config.logging) {
            errors.push(e);
        }

        if let Err(e) = Self::validate_vector(&config.vector) {
            errors.push(e);
        }

        if let Err(e) = Self::validate_environment(&config.environment) {
            errors.push(e);
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// 验证服务器配置
    pub fn validate_server(config: &crate::config::ServerConfig) -> Result<(), CommonError> {
        if config.port == 0 {
            return Err(CommonError::validation("服务器端口不能为 0"));
        }

        if config.port < 1024 && !cfg!(test) {
            return Err(CommonError::validation("建议使用 1024 以上的端口"));
        }

        if config.host.is_empty() {
            return Err(CommonError::validation("服务器主机地址不能为空"));
        }

        if let Some(workers) = config.workers {
            if workers == 0 {
                return Err(CommonError::validation("工作线程数不能为 0"));
            }
            if workers > 32 {
                return Err(CommonError::validation("工作线程数不建议超过 32"));
            }
        }

        Ok(())
    }

    /// 验证数据库配置
    pub fn validate_database(config: &crate::config::DatabaseConfig) -> Result<(), CommonError> {
        if config.url.is_empty() {
            return Err(CommonError::validation("数据库 URL 不能为空"));
        }

        // 验证 URL 格式
        if let Err(_) = Url::parse(&config.url) {
            return Err(CommonError::validation("数据库 URL 格式无效"));
        }

        if config.max_connections == 0 {
            return Err(CommonError::validation("数据库最大连接数不能为 0"));
        }

        if config.min_connections > config.max_connections {
            return Err(CommonError::validation("数据库最小连接数不能大于最大连接数"));
        }

        if config.connect_timeout == 0 {
            return Err(CommonError::validation("数据库连接超时不能为 0"));
        }

        Ok(())
    }

    /// 验证 AI 配置
    pub fn validate_ai(config: &crate::config::AiConfig) -> Result<(), CommonError> {
        if config.model_endpoint.is_empty() {
            return Err(CommonError::validation("AI 模型端点不能为空"));
        }

        // 验证端点 URL 格式
        if let Err(_) = Url::parse(&config.model_endpoint) {
            return Err(CommonError::validation("AI 模型端点 URL 格式无效"));
        }

        if config.max_tokens == 0 {
            return Err(CommonError::validation("AI 最大 token 数不能为 0"));
        }

        if config.max_tokens > 100000 {
            return Err(CommonError::validation("AI 最大 token 数不建议超过 100000"));
        }

        if !(0.0..=2.0).contains(&config.temperature) {
            return Err(CommonError::validation("AI 温度参数必须在 0.0-2.0 之间"));
        }

        if config.timeout == 0 {
            return Err(CommonError::validation("AI 请求超时不能为 0"));
        }

        if config.retry_attempts > 10 {
            return Err(CommonError::validation("AI 重试次数不建议超过 10"));
        }

        Ok(())
    }

    /// 验证 Redis 配置
    #[cfg(feature = "redis")]
    pub fn validate_redis(config: &crate::config::RedisConfig) -> Result<(), CommonError> {
        if config.url.is_empty() {
            return Err(CommonError::validation("Redis URL 不能为空"));
        }

        // 验证 Redis URL 格式
        if let Err(_) = Url::parse(&config.url) {
            return Err(CommonError::validation("Redis URL 格式无效"));
        }

        if config.max_connections == 0 {
            return Err(CommonError::validation("Redis 最大连接数不能为 0"));
        }

        if config.connection_timeout == 0 {
            return Err(CommonError::validation("Redis 连接超时不能为 0"));
        }

        if config.response_timeout == 0 {
            return Err(CommonError::validation("Redis 响应超时不能为 0"));
        }

        Ok(())
    }

    /// 验证安全配置
    pub fn validate_security(config: &crate::config::SecurityConfig) -> Result<(), CommonError> {
        if config.jwt_secret.len() < 32 {
            return Err(CommonError::validation("JWT 密钥长度不能少于 32 个字符"));
        }

        if config.jwt_expiration == 0 {
            return Err(CommonError::validation("JWT 过期时间不能为 0"));
        }

        if config.jwt_expiration > 86400 * 30 { // 30 天
            return Err(CommonError::validation("JWT 过期时间不建议超过 30 天"));
        }

        if !(4..=31).contains(&config.bcrypt_cost) {
            return Err(CommonError::validation("bcrypt 成本参数必须在 4-31 之间"));
        }

        if config.rate_limit_requests == 0 {
            return Err(CommonError::validation("限流请求数不能为 0"));
        }

        if config.rate_limit_window == 0 {
            return Err(CommonError::validation("限流时间窗口不能为 0"));
        }

        Ok(())
    }

    /// 验证存储配置
    pub fn validate_storage(config: &crate::config::StorageConfig) -> Result<(), CommonError> {
        if config.path.is_empty() {
            return Err(CommonError::validation("存储路径不能为空"));
        }

        // 检查存储路径是否存在，如果不存在尝试创建
        let storage_path = Path::new(&config.path);
        if !storage_path.exists() {
            if let Err(e) = std::fs::create_dir_all(storage_path) {
                return Err(CommonError::validation(
                    format!("无法创建存储目录 {}: {}", config.path, e)
                ));
            }
        }

        if config.max_file_size == 0 {
            return Err(CommonError::validation("最大文件大小不能为 0"));
        }

        if config.max_file_size > 1024 * 1024 * 1024 { // 1GB
            return Err(CommonError::validation("最大文件大小不建议超过 1GB"));
        }

        if config.allowed_extensions.is_empty() {
            return Err(CommonError::validation("允许的文件扩展名列表不能为空"));
        }

        Ok(())
    }

    /// 验证日志配置
    pub fn validate_logging(config: &crate::config::LoggingConfig) -> Result<(), CommonError> {
        let valid_levels = ["trace", "debug", "info", "warn", "error"];
        if !valid_levels.contains(&config.level.as_str()) {
            return Err(CommonError::validation(
                format!("无效的日志级别: {}，有效值: {:?}", config.level, valid_levels)
            ));
        }

        let valid_formats = ["json", "pretty", "compact"];
        if !valid_formats.contains(&config.format.as_str()) {
            return Err(CommonError::validation(
                format!("无效的日志格式: {}，有效值: {:?}", config.format, valid_formats)
            ));
        }

        if config.file_enabled {
            if let Some(ref path) = config.file_path {
                let log_dir = Path::new(path).parent().unwrap_or(Path::new("."));
                if !log_dir.exists() {
                    if let Err(e) = std::fs::create_dir_all(log_dir) {
                        return Err(CommonError::validation(
                            format!("无法创建日志目录: {}", e)
                        ));
                    }
                }
            } else {
                return Err(CommonError::validation("启用文件日志时必须指定日志文件路径"));
            }
        }

        Ok(())
    }

    /// 验证向量配置
    pub fn validate_vector(config: &crate::config::VectorConfig) -> Result<(), CommonError> {
        if config.dimension == 0 {
            return Err(CommonError::validation("向量维度不能为 0"));
        }

        if config.dimension > 4096 {
            return Err(CommonError::validation("向量维度不建议超过 4096"));
        }

        if !(0.0..=1.0).contains(&config.similarity_threshold) {
            return Err(CommonError::validation("相似度阈值必须在 0.0-1.0 之间"));
        }

        let valid_index_types = ["hnsw", "ivf", "flat"];
        if !valid_index_types.contains(&config.index_type.as_str()) {
            return Err(CommonError::validation(
                format!("无效的索引类型: {}，有效值: {:?}", config.index_type, valid_index_types)
            ));
        }

        if config.ef_construction == 0 {
            return Err(CommonError::validation("HNSW ef_construction 参数不能为 0"));
        }

        if config.m == 0 {
            return Err(CommonError::validation("HNSW m 参数不能为 0"));
        }

        Ok(())
    }

    /// 验证环境配置
    pub fn validate_environment(config: &crate::config::EnvironmentConfig) -> Result<(), CommonError> {
        let valid_environments = ["development", "staging", "production", "test"];
        if !valid_environments.contains(&config.name.as_str()) {
            return Err(CommonError::validation(
                format!("无效的环境名称: {}，有效值: {:?}", config.name, valid_environments)
            ));
        }

        if config.version.is_empty() {
            return Err(CommonError::validation("版本信息不能为空"));
        }

        Ok(())
    }
}