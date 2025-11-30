//! Docker-based integration tests for nginx-x402 module
//!
//! These tests use Docker to run nginx with the module in an isolated environment.
//! Requires Docker to be installed and running.
//!
//! # Test Organization
//!
//! This test suite has been split into multiple modules for better organization and maintainability:
//!
//! - **Basic Tests** (`basic_tests`): Docker setup, health checks, metrics, basic 402 responses
//! - **HTTP Method Tests** (`http_method_tests`): OPTIONS, HEAD, TRACE, GET request handling
//! - **Proxy Payment Tests** (`proxy_payment_tests`): Interaction between payment verification and proxy_pass
//! - **WebSocket/Subrequest Tests** (`websocket_subrequest_tests`): WebSocket upgrades and subrequest detection
//! - **Content Type Tests** (`content_type_tests`): JSON vs HTML response format detection
//! - **Config Tests** (`config_tests`): Configuration options (asset, network_id, etc.)
//!
//! # Running Tests
//!
//! To run all Docker integration tests:
//! ```bash
//! cargo test --test docker_integration_test --features integration-test
//! ```
//!
//! To run a specific test module:
//! ```bash
//! cargo test --test docker_integration_test basic_tests --features integration-test
//! ```
//!
//! To run a specific test:
//! ```bash
//! cargo test --test docker_integration_test test_402_response --features integration-test
//! ```
//!
//! # Test Structure
//!
//! Each test module follows these principles:
//!
//! 1. **Clear Purpose**: Each test has a descriptive name and detailed comments
//! 2. **Expected Behavior**: Tests document what should happen and why
//! 3. **Error Handling**: Tests gracefully handle Docker/nginx unavailability
//! 4. **Retry Logic**: Tests include retry logic for timing-sensitive operations
//!
//! # Development Guidelines
//!
//! When adding new tests:
//!
//! 1. **Choose the Right Module**: Place tests in the most appropriate module
//! 2. **Use Common Utilities**: Leverage functions from `common` module
//! 3. **Document Thoroughly**: Include comments explaining test purpose and expected behavior
//! 4. **Follow Naming**: Use descriptive test names that explain what is being tested
//! 5. **Keep Tests Focused**: Each test should verify one specific behavior
//!
//! # Module Size Guidelines
//!
//! To maintain readability and ease of navigation:
//!
//! - Each module file should be ≤ 500 lines
//! - If a module grows too large, consider splitting it further
//! - Common utilities should always be in the `common` module
//!
//! Note: This requires the 'integration-test' feature to be enabled.

#[cfg(feature = "integration-test")]
mod docker_integration;

// Re-export modules for easier access (optional, for documentation purposes)
#[cfg(feature = "integration-test")]
pub use docker_integration::*;
