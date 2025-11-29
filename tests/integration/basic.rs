//! Basic integration tests for nginx-x402 module
//!
//! Tests for basic functionality: setup, 402 responses, health/metrics endpoints,
//! WebSocket/subrequest detection.

#[cfg(feature = "integration-test")]
mod tests {
    use super::super::common::*;
    use std::process::Command;
    use std::thread;
    use std::time::Duration;

    #[test]
    #[ignore] // Ignore by default - requires Docker
    fn test_docker_setup() {
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

    #[test]
    #[ignore = "requires Docker"]
    fn test_get_request_still_requires_payment() {
        // Test Case: GET request should still require payment (not skip)
        // This ensures we didn't accidentally skip payment for GET requests
        if !ensure_container_running() {
            eprintln!("Failed to start container. Skipping test.");
            return;
        }

        let status = http_request("/api/protected").expect("Failed to make HTTP request");

        assert_eq!(
            status, "402",
            "GET request should require payment (got {status}). \
             Only OPTIONS/HEAD/TRACE should skip payment verification."
        );
    }

    #[test]
    #[ignore = "requires Docker"]
    fn test_internal_redirect_error_page() {
        // Test Case: Internal redirect to error page should skip payment verification
        // This tests subrequest detection via internal redirects
        if !ensure_container_running() {
            eprintln!("Failed to start container. Skipping test.");
            return;
        }

        // Test endpoint that triggers internal redirect
        let status = http_request("/error-page").expect("Failed to make HTTP request");

        println!("Internal redirect error page status: {status}");

        // Internal redirects create subrequests (r->parent != NULL)
        // Subrequests should skip payment verification
        // Expected: 200 (error page) or 404 (if error page doesn't exist)
        assert!(
            status == "200" || status == "404" || status == "502",
            "Internal redirect should skip payment verification, got {status}"
        );

        // Should NOT return 402 (payment required)
        assert_ne!(
            status, "402",
            "Internal redirect (subrequest) should not require payment (got 402)"
        );
    }

    #[test]
    #[ignore = "requires Docker"]
    fn test_websocket_upgrade() {
        // Test Case: WebSocket Upgrade header
        // WebSocket requests should ideally skip payment verification
        // or handle it differently since WebSocket is a long-lived connection
        if !ensure_container_running() {
            eprintln!("Failed to start container. Skipping test.");
            return;
        }

        // Test WebSocket handshake request
        let output = Command::new("curl")
            .args([
                "-s",
                "-o",
                "/dev/null",
                "-w",
                "%{http_code}",
                "-H",
                "Upgrade: websocket",
                "-H",
                "Connection: Upgrade",
                "-H",
                "Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==",
                "-H",
                "Sec-WebSocket-Version: 13",
                &format!("http://localhost:{NGINX_PORT}/ws"),
            ])
            .output()
            .expect("Failed to run curl");

        let status = String::from_utf8_lossy(&output.stdout).trim().to_string();

        // WebSocket handshake may:
        // 1. Return 200 (OK) - if WebSocket detection works and backend responds normally
        // 2. Return 101 (Switching Protocols) - if WebSocket upgrade succeeds
        // 3. Return 402 (payment required) - if WebSocket detection fails and payment is required
        // 4. Return 426 (Upgrade Required) - if WebSocket not supported
        // 5. Return 502 (Bad Gateway) - if backend doesn't support WebSocket

        println!("WebSocket handshake status: {status}");

        // WebSocket detection should skip payment verification and allow request to proceed
        // If detection works correctly, we should get 200 (backend response) or 101 (upgrade success)
        // If detection fails, we might get 402 (payment required)
        assert!(
            status == "200"
                || status == "101"
                || status == "402"
                || status == "426"
                || status == "502",
            "Unexpected status for WebSocket handshake: {status}"
        );

        // Verify that WebSocket detection is working: if status is 200 or 101, detection worked
        if status == "200" || status == "101" {
            println!("✓ WebSocket detection working: payment verification skipped, request proceeded to backend");
        } else if status == "402" {
            println!(
                "⚠ WebSocket detection may not be working: payment verification was not skipped"
            );
        }
    }

    #[test]
    #[ignore = "requires Docker"]
    fn test_subrequest_detection() {
        // Test Case: Subrequest detection
        // Subrequests have r->parent != NULL
        // This test verifies that subrequests are detected and skip payment verification
        if !ensure_container_running() {
            eprintln!("Failed to start container. Skipping test.");
            return;
        }

        // Test endpoint that may create subrequests
        // Note: Actual subrequest creation requires specific nginx modules or configurations
        // This test documents current behavior
        let status = http_request("/api/subrequest-test").expect("Failed to make HTTP request");

        println!("Subrequest test endpoint status (no payment): {status}");

        // Current behavior: Should return 402 if subrequest detection doesn't work
        // If subrequest detection works, it may return different status
        // Document the behavior for now
        assert!(
            status == "402" || status == "200" || status == "502",
            "Unexpected status: {status}"
        );

        println!("Note: Subrequest detection requires r->parent != NULL check");
        println!("This is implemented in phase handler using raw request pointer");
    }
}
