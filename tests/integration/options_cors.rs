//! OPTIONS request CORS integration tests for nginx-x402 module
//!
//! Additional tests for OPTIONS request CORS handling and edge cases.

#[cfg(feature = "integration-test")]
mod tests {
    use super::super::common::*;
    use std::process::Command;

    #[test]
    #[ignore = "requires Docker"]
    fn test_options_request_with_different_origins() {
        // Test Case: OPTIONS request with different Origin headers
        // Verify CORS headers are returned correctly for different origins
        if !ensure_container_running() {
            eprintln!("Failed to start container. Skipping test.");
            return;
        }

        let origins = vec![
            "http://127.0.0.1:8080",
            "http://localhost:3000",
            "https://example.com",
            "http://example.com:8080",
        ];

        for origin in origins {
            let result = http_request_with_method_and_headers(
                "/api/protected-proxy",
                "OPTIONS",
                &[
                    ("Origin", origin),
                    ("Access-Control-Request-Method", "GET"),
                    ("Access-Control-Request-Headers", "content-type"),
                ],
            );

            match result {
                Some((status, headers)) => {
                    println!(
                        "OPTIONS request with Origin: {} - Status: {}",
                        origin, status
                    );

                    // Must not require payment
                    assert_ne!(
                        status, "402",
                        "OPTIONS request with Origin {} must not require payment (got 402)",
                        origin
                    );

                    // Must return 204
                    assert_eq!(
                        status, "204",
                        "OPTIONS request with Origin {} must return 204, got {}",
                        origin, status
                    );

                    // Must have CORS headers
                    let headers_lower = headers.to_lowercase();
                    assert!(
                        headers_lower.contains("access-control-allow-origin"),
                        "OPTIONS response with Origin {} must have Access-Control-Allow-Origin header",
                        origin
                    );
                }
                None => {
                    panic!("Failed to make OPTIONS request with Origin: {}", origin);
                }
            }
        }

        println!("✓ OPTIONS request with different origins validation passed");
    }

    #[test]
    #[ignore = "requires Docker"]
    fn test_options_request_response_body_must_be_empty() {
        // Test Case: OPTIONS request must have empty response body
        // Verify that OPTIONS preflight request returns no body content
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
                "OPTIONS",
                "-H",
                "Origin: http://127.0.0.1:8080",
                "-H",
                "Access-Control-Request-Method: GET",
                "-H",
                "Access-Control-Request-Headers: content-type",
                &url,
            ])
            .output();

        match output {
            Ok(output) => {
                let response = String::from_utf8_lossy(&output.stdout).to_string();
                println!("OPTIONS request full response:\n{}", response);

                // Split headers and body
                let parts: Vec<&str> = response.split("\r\n\r\n").collect();
                let headers = parts.get(0).unwrap_or(&"");
                let body = parts.get(1).unwrap_or(&"");

                // Verify status is 204
                assert!(
                    headers.contains("204"),
                    "OPTIONS request must return 204 status"
                );

                // Verify body is empty or only contains whitespace
                let body_trimmed = body.trim();
                assert!(
                    body_trimmed.is_empty(),
                    "OPTIONS request body must be empty, but got: '{}'",
                    body_trimmed.chars().take(100).collect::<String>()
                );

                println!("✓ OPTIONS request response body validation passed");
            }
            Err(e) => {
                panic!("Failed to make OPTIONS request: {}", e);
            }
        }
    }

    #[test]
    #[ignore = "requires Docker"]
    fn test_options_request_missing_cors_headers_fails() {
        // Test Case: OPTIONS request without Origin header should still work
        // But CORS headers may not be present (this is acceptable)
        if !ensure_container_running() {
            eprintln!("Failed to start container. Skipping test.");
            return;
        }

        let result = http_request_with_method_and_headers(
            "/api/protected-proxy",
            "OPTIONS",
            &[
                // No Origin header
                ("Access-Control-Request-Method", "GET"),
            ],
        );

        match result {
            Some((status, headers)) => {
                println!("OPTIONS request status (no Origin): {status}");
                println!("Response headers:\n{}", headers);

                // Must not require payment
                assert_ne!(
                    status, "402",
                    "OPTIONS request must not require payment even without Origin header (got 402)"
                );

                // Should still return 204
                assert_eq!(
                    status, "204",
                    "OPTIONS request should return 204 even without Origin header, got {status}"
                );

                // CORS headers may or may not be present without Origin
                // This is acceptable behavior
                println!("✓ OPTIONS request without Origin header handled correctly");
            }
            None => {
                panic!("Failed to make OPTIONS request without Origin header");
            }
        }
    }

    #[test]
    #[ignore = "requires Docker"]
    fn test_options_request_with_payment_header_still_skips_payment() {
        // Test Case: OPTIONS request with payment header should still skip payment verification
        // OPTIONS requests should never require payment, even if X-PAYMENT header is present
        if !ensure_container_running() {
            eprintln!("Failed to start container. Skipping test.");
            return;
        }

        let result = http_request_with_method_and_headers(
            "/api/protected",
            "OPTIONS",
            &[
                ("Origin", "http://127.0.0.1:8080"),
                ("X-PAYMENT", "dummy-payment-header"),
            ],
        );

        match result {
            Some((status, headers)) => {
                println!("OPTIONS request status (with payment header): {status}");

                // OPTIONS request must skip payment verification even with X-PAYMENT header
                assert_ne!(
                    status, "402",
                    "OPTIONS request must skip payment verification even with X-PAYMENT header (got 402)"
                );

                // Should return 204 (not 402)
                assert_eq!(
                    status, "204",
                    "OPTIONS request with payment header should return 204 (payment skipped), got {status}"
                );

                println!("✓ OPTIONS request correctly skips payment even with X-PAYMENT header");
            }
            None => {
                panic!("Failed to make OPTIONS request with payment header");
            }
        }
    }
}
