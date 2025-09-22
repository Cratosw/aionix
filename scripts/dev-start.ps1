# 开发环境启动脚本

Write-Host "🚀 启动 Aionix AI Studio 开发环境..." -ForegroundColor Green

# 检查 Rust 环境
Write-Host "📋 检查 Rust 环境..." -ForegroundColor Yellow
if (!(Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Host "❌ 未找到 Cargo，请先安装 Rust" -ForegroundColor Red
    exit 1
}

# 检查 PostgreSQL
Write-Host "📋 检查 PostgreSQL 连接..." -ForegroundColor Yellow
$env:DATABASE_URL = if ($env:DATABASE_URL) { $env:DATABASE_URL } else { "postgresql://postgres:password@localhost:5432/aionix" }
Write-Host "数据库连接: $env:DATABASE_URL" -ForegroundColor Cyan

# 检查 Redis
Write-Host "📋 检查 Redis 连接..." -ForegroundColor Yellow
$env:REDIS_URL = if ($env:REDIS_URL) { $env:REDIS_URL } else { "redis://localhost:6379" }
Write-Host "Redis 连接: $env:REDIS_URL" -ForegroundColor Cyan

# 设置其他环境变量
$env:JWT_SECRET = if ($env:JWT_SECRET) { $env:JWT_SECRET } else { "your-super-secret-jwt-key-change-in-production" }
$env:RUST_LOG = if ($env:RUST_LOG) { $env:RUST_LOG } else { "info,aionix=debug" }
$env:SERVER_HOST = if ($env:SERVER_HOST) { $env:SERVER_HOST } else { "127.0.0.1" }
$env:SERVER_PORT = if ($env:SERVER_PORT) { $env:SERVER_PORT } else { "8080" }

Write-Host "🔧 环境变量配置完成" -ForegroundColor Green
Write-Host "  - JWT_SECRET: ****" -ForegroundColor Cyan
Write-Host "  - RUST_LOG: $env:RUST_LOG" -ForegroundColor Cyan
Write-Host "  - SERVER: $env:SERVER_HOST`:$env:SERVER_PORT" -ForegroundColor Cyan

# 编译检查
Write-Host "🔍 执行编译检查..." -ForegroundColor Yellow
$checkResult = cargo check --message-format=short 2>&1
if ($LASTEXITCODE -ne 0) {
    Write-Host "⚠️  编译检查发现问题，但继续启动..." -ForegroundColor Yellow
    Write-Host "编译输出:" -ForegroundColor Gray
    $checkResult | Select-Object -First 10
    Write-Host "..." -ForegroundColor Gray
} else {
    Write-Host "✅ 编译检查通过" -ForegroundColor Green
}

# 启动服务
Write-Host "🚀 启动 AI Studio 服务..." -ForegroundColor Green
Write-Host "访问地址: http://$env:SERVER_HOST`:$env:SERVER_PORT" -ForegroundColor Cyan
Write-Host "API 文档: http://$env:SERVER_HOST`:$env:SERVER_PORT/api/v1/docs" -ForegroundColor Cyan
Write-Host "健康检查: http://$env:SERVER_HOST`:$env:SERVER_PORT/api/v1/health" -ForegroundColor Cyan
Write-Host ""
Write-Host "按 Ctrl+C 停止服务" -ForegroundColor Yellow
Write-Host "===========================================" -ForegroundColor Green

try {
    cargo run
} catch {
    Write-Host "❌ 启动失败: $_" -ForegroundColor Red
    exit 1
}