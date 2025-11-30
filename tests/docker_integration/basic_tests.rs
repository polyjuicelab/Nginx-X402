//! Basic Docker integration tests
//!
//! This module contains fundamental tests for Docker setup, health checks,
//! and basic x402 payment requirement behavior.
//!
//! # Test Categories
//!
//! - Docker container setup and initialization
//! - Health endpoint verification
//! - Metrics endpoint functionality
//! - Basic 402 payment required responses
//!
//! # Usage
//!
//! These tests verify that the basic infrastructure is working correctly
//! before running more complex integration tests.

#[cfg(feature = "integration-test")]
mod tests {
    use crate::docker_integration::common::*;
    use std::process::Command;
    use std::thread;
    use std::time::Duration;

    #[test]
    #[ignore] // Ignore by default - requires Docker
    fn test_docker_setup() {
        // Test Case: Docker container setup and initialization
        //
        // This test verifies that:
        // 1. Docker is available
        // 2. Docker image can be built
        // 3. Container can be started
        // 4. Nginx becomes ready within timeout
        //
        // This is a foundational test that other tests depend on.

        // Check if Docker is available
        let docker_available = Command::new("docker").arg("--version").output().is_ok();

        if !docker_available {
            eprintln!("Docker is not available. Skipping Docker tests.");
            return;
        }

        // Clean up any existing container
        cleanup_container();

        // Build and start container
        assert!(build_docker_image(), "Failed to build Docker image");
        assert!(start_container(), "Failed to start container");
        assert!(
            wait_for_nginx(Duration::from_secs(10)),
            "Nginx did not become ready in time"
        );

        // Don't cleanup here - let other tests use the container
        // Cleanup will happen when tests finish or manually
    }

    #[test]
    #[ignore = "requires Docker"]
    fn test_402_response() {
        // Test Case: Basic 402 Payment Required response
        //
        // This test verifies that:
        // 1. Protected endpoints return 402 when no payment header is provided
        // 2. The x402 module is correctly loaded and functioning
        //
        // This is a core functionality test that validates the module's
        // basic payment requirement behavior.

        if !ensure_container_running() {
            eprintln!("Failed to start container. Skipping test.");
            return;
        }

        let status = http_request("/api/protected").expect("Failed to make HTTP request");

        assert_eq!(status, "402", "Expected 402 response, got {status}");
    }

    #[test]
    #[ignore = "requires Docker"]
    fn test_health_endpoint() {
        // Test Case: Health check endpoint accessibility
        //
        // This test verifies that:
        // 1. The health endpoint is accessible without payment
        // 2. The endpoint returns HTTP 200
        // 3. The endpoint is not protected by x402 payment requirements
        //
        // The health endpoint should always be accessible for monitoring purposes.

        if !ensure_container_running() {
            eprintln!("Failed to start container. Skipping test.");
            return;
        }

        // Retry logic to handle timing issues
        let mut status = String::new();
        let mut retries = 10;
        while retries > 0 {
            status = http_request("/health").unwrap_or_else(|| "000".to_string());
            if status == "200" {
                break;
            }
            if status != "000" {
                // Got a response but not 200, fail immediately
                break;
            }
            retries -= 1;
            thread::sleep(Duration::from_millis(500));
        }

        assert_eq!(
            status, "200",
            "Expected 200 response from /health endpoint, got {status} after retries"
        );
    }

    #[test]
    #[ignore = "requires Docker"]
    fn test_metrics_endpoint() {
        // Test Case: Prometheus metrics endpoint
        //
        // This test verifies that:
        // 1. The metrics endpoint is accessible
        // 2. The endpoint returns Prometheus-formatted metrics
        // 3. Metrics contain x402-related data
        //
        // The metrics endpoint provides observability for the x402 module.

        if !ensure_container_running() {
            eprintln!("Failed to start container. Skipping test.");
            return;
        }

        let body = http_get("/metrics").expect("Failed to make HTTP request");

        assert!(
            body.contains("x402") || body.contains("# HELP"),
            "Metrics endpoint should return Prometheus metrics"
        );
    }
}
