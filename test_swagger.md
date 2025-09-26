# Swagger UI 测试

## 可访问的 Swagger 路径

1. **主要 Swagger UI**: http://127.0.0.1:8080/api/v1/docs/
2. **备用 Swagger UI**: http://127.0.0.1:8080/docs/
3. **OpenAPI JSON**: http://127.0.0.1:8080/api/v1/openapi.json

## 测试步骤

1. 启动服务器: `cargo run --bin aionix`
2. 访问上述任一 URL
3. 验证 Swagger UI 界面是否正常显示
4. 检查 API 文档是否完整

## 预期结果

- Swagger UI 界面应该显示完整的 API 文档
- 包含所有已定义的端点
- 可以进行 API 测试

## 当前配置

- utoipa: 4.x
- utoipa-swagger-ui: 6.x
- 支持 actix-web 集成