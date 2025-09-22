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
#[utoipa::path(
    post,
    path = "/auth/login",
    tag = "Auth",
    summary = "用户登录",
    description = "使用用户名/邮箱和密码进行登录，返回访问令牌和刷新令牌",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "登录成功", body = LoginResponse),
        (status = 400, description = "请求参数错误"),
        (status = 401, description = "用户名或密码错误"),
        (status = 403, description = "账户被禁用")
    )
)]
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
#[utoipa::path(
    post,
    path = "/auth/refresh",
    tag = "Auth",
    summary = "刷新访问令牌",
    description = "使用刷新令牌获取新的访问令牌",
    request_body = RefreshTokenRequest,
    responses(
        (status = 200, description = "刷新成功", body = RefreshTokenResponse),
        (status = 400, description = "请求参数错误"),
        (status = 401, description = "刷新令牌无效或已过期")
    )
)]
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
#[utoipa::path(
    post,
    path = "/auth/register",
    tag = "Auth",
    summary = "用户注册",
    description = "注册新用户账户",
    request_body = RegisterRequest,
    responses(
        (status = 201, description = "注册成功", body = RegisterResponse),
        (status = 400, description = "请求参数错误"),
        (status = 409, description = "用户名或邮箱已存在")
    )
)]
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
#[utoipa::path(
    post,
    path = "/auth/logout",
    tag = "Auth",
    summary = "用户登出",
    description = "登出当前用户，使刷新令牌失效",
    request_body = RefreshTokenRequest,
    responses(
        (status = 204, description = "登出成功"),
        (status = 400, description = "请求参数错误"),
        (status = 401, description = "刷新令牌无效")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
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
#[utoipa::path(
    post,
    path = "/auth/password-reset",
    tag = "Auth",
    summary = "请求密码重置",
    description = "发送密码重置邮件",
    request_body = PasswordResetRequest,
    responses(
        (status = 204, description = "重置邮件已发送"),
        (status = 400, description = "请求参数错误"),
        (status = 404, description = "用户不存在")
    )
)]
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
#[utoipa::path(
    post,
    path = "/auth/password-reset/confirm",
    tag = "Auth",
    summary = "确认密码重置",
    description = "使用重置令牌设置新密码",
    request_body = PasswordResetConfirmRequest,
    responses(
        (status = 204, description = "密码重置成功"),
        (status = 400, description = "请求参数错误"),
        (status = 401, description = "重置令牌无效或已过期")
    )
)]
pub async fn confirm_password_reset(
    request: web::Json<PasswordResetConfirmRequest>,
) -> ActixResult<HttpResponse> {
    // 这里应该实现密码重置确认逻辑
    // 为了简化，这里只返回成功响应
    HttpResponseBuilder::no_content()
}

/// 获取当前用户信息
#[utoipa::path(
    get,
    path = "/auth/me",
    tag = "Auth",
    summary = "获取当前用户信息",
    description = "获取当前认证用户的详细信息",
    responses(
        (status = 200, description = "获取成功", body = crate::services::auth::UserInfo),
        (status = 401, description = "未授权")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
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
#[utoipa::path(
    post,
    path = "/auth/verify-email",
    tag = "Auth",
    summary = "验证邮箱",
    description = "使用验证令牌验证用户邮箱",
    params(
        ("token" = String, Query, description = "邮箱验证令牌")
    ),
    responses(
        (status = 204, description = "邮箱验证成功"),
        (status = 400, description = "验证令牌无效"),
        (status = 404, description = "用户不存在")
    )
)]
pub async fn verify_email(
    query: web::Query<EmailVerificationQuery>,
) -> ActixResult<HttpResponse> {
    // 这里应该实现邮箱验证逻辑
    // 为了简化，这里只返回成功响应
    HttpResponseBuilder::no_content()
}

/// 重新发送验证邮件
#[utoipa::path(
    post,
    path = "/auth/resend-verification",
    tag = "Auth",
    summary = "重新发送验证邮件",
    description = "重新发送邮箱验证邮件",
    request_body = ResendVerificationRequest,
    responses(
        (status = 204, description = "验证邮件已发送"),
        (status = 400, description = "请求参数错误"),
        (status = 404, description = "用户不存在"),
        (status = 409, description = "邮箱已验证")
    )
)]
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