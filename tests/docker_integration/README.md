# Docker Integration Tests

本目录包含 nginx-x402 模块的 Docker 集成测试，已按功能拆分为多个模块文件，每个文件不超过 500 行，便于维护和导航。

## 📁 文件结构

```
docker_integration/
├── README.md                    # 本文档
├── mod.rs                       # 模块组织文件
├── common.rs                    # 共享工具函数 (342行)
├── basic_tests.rs               # 基础测试 (147行)
├── http_method_tests.rs         # HTTP方法测试 (258行)
├── proxy_payment_tests.rs       # 代理和支付验证测试 (188行)
├── websocket_subrequest_tests.rs # WebSocket和子请求测试 (251行)
├── content_type_tests.rs        # 内容类型测试 (155行)
└── config_tests.rs              # 配置测试 (238行)
```

## 🎯 模块说明

### `common.rs` - 共享工具函数

提供所有测试模块共享的工具函数：

- **Docker 管理**：
  - `build_docker_image()` - 构建 Docker 测试镜像
  - `start_container()` - 启动 Docker 容器
  - `cleanup_container()` - 清理 Docker 容器
  - `ensure_container_running()` - 确保容器运行（自动启动）

- **Nginx 状态检查**：
  - `nginx_is_ready()` - 检查 nginx 是否就绪
  - `wait_for_nginx()` - 等待 nginx 就绪（带重试逻辑）

- **HTTP 请求工具**：
  - `http_request()` - 发送 HTTP 请求并返回状态码
  - `http_get()` - 发送 HTTP 请求并返回响应体
  - `http_request_with_headers()` - 带自定义请求头的 HTTP 请求
  - `http_request_with_method()` - 指定 HTTP 方法的请求

**使用原则**：所有测试模块应使用这些共享函数，避免代码重复。

### `basic_tests.rs` - 基础测试

测试 Docker 设置和基本功能：

- ✅ `test_docker_setup()` - Docker 容器设置和初始化
- ✅ `test_402_response()` - 基本 402 支付要求响应
- ✅ `test_health_endpoint()` - 健康检查端点可访问性
- ✅ `test_metrics_endpoint()` - Prometheus 指标端点

**测试重点**：验证基础设施是否正常工作，模块是否正确加载。

### `http_method_tests.rs` - HTTP 方法测试

测试不同 HTTP 方法如何处理支付验证：

- ✅ `test_options_request_skips_payment()` - OPTIONS 请求（CORS 预检）应跳过支付
- ✅ `test_head_request_skips_payment()` - HEAD 请求应跳过支付
- ✅ `test_trace_request_skips_payment()` - TRACE 请求应跳过支付
- ✅ `test_get_request_still_requires_payment()` - GET 请求仍需要支付

**测试重点**：验证某些 HTTP 方法（OPTIONS、HEAD、TRACE）应绕过支付验证，而 GET 等正常请求仍需要支付。

### `proxy_payment_tests.rs` - 代理和支付验证测试

测试 x402 支付验证与 nginx proxy_pass 的交互：

- ✅ `test_proxy_pass_without_payment()` - 无支付头时 proxy_pass 应返回 402
- ✅ `test_proxy_pass_with_invalid_payment()` - 无效支付头时不应代理到后端
- ✅ `test_proxy_pass_verification_order()` - 支付验证应在 proxy_pass 之前执行

**测试重点**：验证支付验证在 ACCESS_PHASE 执行，早于 proxy_pass 的 CONTENT_PHASE，确保未支付请求不会到达后端。

### `websocket_subrequest_tests.rs` - WebSocket 和子请求测试

测试特殊请求类型：

- ✅ `test_websocket_upgrade()` - WebSocket 升级请求处理
- ✅ `test_subrequest_detection()` - 子请求检测（应跳过支付）
- ✅ `test_internal_redirect_error_page()` - 内部重定向（error_page）处理

**测试重点**：验证 WebSocket 和子请求等特殊场景的支付验证行为。

### `content_type_tests.rs` - 内容类型测试

测试响应格式检测（JSON vs HTML）：

- ✅ `test_content_type_json_returns_json_response()` - Content-Type: application/json 应返回 JSON
- ✅ `test_content_type_json_without_user_agent()` - 仅 Content-Type 也应返回 JSON
- ✅ `test_browser_request_without_content_type_returns_html()` - 浏览器请求应返回 HTML

**测试重点**：验证模块能根据请求头正确返回 JSON（API 客户端）或 HTML（浏览器）格式。

### `config_tests.rs` - 配置测试

测试各种 x402 配置选项：

- ✅ `test_asset_fallback_uses_default_usdc()` - 未指定资产时使用默认 USDC
- ✅ `test_network_id_configuration()` - network_id 配置（chainId）
- ✅ `test_network_id_mainnet()` - 主网 network_id
- ✅ `test_custom_asset_address()` - 自定义资产地址
- ✅ `test_network_id_takes_precedence()` - network_id 优先于 network

**测试重点**：验证各种配置选项的正确行为，包括默认值、优先级等。

## 🚀 运行测试

### 运行所有测试

```bash
cargo test --test docker_integration_test --features integration-test
```

### 运行特定模块

```bash
# 基础测试
cargo test --test docker_integration_test basic_tests --features integration-test

# HTTP 方法测试
cargo test --test docker_integration_test http_method_tests --features integration-test

# 代理和支付验证测试
cargo test --test docker_integration_test proxy_payment_tests --features integration-test

# WebSocket 和子请求测试
cargo test --test docker_integration_test websocket_subrequest_tests --features integration-test

# 内容类型测试
cargo test --test docker_integration_test content_type_tests --features integration-test

# 配置测试
cargo test --test docker_integration_test config_tests --features integration-test
```

### 运行单个测试

```bash
cargo test --test docker_integration_test test_402_response --features integration-test
```

### 运行并显示输出

```bash
cargo test --test docker_integration_test --features integration-test -- --nocapture
```

## 📝 开发指南

### 添加新测试

1. **选择正确的模块**：根据测试内容选择最合适的模块文件
2. **使用共享工具**：使用 `common` 模块中的函数，避免重复代码
3. **详细注释**：为每个测试添加注释，说明测试目的和预期行为
4. **命名规范**：使用描述性的测试名称，清楚说明测试内容
5. **保持专注**：每个测试应验证一个特定的行为

### 模块大小原则

- ✅ 每个模块文件应 ≤ 500 行
- ✅ 如果模块增长过大，考虑进一步拆分
- ✅ 共享工具应始终放在 `common` 模块中

### 测试结构

每个测试应遵循以下结构：

```rust
#[test]
#[ignore = "requires Docker"]
fn test_example() {
    // 1. 测试目的说明
    // 2. 预期行为说明
    
    if !ensure_container_running() {
        eprintln!("Failed to start container. Skipping test.");
        return;
    }
    
    // 3. 执行测试
    // 4. 验证结果
    // 5. 输出成功消息
}
```

## 🔍 测试覆盖范围

### 功能覆盖

- ✅ Docker 容器管理
- ✅ 基本支付要求响应（402）
- ✅ HTTP 方法处理（GET、POST、OPTIONS、HEAD、TRACE）
- ✅ 代理和支付验证交互
- ✅ WebSocket 升级
- ✅ 子请求检测
- ✅ 响应格式检测（JSON/HTML）
- ✅ 配置选项（asset、network、network_id）

### 边界情况

- ✅ 无支付头
- ✅ 无效支付头
- ✅ 容器未运行
- ✅ Nginx 未就绪
- ✅ 网络错误

## 🐛 故障排除

### Docker 相关问题

如果测试失败，检查：

1. **Docker 是否运行**：`docker ps`
2. **容器状态**：`docker ps -a | grep nginx-x402-test-container`
3. **容器日志**：`docker logs nginx-x402-test-container`
4. **清理容器**：`docker stop nginx-x402-test-container && docker rm nginx-x402-test-container`

### Nginx 相关问题

如果 nginx 未就绪：

1. 检查容器是否运行：`docker ps`
2. 检查 nginx 日志：`docker logs nginx-x402-test-container`
3. 手动测试健康端点：`curl http://localhost:8080/health`

### 测试超时

如果测试超时：

1. 增加重试次数或超时时间
2. 检查系统资源（CPU、内存）
3. 检查网络连接

## 📚 相关文档

- [主测试目录 README](../README.md)
- [集成测试状态](../INTEGRATION_TEST_STATUS.md)
- [测试总结](../TEST_SUMMARY.md)

## 🤝 贡献

添加新测试时，请：

1. 遵循现有的代码风格和结构
2. 添加详细的注释和文档
3. 确保测试在正确的模块中
4. 验证文件大小不超过 500 行
5. 运行所有测试确保没有破坏现有功能

