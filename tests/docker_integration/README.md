# Docker Integration Tests

This directory contains Docker-based integration tests for the nginx-x402 module, organized into multiple module files with each file not exceeding 500 lines for better maintainability and navigation.

## 📁 File Structure

```
docker_integration/
├── README.md                    # This document
├── mod.rs                       # Module organization file
├── common.rs                    # Shared utility functions (342 lines)
├── basic_tests.rs               # Basic tests (147 lines)
├── http_method_tests.rs         # HTTP method tests (258 lines)
├── proxy_payment_tests.rs       # Proxy and payment verification tests (188 lines)
├── websocket_subrequest_tests.rs # WebSocket and subrequest tests (251 lines)
├── content_type_tests.rs        # Content type tests (155 lines)
└── config_tests.rs              # Configuration tests (238 lines)
```

## 🎯 Module Descriptions

### `common.rs` - Shared Utility Functions

Provides shared utility functions used by all test modules:

- **Docker Management**:
  - `build_docker_image()` - Build Docker test image
  - `start_container()` - Start Docker container
  - `cleanup_container()` - Clean up Docker container
  - `ensure_container_running()` - Ensure container is running (auto-start)

- **Nginx Status Checks**:
  - `nginx_is_ready()` - Check if nginx is ready
  - `wait_for_nginx()` - Wait for nginx to be ready (with retry logic)

- **HTTP Request Utilities**:
  - `http_request()` - Send HTTP request and return status code
  - `http_get()` - Send HTTP request and return response body
  - `http_request_with_headers()` - HTTP request with custom headers
  - `http_request_with_method()` - Request with specified HTTP method

**Usage Principle**: All test modules should use these shared functions to avoid code duplication.

### `basic_tests.rs` - Basic Tests

Tests Docker setup and basic functionality:

- ✅ `test_docker_setup()` - Docker container setup and initialization
- ✅ `test_402_response()` - Basic 402 payment required response
- ✅ `test_health_endpoint()` - Health check endpoint accessibility
- ✅ `test_metrics_endpoint()` - Prometheus metrics endpoint

**Test Focus**: Verifies that infrastructure is working correctly and the module is properly loaded.

### `http_method_tests.rs` - HTTP Method Tests

Tests how different HTTP methods handle payment verification:

- ✅ `test_options_request_skips_payment()` - OPTIONS request (CORS preflight) should skip payment
- ✅ `test_head_request_skips_payment()` - HEAD request should skip payment
- ✅ `test_trace_request_skips_payment()` - TRACE request should skip payment
- ✅ `test_get_request_still_requires_payment()` - GET request still requires payment

**Test Focus**: Verifies that certain HTTP methods (OPTIONS, HEAD, TRACE) should bypass payment verification, while normal requests like GET still require payment.

### `proxy_payment_tests.rs` - Proxy and Payment Verification Tests

Tests the interaction between x402 payment verification and nginx proxy_pass:

- ✅ `test_proxy_pass_without_payment()` - proxy_pass without payment header should return 402
- ✅ `test_proxy_pass_with_invalid_payment()` - Invalid payment header should not proxy to backend
- ✅ `test_proxy_pass_verification_order()` - Payment verification should happen before proxy_pass

**Test Focus**: Verifies that payment verification executes in ACCESS_PHASE, before proxy_pass's CONTENT_PHASE, ensuring unpaid requests don't reach the backend.

### `websocket_subrequest_tests.rs` - WebSocket and Subrequest Tests

Tests special request types:

- ✅ `test_websocket_upgrade()` - WebSocket upgrade request handling
- ✅ `test_subrequest_detection()` - Subrequest detection (should skip payment)
- ✅ `test_internal_redirect_error_page()` - Internal redirect (error_page) handling

**Test Focus**: Verifies payment verification behavior for WebSocket and subrequest scenarios.

### `content_type_tests.rs` - Content Type Tests

Tests response format detection (JSON vs HTML):

- ✅ `test_content_type_json_returns_json_response()` - Content-Type: application/json should return JSON
- ✅ `test_content_type_json_without_user_agent()` - Content-Type alone should return JSON
- ✅ `test_browser_request_without_content_type_returns_html()` - Browser request should return HTML

**Test Focus**: Verifies that the module correctly returns JSON (API clients) or HTML (browsers) based on request headers.

### `config_tests.rs` - Configuration Tests

Tests various x402 configuration options:

- ✅ `test_asset_fallback_uses_default_usdc()` - Default USDC when asset not specified
- ✅ `test_network_id_configuration()` - network_id configuration (chainId)
- ✅ `test_network_id_mainnet()` - Mainnet network_id
- ✅ `test_custom_asset_address()` - Custom asset address
- ✅ `test_network_id_takes_precedence()` - network_id takes precedence over network

**Test Focus**: Verifies correct behavior of various configuration options, including defaults and precedence.

## 🚀 Running Tests

### Run All Tests

```bash
cargo test --test docker_integration_test --features integration-test
```

### Run Specific Module

```bash
# Basic tests
cargo test --test docker_integration_test basic_tests --features integration-test

# HTTP method tests
cargo test --test docker_integration_test http_method_tests --features integration-test

# Proxy and payment verification tests
cargo test --test docker_integration_test proxy_payment_tests --features integration-test

# WebSocket and subrequest tests
cargo test --test docker_integration_test websocket_subrequest_tests --features integration-test

# Content type tests
cargo test --test docker_integration_test content_type_tests --features integration-test

# Configuration tests
cargo test --test docker_integration_test config_tests --features integration-test
```

### Run Single Test

```bash
cargo test --test docker_integration_test test_402_response --features integration-test
```

### Run with Output

```bash
cargo test --test docker_integration_test --features integration-test -- --nocapture
```

## 📝 Development Guidelines

### Adding New Tests

1. **Choose the Right Module**: Place tests in the most appropriate module file based on functionality
2. **Use Shared Utilities**: Use functions from the `common` module to avoid code duplication
3. **Detailed Comments**: Add comments explaining test purpose and expected behavior
4. **Naming Convention**: Use descriptive test names that clearly explain what is being tested
5. **Stay Focused**: Each test should verify one specific behavior

### Module Size Principles

- ✅ Each module file should be ≤ 500 lines
- ✅ If a module grows too large, consider splitting it further
- ✅ Shared utilities should always be in the `common` module

### Test Structure

Each test should follow this structure:

```rust
#[test]
#[ignore = "requires Docker"]
fn test_example() {
    // 1. Test purpose explanation
    // 2. Expected behavior explanation
    
    if !ensure_container_running() {
        eprintln!("Failed to start container. Skipping test.");
        return;
    }
    
    // 3. Execute test
    // 4. Verify results
    // 5. Output success message
}
```

## 🔍 Test Coverage

### Functional Coverage

- ✅ Docker container management
- ✅ Basic payment required response (402)
- ✅ HTTP method handling (GET, POST, OPTIONS, HEAD, TRACE)
- ✅ Proxy and payment verification interaction
- ✅ WebSocket upgrade
- ✅ Subrequest detection
- ✅ Response format detection (JSON/HTML)
- ✅ Configuration options (asset, network, network_id)

### Edge Cases

- ✅ No payment header
- ✅ Invalid payment header
- ✅ Container not running
- ✅ Nginx not ready
- ✅ Network errors

## 🐛 Troubleshooting

### Docker-Related Issues

If tests fail, check:

1. **Is Docker running**: `docker ps`
2. **Container status**: `docker ps -a | grep nginx-x402-test-container`
3. **Container logs**: `docker logs nginx-x402-test-container`
4. **Clean up container**: `docker stop nginx-x402-test-container && docker rm nginx-x402-test-container`

### Nginx-Related Issues

If nginx is not ready:

1. Check if container is running: `docker ps`
2. Check nginx logs: `docker logs nginx-x402-test-container`
3. Manually test health endpoint: `curl http://localhost:8080/health`

### Test Timeouts

If tests timeout:

1. Increase retry count or timeout duration
2. Check system resources (CPU, memory)
3. Check network connectivity

## 📚 Related Documentation

- [Main Test Directory README](../README.md)
- [Integration Test Status](../INTEGRATION_TEST_STATUS.md)
- [Test Summary](../TEST_SUMMARY.md)

## 🤝 Contributing

When adding new tests:

1. Follow existing code style and structure
2. Add detailed comments and documentation
3. Ensure tests are in the correct module
4. Verify file size does not exceed 500 lines
5. Run all tests to ensure no existing functionality is broken
