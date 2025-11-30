//! Proxy and payment verification tests
//!
//! This module tests the interaction between x402 payment verification and nginx proxy_pass.
//!
//! # Test Categories
//!
//! - proxy_pass without payment - should return 402, not proxy to backend
//! - proxy_pass with invalid payment - should return 402/500, not proxy to backend
//! - Payment verification order - should happen before proxy_pass handler runs
//!
//! # Background
//!
//! The x402 module runs in the ACCESS_PHASE of nginx request processing, which happens
//! before the CONTENT_PHASE where proxy_pass handlers typically run. This ensures that
//! payment verification occurs before proxying requests to backend servers.
//!
//! Key behavior:
//! - Payment verification happens in ACCESS_PHASE
//! - proxy_pass handler runs in CONTENT_PHASE (after ACCESS_PHASE)
//! - If payment verification fails, request should return 402 without reaching backend

#[cfg(feature = "integration-test")]
mod tests {
    use super::common::*;
    use std::process::Command;
    use std::thread;
    use std::time::Duration;

    #[test]
    #[ignore = "requires Docker"]
    fn test_proxy_pass_without_payment() {
        // Test Case: proxy_pass without payment header
        //
        // This test verifies that:
        // 1. x402 handler works correctly with proxy_pass configured
        // 2. When no payment header is provided, should return 402 (not proxy to backend)
        // 3. Backend should NOT be called when payment verification fails
        //
        // Expected behavior:
        // - Should return 402 (payment required)
        // - Should NOT return 502 (Bad Gateway, which would indicate backend was called)

        if !ensure_container_running() {
            eprintln!("Failed to start container. Skipping test.");
            return;
        }

        // Retry logic: sometimes nginx needs a moment to be fully ready
        let mut status = String::new();
        let mut retries = 5;
        while retries > 0 {
            status = http_request("/api/protected-proxy").unwrap_or_else(|| "000".to_string());
            if status != "000" {
                break;
            }
            retries -= 1;
            thread::sleep(Duration::from_millis(500));
        }

        // 000 means curl failed to connect, which shouldn't happen if container is running
        if status == "000" {
            eprintln!("Got 000 status code after retries - curl failed to connect");
            eprintln!("Checking if nginx is still responding...");
            let health_status = http_request("/health").unwrap_or_else(|| "000".to_string());
            eprintln!("Health endpoint status: {health_status}");
            panic!("Failed to connect to /api/protected-proxy (got 000), but health check returned: {health_status}");
        }

        assert_eq!(
            status, "402",
            "Expected 402 response when no payment header provided with proxy_pass, got {status}"
        );
    }

    #[test]
    #[ignore = "requires Docker"]
    fn test_proxy_pass_with_invalid_payment() {
        // Test Case: proxy_pass with invalid payment header
        //
        // This test verifies that:
        // 1. Invalid payment headers are rejected before proxying
        // 2. Backend should NOT be called when payment verification fails
        // 3. Should return 402 (payment verification failed) or 500 (facilitator error)
        //
        // Expected behavior:
        // - Should return 402 (payment verification failed) or 500 (facilitator error)
        // - Should NOT return 502 (Bad Gateway, which would indicate backend was called)

        if !ensure_container_running() {
            eprintln!("Failed to start container. Skipping test.");
            return;
        }

        // Retry logic: sometimes nginx needs a moment to be fully ready
        let mut status = String::new();
        let mut retries = 5;
        while retries > 0 {
            let output = Command::new("curl")
                .args([
                    "-s",
                    "-o",
                    "/dev/null",
                    "-w",
                    "%{http_code}",
                    "-H",
                    "X-PAYMENT: invalid-payment-header",
                    &format!("http://localhost:{NGINX_PORT}/api/protected-proxy"),
                ])
                .output();

            match output {
                Ok(output) => {
                    status = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    if status != "000" {
                        break;
                    }
                }
                Err(_) => {
                    status = "000".to_string();
                }
            }
            retries -= 1;
            thread::sleep(Duration::from_millis(500));
        }

        // 000 means curl failed to connect, which shouldn't happen if container is running
        if status == "000" {
            eprintln!("Got 000 status code after retries - curl failed to connect");
            eprintln!("Checking if nginx is still responding...");
            let health_status = http_request("/health").unwrap_or_else(|| "000".to_string());
            eprintln!("Health endpoint status: {health_status}");
            panic!("Failed to connect to /api/protected-proxy (got 000), but health check returned: {health_status}");
        }

        // Should return 402 (payment verification failed) or 500 (facilitator error)
        // but NOT proxy to backend (which would return 502 Bad Gateway)
        assert!(
            status == "402" || status == "500",
            "Expected 402 or 500 response when invalid payment provided with proxy_pass, got {status}"
        );
        assert_ne!(
            status, "502",
            "Should not proxy to backend when payment is invalid (got 502 Bad Gateway)"
        );
    }

    #[test]
    #[ignore = "requires Docker"]
    fn test_proxy_pass_verification_order() {
        // Test Case: Payment verification happens before proxy_pass
        //
        // This is a critical test that verifies the execution order:
        // 1. x402 handler runs in ACCESS_PHASE
        // 2. proxy_pass handler runs in CONTENT_PHASE (after ACCESS_PHASE)
        // 3. If payment verification fails in ACCESS_PHASE, request returns 402 without
        //    reaching CONTENT_PHASE where proxy_pass would run
        //
        // Expected behavior:
        // - Request without payment should return 402
        // - Backend should NOT be called (no backend response in body)
        // - This proves payment verification happens before proxy_pass

        if !ensure_container_running() {
            eprintln!("Failed to start container. Skipping test.");
            return;
        }

        // Request without payment should return 402, not reach backend
        let status = http_request("/api/protected-proxy").expect("Failed to make HTTP request");

        assert_eq!(
            status, "402",
            "Payment verification should happen before proxy_pass. Expected 402, got {status}"
        );

        // Verify that backend was NOT called by checking that we didn't get backend response
        // If backend was called, we would get 502 Bad Gateway or backend's JSON response
        let body = http_get("/api/protected-proxy");
        if let Some(response_body) = body {
            // Should not contain backend response JSON
            assert!(
                !response_body.contains("\"status\":\"ok\"") && !response_body.contains("Backend response"),
                "Backend should not be called when payment verification fails. Got backend response: {response_body}"
            );
        }
    }
}

