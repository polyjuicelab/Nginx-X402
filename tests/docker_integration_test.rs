//! Docker-based integration tests for nginx-x402 module
//!
//! These tests use Docker to run nginx with the module in an isolated environment.
//! Requires Docker to be installed and running.
//!
//! To run:
//!   cargo test --test docker_integration_test --features integration-test
//!
//! Note: This requires the 'integration-test' feature to be enabled.

#[cfg(feature = "integration-test")]
mod integration;
