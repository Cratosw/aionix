// 认证 API 处理器

use actix_web::{web, HttpRequest, HttpResponse, Result as ActixResult};

use crate::api::responses::HttpResponseBuilder;
use crate::services::auth::{
    AuthService, LoginRequest, RefreshTokenRequest,
    RegisterRequest, PasswordResetRequest, PasswordResetConfirmRequest,
    EmailVerificationQuery, ResendVerificationRequest
};
use crate::db::DatabaseManager;
use crate::errors::AiStudioError;
use crate::api::extractors::AuthExtractor;
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
        .peer_addr()
        .map(|s| s.to_string());
    
    let user_agent = req
        .headers()
        .get("User-Agent")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());

    let response = service.login(request.into_inner(), client_ip, user_agent).await?;

    HttpResponseBuilder::ok(response)
}

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

pub async fn register(
    request: web::Json<RegisterRequest>,
) -> ActixResult<HttpResponse> {
    let db_manager = DatabaseManager::get()?;
    let service = AuthService::new(
        db_manager.get_connection().clone(),
        "default_jwt_secret".to_string(), // 应该从配置中获取
        None,
        None,
    );

    let response = service.register(request.into_inner()).await?;

    HttpResponseBuilder::created(response)
}

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

pub async fn confirm_password_reset(
    request: web::Json<PasswordResetConfirmRequest>,
) -> ActixResult<HttpResponse> {
    let db_manager = DatabaseManager::get()?;
    let service = AuthService::new(
        db_manager.get_connection().clone(),
        "default_jwt_secret".to_string(),
        None,
        None,
    );

    service.confirm_password_reset(request.into_inner()).await?;

    HttpResponseBuilder::no_content()
}

pub async fn get_current_user(
    auth: AuthExtractor,
) -> ActixResult<HttpResponse> {
    let db_manager = DatabaseManager::get()?;
    let db = db_manager.get_connection();

    // 从数据库获取最新的用户信息
    let user = crate::db::entities::user::Entity::find_by_id(auth.user_id)
        .one(db)
        .await
        .map_err(|e| AiStudioError::internal(format!("数据库错误: {}", e)))?
        .ok_or_else(|| AiStudioError::not_found("用户"))?;

    let user_info = crate::services::auth::UserInfo {
        id: user.id,
        tenant_id: user.tenant_id,
        username: user.username,
        email: user.email,
        display_name: user.display_name,
        avatar_url: user.avatar_url,
        role: user.role.to_string(),
        permissions: serde_json::from_value(user.permissions).unwrap_or_default(),
        last_login_at: user.last_login_at.map(|dt| dt.into()),
        created_at: user.created_at.into(),
    };

    HttpResponseBuilder::ok(user_info)
}

pub async fn verify_email(
    _query: web::Query<EmailVerificationQuery>,
) -> ActixResult<HttpResponse> {
    // 这里应该实现邮箱验证逻辑
    // 为了简化，这里只返回成功响应
    HttpResponseBuilder::no_content()
}

pub async fn resend_verification_email(
    _request: web::Json<ResendVerificationRequest>,
) -> ActixResult<HttpResponse> {
    // 这里应该实现重新发送验证邮件的逻辑
    // 为了简化，这里只返回成功响应
    HttpResponseBuilder::no_content()
}

pub fn auth_routes(cfg: &mut web::ServiceConfig) {
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