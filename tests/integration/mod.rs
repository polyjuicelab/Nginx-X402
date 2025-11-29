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
mod common;

#[cfg(feature = "integration-test")]
mod http_helpers;

#[cfg(feature = "integration-test")]
mod basic;

#[cfg(feature = "integration-test")]
mod proxy_pass;

#[cfg(feature = "integration-test")]
mod options;

#[cfg(feature = "integration-test")]
mod options_cors;

#[cfg(feature = "integration-test")]
mod head;

#[cfg(feature = "integration-test")]
mod head_cors;

#[cfg(feature = "integration-test")]
mod trace;

#[cfg(feature = "integration-test")]
mod config;

#[cfg(feature = "integration-test")]
mod resource;
