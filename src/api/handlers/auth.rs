// 认证 API 处理器

use actix_web::{web, HttpRequest, HttpResponse, Result as ActixResult};

use sea_orm::EntityTrait;
use crate::api::responses::HttpResponseBuilder;
use crate::services::auth::{
    AuthService, LoginRequest, RefreshTokenRequest,
    RegisterRequest, PasswordResetRequest, PasswordResetConfirmRequest,
    EmailVerificationQuery, ResendVerificationRequest
};
use crate::db::DatabaseManager;
use crate::errors::AiStudioError;
use crate::api::extractors::AuthExtractor;

///用户登录
#[utoipa::path(
    post,
    path = "/auth/login",
    tag = "auth",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "登录成功", body = LoginResponse),
        (status = 401, description = "认证失败", body = ApiError)
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
        .peer_addr()
        .map(|s| s.to_string());
    
    let response = service.login(request.into_inner(), client_ip, None).await?;

    HttpResponseBuilder::ok(response)
}

///刷新令牌
#[utoipa::path(
    post,
    path = "/auth/refresh",
    tag = "auth",
    request_body = RefreshTokenRequest,
    responses(
        (status = 200, description = "令牌刷新成功", body = LoginResponse),
        (status = 401, description = "刷新令牌无效", body = ApiError)
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

///用户注册
#[utoipa::path(
    post,
    path = "/auth/register",
    tag = "auth",
    request_body = RegisterRequest,
    responses(
        (status = 201, description = "注册成功", body = RegisterResponse),
        (status = 400, description = "注册参数错误", body = ApiError),
        (status = 409, description = "用户已存在", body = ApiError)
    )
)]
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

///用户登出
#[utoipa::path(
    post,
    path = "/auth/logout",
    tag = "auth",
    request_body = RefreshTokenRequest,
    responses(
        (status = 204, description = "登出成功"),
        (status = 401, description = "令牌无效", body = ApiError)
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

///请求密码重置
#[utoipa::path(
    post,
    path = "/auth/password-reset",
    tag = "auth",
    request_body = PasswordResetRequest,
    responses(
        (status = 204, description = "重置邮件已发送"),
        (status = 404, description = "用户不存在", body = ApiError)
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

///确认密码重置
#[utoipa::path(
    post,
    path = "/auth/password-reset/confirm",
    tag = "auth",
    request_body = PasswordResetConfirmRequest,
    responses(
        (status = 204, description = "密码重置成功"),
        (status = 400, description = "重置令牌无效", body = ApiError)
    )
)]
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

///获取当前用户信息
#[utoipa::path(
    get,
    path = "/auth/me",
    tag = "auth",
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "用户信息", body = UserInfo),
        (status = 401, description = "未认证", body = ApiError)
    )
)]
pub async fn get_current_user(
    auth: AuthExtractor,
) -> ActixResult<HttpResponse> {
    HttpResponseBuilder::ok(auth.user_info)
}

///更新用户资料
#[utoipa::path(
    put,
    path = "/auth/profile",
    tag = "auth",
    security(
        ("bearer_auth" = [])
    ),
    request_body = UpdateUserProfileRequest,
    responses(
        (status = 200, description = "资料更新成功", body = UserInfo),
        (status = 401, description = "未认证", body = ApiError),
        (status = 400, description = "参数错误", body = ApiError)
    )
)]
pub async fn update_user_profile(
    auth: AuthExtractor,
    request: web::Json<UpdateUserProfileRequest>,
) -> ActixResult<HttpResponse> {
    let db_manager = DatabaseManager::get()?;
    let service = AuthService::new(
        db_manager.get_connection().clone(),
        "default_jwt_secret".to_string(),
        None,
        None,
    );

    let updated_user = service.update_user_profile(auth.user_info.id, request.into_inner()).await?;

    HttpResponseBuilder::ok(updated_user)
}

// 配置认证路由
pub fn configure_auth_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/auth")
            .route("/login", web::post().to(login))
            .route("/logout", web::post().to(logout))
            .route("/refresh", web::post().to(refresh_token))
            .route("/register", web::post().to(register))
            .route("/password-reset", web::post().to(request_password_reset))
            .route("/password-reset/confirm", web::post().to(confirm_password_reset))
            .route("/me", web::get().to(get_current_user))
            .route("/profile", web::put().to(update_user_profile))
    );
}

// 临时类型定义，应该移到适当的模块中
#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct UpdateUserProfileRequest {
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub avatar_url: Option<String>,
}