// 认证 API 处理器

use actix_web::{web, HttpRequest, HttpResponse, Result as ActixResult};
use utoipa::{OpenApi, ToSchema};

use crate::api::extractors::{TenantExtractor, OptionalAuthExtractor, RequestIdExtractor};
use crate::api::responses::HttpResponseBuilder;
use crate::services::auth::{
    AuthService, LoginRequest, LoginResponse, RefreshTokenRequest, RefreshTokenResponse,
    RegisterRequest, RegisterResponse, PasswordResetRequest, PasswordResetConfirmRequest,
};
use crate::db::DatabaseManager;
use crate::errors::AiStudioError;

/// 认证 API 文档
// #[derive(OpenApi)]
// #[openapi(
//     paths(
//         login,
//         refresh_token,
//         register,
//         logout,
//         request_password_reset,
//         confirm_password_reset,
//         get_current_user,
//     ),
//     components(schemas(
//         LoginRequest,
//         LoginResponse,
//         RefreshTokenRequest,
//         RefreshTokenResponse,
//         RegisterRequest,
//         RegisterResponse,
//         PasswordResetRequest,
//         PasswordResetConfirmRequest,
//         crate::services::auth::UserInfo,
//         crate::services::auth::TenantInfo,
//     ))
// )]
// pub struct AuthApiDoc;

/// 用户登录
pub async fn login(
    req: HttpRequest,
    request: web::Json<LoginRequest>,
) -> ActixResult<HttpResponse> {
    let db_manager = DatabaseManager::get()?;
    let service = AuthService::new(
        db_manager.get_connection().clone(),
        "default_jwt_secret".to_string(), // 应该从配置中获取
        None,
        None,
    );

    // 提取客户端信息
    let client_ip = req
        .connection_info()
        .remote_addr()
        .map(|s| s.to_string());
    
    let user_agent = req
        .headers()
        .get("User-Agent")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());

    let response = service.login(request.into_inner(), client_ip, user_agent).await?;

    HttpResponseBuilder::ok(response)
}

/// 刷新访问令牌
pub async fn refresh_token(
    request: web::Json<RefreshTokenRequest>,
) -> ActixResult<HttpResponse> {
    let db_manager = DatabaseManager::get()?;
    let service = AuthService::new(
        db_manager.get_connection().clone(),
        "default_jwt_secret".to_string(),
        None,
        None,
    );

    let response = service.refresh_token(request.into_inner()).await?;

    HttpResponseBuilder::ok(response)
}

/// 用户注册
pub async fn register(
    request: web::Json<RegisterRequest>,
) -> ActixResult<HttpResponse> {
    let db_manager = DatabaseManager::get()?;
    let service = AuthService::new(
        db_manager.get_connection().clone(),
        "default_jwt_secret".to_string(),
        None,
        None,
    );

    let response = service.register(request.into_inner()).await?;

    HttpResponseBuilder::created(response)
}

/// 用户登出
pub async fn logout(
    request: web::Json<RefreshTokenRequest>,
) -> ActixResult<HttpResponse> {
    let db_manager = DatabaseManager::get()?;
    let service = AuthService::new(
        db_manager.get_connection().clone(),
        "default_jwt_secret".to_string(),
        None,
        None,
    );

    service.logout(&request.refresh_token).await?;

    HttpResponseBuilder::no_content()
}

/// 请求密码重置
pub async fn request_password_reset(
    request: web::Json<PasswordResetRequest>,
) -> ActixResult<HttpResponse> {
    let db_manager = DatabaseManager::get()?;
    let service = AuthService::new(
        db_manager.get_connection().clone(),
        "default_jwt_secret".to_string(),
        None,
        None,
    );

    service.request_password_reset(request.into_inner()).await?;

    HttpResponseBuilder::no_content()
}

/// 确认密码重置
pub async fn confirm_password_reset(
    request: web::Json<PasswordResetConfirmRequest>,
) -> ActixResult<HttpResponse> {
    // 这里应该实现密码重置确认逻辑
    // 为了简化，这里只返回成功响应
    HttpResponseBuilder::no_content()
}

/// 获取当前用户信息
pub async fn get_current_user(
    auth: crate::api::extractors::AuthExtractor,
) -> ActixResult<HttpResponse> {
    let db_manager = DatabaseManager::get()?;
    let db = db_manager.get_connection();

    // 从数据库获取最新的用户信息
    let user = crate::db::entities::user::Entity::find_by_id(auth.user_id)
        .one(db)
        .await?
        .ok_or_else(|| AiStudioError::not_found("用户"))?;

    let user_info = crate::services::auth::UserInfo {
        id: user.id,
        tenant_id: user.tenant_id,
        username: user.username,
        email: user.email,
        display_name: user.display_name,
        avatar_url: user.avatar_url,
        role: user.role,
        permissions: serde_json::from_value(user.permissions).unwrap_or_default(),
        last_login_at: user.last_login_at.map(|dt| dt.into()),
        created_at: user.created_at.into(),
    };

    HttpResponseBuilder::ok(user_info)
}

/// 验证邮箱
pub async fn verify_email(
    query: web::Query<EmailVerificationQuery>,
) -> ActixResult<HttpResponse> {
    // 这里应该实现邮箱验证逻辑
    // 为了简化，这里只返回成功响应
    HttpResponseBuilder::no_content()
}

/// 重新发送验证邮件
pub async fn resend_verification_email(
    request: web::Json<ResendVerificationRequest>,
) -> ActixResult<HttpResponse> {
    // 这里应该实现重新发送验证邮件逻辑
    // 为了简化，这里只返回成功响应
    HttpResponseBuilder::no_content()
}

// 辅助结构体

/// 邮箱验证查询参数
#[derive(serde::Deserialize)]
pub struct EmailVerificationQuery {
    pub token: String,
}

/// 重新发送验证邮件请求
#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct ResendVerificationRequest {
    /// 邮箱
    pub email: String,
    /// 租户标识符
    pub tenant_slug: String,
}

/// 配置认证路由
pub fn configure_auth_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/auth")
            .route("/login", web::post().to(login))
            .route("/refresh", web::post().to(refresh_token))
            .route("/register", web::post().to(register))
            .route("/logout", web::post().to(logout))
            .route("/password-reset", web::post().to(request_password_reset))
            .route("/password-reset/confirm", web::post().to(confirm_password_reset))
            .route("/me", web::get().to(get_current_user))
            .route("/verify-email", web::post().to(verify_email))
            .route("/resend-verification", web::post().to(resend_verification_email))
    );
}