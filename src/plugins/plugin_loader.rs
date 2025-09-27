// 插件加载器
// 实现插件的动态加载和卸载机制

use std::sync::Arc;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use tracing::{info, warn, error, debug};
use tokio::fs;

use crate::plugins::plugin_interface::{PluginFactory, PluginMetadata, Plugin};
use crate::errors::AiStudioError;

/// 插件加载器
pub struct PluginLoader {
    /// 插件目录
    plugins_directory: PathBuf,
    /// 已加载的插件工厂
    loaded_factories: Arc<tokio::sync::RwLock<HashMap<String, Arc<dyn PluginFactory>>>>,
    /// 加载器配置
    config: LoaderConfig,
}

/// 加载器配置
#[derive(Debug, Clone)]
pub struct LoaderConfig {
    /// 支持的插件格式
    pub supported_formats: Vec<PluginFormat>,
    /// 是否启用插件验证
    pub enable_verification: bool,
    /// 插件加载超时时间（秒）
    pub load_timeout_seconds: u64,
    /// 最大插件大小（MB）
    pub max_plugin_size_mb: u64,
}

impl Default for LoaderConfig {
    fn default() -> Self {
        Self {
            supported_formats: vec![
                PluginFormat::Native,
                PluginFormat::Wasm,
                PluginFormat::Script,
            ],
            enable_verification: true,
            load_timeout_seconds: 30,
            max_plugin_size_mb: 100,
        }
    }
}

/// 插件格式
#[derive(Debug, Clone, PartialEq)]
pub enum PluginFormat {
    /// 原生动态库
    Native,
    /// WebAssembly
    Wasm,
    /// 脚本插件
    Script,
    /// 容器插件
    Container,
}

/// 插件包信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginPackage {
    /// 包路径
    pub path: PathBuf,
    /// 插件元数据
    pub metadata: PluginMetadata,
    /// 插件格式
    pub format: PluginFormat,
    /// 包大小（字节）
    pub size_bytes: u64,
    /// 校验和
    pub checksum: String,
    /// 加载时间
    pub loaded_at: chrono::DateTime<chrono::Utc>,
}

/// 加载结果
#[derive(Debug, Clone)]
pub struct LoadResult {
    /// 是否成功
    pub success: bool,
    /// 插件工厂
    pub factory: Option<Arc<dyn PluginFactory>>,
    /// 插件包信息
    pub package_info: Option<PluginPackage>,
    /// 错误信息
    pub error: Option<String>,
    /// 加载时间（毫秒）
    pub load_time_ms: u64,
}

impl PluginLoader {
    /// 创建新的插件加载器
    pub fn new(plugins_directory: PathBuf, config: Option<LoaderConfig>) -> Self {
        Self {
            plugins_directory,
            loaded_factories: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            config: config.unwrap_or_default(),
        }
    }
    
    /// 加载插件
    pub async fn load_plugin(&self, source: &str) -> Result<Arc<dyn PluginFactory>, AiStudioError> {
        info!("加载插件: {}", source);
        
        let start_time = std::time::Instant::now();
        
        // 解析插件源
        let plugin_path = self.resolve_plugin_path(source).await?;
        
        // 检查插件大小
        self.check_plugin_size(&plugin_path).await?;
        
        // 检测插件格式
        let format = self.detect_plugin_format(&plugin_path).await?;
        
        // 验证插件
        if self.config.enable_verification {
            self.verify_plugin(&plugin_path, &format).await?;
        }
        
        // 加载插件工厂
        let factory = match format {
            PluginFormat::Native => self.load_native_plugin(&plugin_path).await?,
            PluginFormat::Wasm => self.load_wasm_plugin(&plugin_path).await?,
            PluginFormat::Script => self.load_script_plugin(&plugin_path).await?,
            PluginFormat::Container => self.load_container_plugin(&plugin_path).await?,
        };
        
        let load_time = start_time.elapsed().as_millis() as u64;
        
        // 缓存工厂
        {
            let mut factories = self.loaded_factories.write().await;
            factories.insert(source.to_string(), factory.clone());
        }
        
        info!("插件加载成功: {} ({}ms)", source, load_time);
        
        Ok(factory)
    }
    
    /// 卸载插件
    pub async fn unload_plugin(&self, source: &str) -> Result<(), AiStudioError> {
        info!("卸载插件: {}", source);
        
        let mut factories = self.loaded_factories.write().await;
        
        if factories.remove(source).is_some() {
            info!("插件卸载成功: {}", source);
            Ok(())
        } else {
            Err(AiStudioError::not_found("插件未加载"))
        }
    }
    
    /// 重新加载插件
    pub async fn reload_plugin(&self, source: &str) -> Result<Arc<dyn PluginFactory>, AiStudioError> {
        info!("重新加载插件: {}", source);
        
        // 先卸载
        if let Err(e) = self.unload_plugin(source).await {
            warn!("卸载插件失败: {} - {}", source, e);
        }
        
        // 重新加载
        self.load_plugin(source).await
    }
    
    /// 扫描插件目录
    pub async fn scan_plugins(&self) -> Result<Vec<PluginPackage>, AiStudioError> {
        debug!("扫描插件目录: {}", self.plugins_directory.display());
        
        let mut packages = Vec::new();
        
        if !self.plugins_directory.exists() {
            warn!("插件目录不存在: {}", self.plugins_directory.display());
            return Ok(packages);
        }
        
        let mut entries = fs::read_dir(&self.plugins_directory).await.map_err(|e| {
            AiStudioError::io(format!("读取插件目录失败: {}", e))
        })?;
        
        while let Some(entry) = entries.next_entry().await.map_err(|e| {
            AiStudioError::io(format!("读取目录条目失败: {}", e))
        })? {
            let path = entry.path();
            
            if let Ok(package) = self.analyze_plugin_package(&path).await {
                packages.push(package);
            }
        }
        
        info!("扫描到 {} 个插件包", packages.len());
        
        Ok(packages)
    }
    
    /// 解析插件路径
    async fn resolve_plugin_path(&self, source: &str) -> Result<PathBuf, AiStudioError> {
        // 支持多种源格式：
        // 1. 本地文件路径
        // 2. URL（用于下载）
        // 3. 插件 ID（从注册表查找）
        
        if source.starts_with("http://") || source.starts_with("https://") {
            // TODO: 实现从 URL 下载插件
            return Err(AiStudioError::not_implemented("URL 下载暂未实现"));
        }
        
        let path = if Path::new(source).is_absolute() {
            PathBuf::from(source)
        } else {
            self.plugins_directory.join(source)
        };
        
        if !path.exists() {
            return Err(AiStudioError::not_found(&format!("插件文件不存在: {}", path.display())));
        }
        
        Ok(path)
    }
    
    /// 检查插件大小
    async fn check_plugin_size(&self, path: &Path) -> Result<(), AiStudioError> {
        let metadata = fs::metadata(path).await.map_err(|e| {
            AiStudioError::io(format!("获取文件元数据失败: {}", e))
        })?;
        
        let size_mb = metadata.len() / (1024 * 1024);
        
        if size_mb > self.config.max_plugin_size_mb {
            return Err(AiStudioError::validation(&format!(
                "插件文件太大: {}MB，最大允许: {}MB",
                size_mb, self.config.max_plugin_size_mb
            )));
        }
        
        Ok(())
    }
    
    /// 检测插件格式
    async fn detect_plugin_format(&self, path: &Path) -> Result<PluginFormat, AiStudioError> {
        let extension = path.extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("");
        
        let format = match extension.to_lowercase().as_str() {
            "so" | "dll" | "dylib" => PluginFormat::Native,
            "wasm" => PluginFormat::Wasm,
            "js" | "py" | "lua" => PluginFormat::Script,
            "tar" | "zip" => {
                // 检查是否为容器插件
                if self.is_container_plugin(path).await? {
                    PluginFormat::Container
                } else {
                    return Err(AiStudioError::validation("未知的插件格式"));
                }
            }
            _ => return Err(AiStudioError::validation("不支持的插件格式")),
        };
        
        // 检查格式是否支持
        if !self.config.supported_formats.contains(&format) {
            return Err(AiStudioError::validation(&format!("不支持的插件格式: {:?}", format)));
        }
        
        Ok(format)
    }
    
    /// 检查是否为容器插件
    async fn is_container_plugin(&self, _path: &Path) -> Result<bool, AiStudioError> {
        // TODO: 实现容器插件检测逻辑
        Ok(false)
    }
    
    /// 验证插件
    async fn verify_plugin(&self, path: &Path, format: &PluginFormat) -> Result<(), AiStudioError> {
        debug!("验证插件: {} ({:?})", path.display(), format);
        
        // TODO: 实现插件验证逻辑
        // 1. 检查文件完整性
        // 2. 验证数字签名
        // 3. 扫描恶意代码
        // 4. 检查权限要求
        
        Ok(())
    }
    
    /// 加载原生插件
    async fn load_native_plugin(&self, path: &Path) -> Result<Arc<dyn PluginFactory>, AiStudioError> {
        debug!("加载原生插件: {}", path.display());
        
        // TODO: 实现原生动态库加载
        // 使用 libloading 或类似库加载 .so/.dll/.dylib 文件
        
        Err(AiStudioError::not_implemented("原生插件加载暂未实现"))
    }
    
    /// 加载 WebAssembly 插件
    async fn load_wasm_plugin(&self, path: &Path) -> Result<Arc<dyn PluginFactory>, AiStudioError> {
        debug!("加载 WASM 插件: {}", path.display());
        
        // TODO: 实现 WebAssembly 插件加载
        // 使用 wasmtime 或 wasmer 运行时
        
        Err(AiStudioError::not_implemented("WASM 插件加载暂未实现"))
    }
    
    /// 加载脚本插件
    async fn load_script_plugin(&self, path: &Path) -> Result<Arc<dyn PluginFactory>, AiStudioError> {
        debug!("加载脚本插件: {}", path.display());
        
        // TODO: 实现脚本插件加载
        // 支持 JavaScript (V8/QuickJS)、Python、Lua 等
        
        Err(AiStudioError::not_implemented("脚本插件加载暂未实现"))
    }
    
    /// 加载容器插件
    async fn load_container_plugin(&self, path: &Path) -> Result<Arc<dyn PluginFactory>, AiStudioError> {
        debug!("加载容器插件: {}", path.display());
        
        // TODO: 实现容器插件加载
        // 使用 Docker 或 Podman 运行容器化插件
        
        Err(AiStudioError::not_implemented("容器插件加载暂未实现"))
    }
    
    /// 分析插件包
    async fn analyze_plugin_package(&self, path: &Path) -> Result<PluginPackage, AiStudioError> {
        let metadata = fs::metadata(path).await.map_err(|e| {
            AiStudioError::io(format!("获取文件元数据失败: {}", e))
        })?;
        
        let format = self.detect_plugin_format(path).await?;
        
        // TODO: 读取插件元数据
        // 从插件文件或配置文件中提取元数据
        
        let plugin_metadata = PluginMetadata {
            id: path.file_stem().unwrap_or_default().to_string_lossy().to_string(),
            name: path.file_name().unwrap_or_default().to_string_lossy().to_string(),
            version: "1.0.0".to_string(),
            description: "插件描述".to_string(),
            author: "未知作者".to_string(),
            license: "未知许可证".to_string(),
            homepage: None,
            repository: None,
            plugin_type: crate::plugins::plugin_interface::PluginType::Custom,
            api_version: "1.0".to_string(),
            min_system_version: "1.0.0".to_string(),
            dependencies: Vec::new(),
            permissions: Vec::new(),
            tags: Vec::new(),
            icon: None,
            created_at: chrono::Utc::now(),
        };
        
        // TODO: 计算文件校验和
        let checksum = self.calculate_checksum(path).await?;
        
        Ok(PluginPackage {
            path: path.to_path_buf(),
            metadata: plugin_metadata,
            format,
            size_bytes: metadata.len(),
            checksum,
            loaded_at: chrono::Utc::now(),
        })
    }
    
    /// 计算文件校验和
    async fn calculate_checksum(&self, path: &Path) -> Result<String, AiStudioError> {
        use sha2::{Sha256, Digest};
        
        let content = fs::read(path).await.map_err(|e| {
            AiStudioError::io(format!("读取文件失败: {}", e))
        })?;
        
        let mut hasher = Sha256::new();
        hasher.update(&content);
        let result = hasher.finalize();
        
        Ok(format!("{:x}", result))
    }
    
    /// 获取已加载的插件
    pub async fn get_loaded_plugins(&self) -> Vec<String> {
        let factories = self.loaded_factories.read().await;
        factories.keys().cloned().collect()
    }
    
    /// 检查插件是否已加载
    pub async fn is_plugin_loaded(&self, source: &str) -> bool {
        let factories = self.loaded_factories.read().await;
        factories.contains_key(source)
    }
    
    /// 清理加载器缓存
    pub async fn clear_cache(&self) -> Result<usize, AiStudioError> {
        let mut factories = self.loaded_factories.write().await;
        let count = factories.len();
        factories.clear();
        
        info!("清理了 {} 个插件工厂缓存", count);
        
        Ok(count)
    }
}

/// 插件加载器工厂
pub struct PluginLoaderFactory;

impl PluginLoaderFactory {
    /// 创建插件加载器实例
    pub fn create(
        plugins_directory: PathBuf,
        config: Option<LoaderConfig>,
    ) -> Arc<PluginLoader> {
        Arc::new(PluginLoader::new(plugins_directory, config))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[test]
    fn test_loader_config_default() {
        let config = LoaderConfig::default();
        assert_eq!(config.load_timeout_seconds, 30);
        assert_eq!(config.max_plugin_size_mb, 100);
        assert!(config.supported_formats.contains(&PluginFormat::Native));
    }
    
    #[tokio::test]
    async fn test_plugin_format_detection() {
        let temp_dir = TempDir::new().unwrap();
        let loader = PluginLoader::new(temp_dir.path().to_path_buf(), None);
        
        // 创建测试文件
        let so_path = temp_dir.path().join("test.so");
        tokio::fs::write(&so_path, b"fake so content").await.unwrap();
        
        let format = loader.detect_plugin_format(&so_path).await.unwrap();
        assert_eq!(format, PluginFormat::Native);
        
        let wasm_path = temp_dir.path().join("test.wasm");
        tokio::fs::write(&wasm_path, b"fake wasm content").await.unwrap();
        
        let format = loader.detect_plugin_format(&wasm_path).await.unwrap();
        assert_eq!(format, PluginFormat::Wasm);
    }
    
    #[tokio::test]
    async fn test_plugin_size_check() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = LoaderConfig::default();
        config.max_plugin_size_mb = 1; // 1MB 限制
        
        let loader = PluginLoader::new(temp_dir.path().to_path_buf(), Some(config));
        
        // 创建小文件
        let small_file = temp_dir.path().join("small.so");
        tokio::fs::write(&small_file, b"small content").await.unwrap();
        
        let result = loader.check_plugin_size(&small_file).await;
        assert!(result.is_ok());
        
        // 创建大文件（超过限制）
        let large_file = temp_dir.path().join("large.so");
        let large_content = vec![0u8; 2 * 1024 * 1024]; // 2MB
        tokio::fs::write(&large_file, large_content).await.unwrap();
        
        let result = loader.check_plugin_size(&large_file).await;
        assert!(result.is_err());
    }
}