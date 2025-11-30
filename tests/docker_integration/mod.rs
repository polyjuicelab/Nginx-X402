//! Docker integration test modules
//!
//! This module organizes Docker-based integration tests into logical groups.
//! Each submodule focuses on a specific aspect of x402 functionality.
//!
//! # Module Structure
//!
//! - `common`: Shared utilities and helper functions
//! - `basic_tests`: Docker setup, health checks, basic 402 responses
//! - `http_method_tests`: HTTP method handling (OPTIONS, HEAD, TRACE, GET)
//! - `proxy_payment_tests`: Proxy and payment verification interaction
//! - `websocket_subrequest_tests`: WebSocket and subrequest handling
//! - `content_type_tests`: Response format detection (JSON vs HTML)
//! - `config_tests`: Configuration options (asset, network, etc.)
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
//! # Test Organization Principles
//!
//! 1. **Single Responsibility**: Each module tests one aspect of functionality
//! 2. **Reusability**: Common utilities are shared via the `common` module
//! 3. **Documentation**: Each test includes detailed comments explaining purpose and expected behavior
//! 4. **Maintainability**: Tests are organized logically for easy navigation and updates

#[cfg(feature = "integration-test")]
pub mod common;

#[cfg(feature = "integration-test")]
pub mod basic_tests;

#[cfg(feature = "integration-test")]
pub mod http_method_tests;

#[cfg(feature = "integration-test")]
pub mod proxy_payment_tests;

#[cfg(feature = "integration-test")]
pub mod websocket_subrequest_tests;

#[cfg(feature = "integration-test")]
pub mod content_type_tests;

#[cfg(feature = "integration-test")]
pub mod config_tests;

