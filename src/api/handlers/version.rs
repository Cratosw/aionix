// 版本信息处理器

use actix_web::{web, HttpResponse, Result as ActixResult};
use utoipa::{OpenApi, ToSchema};
use chrono::Utc;

use crate::api::models::ApiVersion;
use crate::api::responses::HttpResponseBuilder;

/// 版本 API 文档
// #[derive(OpenApi)]
// #[openapi(
//     paths(get_version, get_build_info),
//     components(schemas(ApiVersion))
// )]
// pub struct VersionApiDoc;

/// 获取 API 版本信息
pub async fn get_version() -> ActixResult<HttpResponse> {
    let version_info = ApiVersion {
        version: env!("CARGO_PKG_VERSION").to_string(),
        build_time: get_build_time(),
        git_hash: get_git_hash(),
        features: get_enabled_features(),
    };

    HttpResponseBuilder::ok(version_info)
}

/// 获取构建信息
pub async fn get_build_info() -> ActixResult<HttpResponse> {
    let build_info = serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "name": env!("CARGO_PKG_NAME"),
        "description": env!("CARGO_PKG_DESCRIPTION"),
        "authors": env!("CARGO_PKG_AUTHORS").split(':').collect::<Vec<&str>>(),
        "repository": env!("CARGO_PKG_REPOSITORY"),
        "license": env!("CARGO_PKG_LICENSE"),
        "rust_version": env!("CARGO_PKG_RUST_VERSION"),
        "build_time": get_build_time(),
        "git_hash": get_git_hash(),
        "git_branch": get_git_branch(),
        "build_profile": get_build_profile(),
        "target_triple": get_target_triple(),
        "features": get_enabled_features(),
        "dependencies": get_dependency_info(),
    });

    HttpResponseBuilder::ok(build_info)
}

/// 获取 API 规范信息
pub async fn get_api_spec() -> ActixResult<HttpResponse> {
    let spec_info = serde_json::json!({
        "openapi": "3.0.3",
        "info": {
            "title": "Aionix AI Studio API",
            "description": "企业级 AI Studio API 接口文档",
            "version": env!("CARGO_PKG_VERSION"),
            "contact": {
                "name": "Aionix Team",
                "url": "https://aionix.ai",
                "email": "support@aionix.ai"
            },
            "license": {
                "name": env!("CARGO_PKG_LICENSE"),
                "url": "https://opensource.org/licenses/MIT"
            }
        },
        "servers": [
            {
                "url": "/api/v1",
                "description": "API v1"
            }
        ],
        "paths": {},
        "components": {},
        "tags": [
            {
                "name": "Health",
                "description": "健康检查相关接口"
            },
            {
                "name": "Version",
                "description": "版本信息相关接口"
            },
            {
                "name": "Tenant",
                "description": "租户管理相关接口"
            },
            {
                "name": "User",
                "description": "用户管理相关接口"
            },
            {
                "name": "Auth",
                "description": "认证相关接口"
            },
            {
                "name": "Knowledge Base",
                "description": "知识库管理相关接口"
            },
            {
                "name": "Document",
                "description": "文档管理相关接口"
            },
            {
                "name": "QA",
                "description": "问答相关接口"
            },
            {
                "name": "Agent",
                "description": "Agent 管理相关接口"
            },
            {
                "name": "Workflow",
                "description": "工作流管理相关接口"
            }
        ]
    });

    HttpResponseBuilder::ok(spec_info)
}

// 私有辅助函数

/// 获取构建时间
fn get_build_time() -> String {
    // 在实际项目中，这应该在构建时设置
    // 这里返回一个占位符
    option_env!("BUILD_TIME")
        .unwrap_or("unknown")
        .to_string()
}

/// 获取 Git 哈希
fn get_git_hash() -> Option<String> {
    option_env!("GIT_HASH").map(|s| s.to_string())
}

/// 获取 Git 分支
fn get_git_branch() -> Option<String> {
    option_env!("GIT_BRANCH").map(|s| s.to_string())
}

/// 获取构建配置
fn get_build_profile() -> String {
    if cfg!(debug_assertions) {
        "debug".to_string()
    } else {
        "release".to_string()
    }
}

/// 获取目标三元组
fn get_target_triple() -> String {
    option_env!("TARGET")
        .unwrap_or("unknown")
        .to_string()
}

/// 获取启用的功能
pub fn get_enabled_features() -> Vec<String> {
    let mut features = vec!["default".to_string()];
    
    #[cfg(feature = "postgres")]
    features.push("postgres".to_string());
    
    #[cfg(feature = "sqlite")]
    features.push("sqlite".to_string());
    
    #[cfg(feature = "redis")]
    features.push("redis".to_string());
    
    #[cfg(feature = "ai")]
    features.push("ai".to_string());
    
    #[cfg(feature = "vector")]
    features.push("vector".to_string());
    
    features
}

/// 获取依赖信息
fn get_dependency_info() -> serde_json::Value {
    serde_json::json!({
        "actix-web": "4.x",
        "sea-orm": "0.12.x",
        "tokio": "1.x",
        "serde": "1.x",
        "uuid": "1.x",
        "chrono": "0.4.x",
        "tracing": "0.1.x",
        "utoipa": "4.x"
    })
}

/// 配置版本路由
pub fn configure_version_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/version")
            .route("", web::get().to(get_version))
            .route("/build-info", web::get().to(get_build_info))
            .route("/spec", web::get().to(get_api_spec))
    );
}