//! HEAD request CORS integration tests for nginx-x402 module
//!
//! Additional tests for HEAD request CORS handling and edge cases.

#[cfg(feature = "integration-test")]
mod tests {
    use super::super::common::*;
    use std::process::Command;

    #[test]
    #[ignore = "requires Docker"]
    fn test_head_request_cors_headers_strict_validation() {
        // Test Case: Strict validation of CORS headers in HEAD response
        // Verify CORS headers are present and correct
        if !ensure_container_running() {
            eprintln!("Failed to start container. Skipping test.");
            return;
        }

        let result = http_request_with_method_and_headers(
            "/api/protected-proxy",
            "HEAD",
            &[("Origin", "http://127.0.0.1:8080")],
        );

        match result {
            Some((status, headers)) => {
                println!("HEAD request status: {status}");
                println!("Response headers:\n{}", headers);

                // Must not require payment
                assert_ne!(
                    status, "402",
                    "HEAD request must not require payment (got 402)"
                );

                // Must return 200 for HEAD
                assert_eq!(status, "200", "HEAD request must return 200, got {status}");

                // Strict validation: Check exact header names (case-insensitive)
                let headers_lower = headers.to_lowercase();

                // Access-Control-Allow-Origin must be present
                assert!(
                    headers_lower.contains("access-control-allow-origin"),
                    "Response MUST contain Access-Control-Allow-Origin header. Headers: {}",
                    headers.chars().take(500).collect::<String>()
                );

                // Content-Type should be present (from backend)
                assert!(
                    headers_lower.contains("content-type"),
                    "HEAD response should have Content-Type header. Headers: {}",
                    headers.chars().take(500).collect::<String>()
                );

                // Content-Length should be present (HEAD requests should have this)
                assert!(
                    headers_lower.contains("content-length"),
                    "HEAD response should have Content-Length header. Headers: {}",
                    headers.chars().take(500).collect::<String>()
                );

                println!("✓ HEAD request CORS headers validation passed");
            }
            None => {
                panic!("Failed to make HEAD request to /api/protected-proxy");
            }
        }
    }

    #[test]
    #[ignore = "requires Docker"]
    fn test_head_request_response_body_must_be_empty() {
        // Test Case: HEAD request must have empty response body
        // Verify that HEAD request returns no body content
        if !ensure_container_running() {
            eprintln!("Failed to start container. Skipping test.");
            return;
        }

        // Use curl to get full response (including body)
        let url = format!("http://localhost:{NGINX_PORT}/api/protected-proxy");
        let output = Command::new("curl")
            .args([
                "-s",
                "-i",
                "-X",
                "HEAD",
                "-H",
                "Origin: http://127.0.0.1:8080",
                &url,
            ])
            .output();

        match output {
            Ok(output) => {
                let response = String::from_utf8_lossy(&output.stdout).to_string();
                println!("HEAD request full response:\n{}", response);

                // Split headers and body
                let parts: Vec<&str> = response.split("\r\n\r\n").collect();
                let headers = parts.get(0).unwrap_or(&"");
                let body = parts.get(1).unwrap_or(&"");

                // Verify status is 200
                assert!(
                    headers.contains("200"),
                    "HEAD request must return 200 status"
                );

                // Verify body is empty or only contains whitespace
                let body_trimmed = body.trim();
                assert!(
                    body_trimmed.is_empty(),
                    "HEAD request body must be empty, but got: '{}'",
                    body_trimmed.chars().take(100).collect::<String>()
                );

                println!("✓ HEAD request response body validation passed");
            }
            Err(e) => {
                panic!("Failed to make HEAD request: {}", e);
            }
        }
    }

    #[test]
    #[ignore = "requires Docker"]
    fn test_head_request_with_payment_header_still_skips_payment() {
        // Test Case: HEAD request with payment header should still skip payment verification
        // HEAD requests should never require payment, even if X-PAYMENT header is present
        if !ensure_container_running() {
            eprintln!("Failed to start container. Skipping test.");
            return;
        }

        let result = http_request_with_method_and_headers(
            "/api/protected",
            "HEAD",
            &[("X-PAYMENT", "dummy-payment-header")],
        );

        match result {
            Some((status, headers)) => {
                println!("HEAD request status (with payment header): {status}");

                // HEAD request must skip payment verification even with X-PAYMENT header
                assert_ne!(
                    status, "402",
                    "HEAD request must skip payment verification even with X-PAYMENT header (got 402)"
                );

                // Should return 200 (not 402)
                assert_eq!(
                    status, "200",
                    "HEAD request with payment header should return 200 (payment skipped), got {status}"
                );

                println!("✓ HEAD request correctly skips payment even with X-PAYMENT header");
            }
            None => {
                panic!("Failed to make HEAD request with payment header");
            }
        }
    }
}
