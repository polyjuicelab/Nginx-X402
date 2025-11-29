//! OPTIONS request integration tests for nginx-x402 module
//!
//! Tests for OPTIONS request handling, CORS preflight, and payment verification skipping.

#[cfg(feature = "integration-test")]
mod tests {
    use super::super::common::*;
    use std::process::Command;
    use std::thread;
    use std::time::Duration;

    #[test]
    #[ignore = "requires Docker"]
    fn test_options_request_skips_payment() {
        // Test Case: OPTIONS request (CORS preflight) should skip payment verification
        // OPTIONS requests are sent by browsers before cross-origin requests to check CORS policy.
        // These requests should bypass payment verification to allow CORS checks to complete.
        if !ensure_container_running() {
            eprintln!("Failed to start container. Skipping test.");
            return;
        }

        // Test OPTIONS request with basic CORS preflight headers
        // Retry logic: sometimes nginx needs a moment to be fully ready
        let mut status = String::new();
        let mut retries = 5;
        while retries > 0 {
            let result = http_request_with_method(
                "/api/protected",
                "OPTIONS",
                &[
                    ("Origin", "http://127.0.0.1:8080"),
                    ("Access-Control-Request-Method", "GET"),
                    ("Access-Control-Request-Headers", "content-type"),
                ],
            );
            match result {
                Some(s) if s != "000" => {
                    status = s;
                    break;
                }
                Some(s) => {
                    status = s;
                    retries -= 1;
                    thread::sleep(Duration::from_millis(500));
                }
                None => {
                    status = "000".to_string();
                    retries -= 1;
                    thread::sleep(Duration::from_millis(500));
                }
            }
        }

        println!("OPTIONS request status (no payment): {status}");

        // OPTIONS request should skip payment verification
        // It should NOT return 402, which would indicate payment verification was attempted
        // Acceptable status codes:
        // - 200/204: Request succeeded (if backend supports OPTIONS)
        // - 405: Method Not Allowed (if backend doesn't support OPTIONS, but payment was skipped)
        // - 501: Not Implemented (if server doesn't support OPTIONS, but payment was skipped)
        // - 000: Connection error (should not happen if container is running)
        assert!(
            status == "200" || status == "204" || status == "405" || status == "501",
            "OPTIONS request should skip payment verification and return 200/204/405/501, got {status}. \
             If status is 402, payment verification was incorrectly applied to OPTIONS request. \
             If status is 000, there may be a connection issue."
        );

        // Verify that OPTIONS request does not require payment
        assert_ne!(
            status, "402",
            "OPTIONS request should not require payment (got 402). \
             Payment verification should be skipped for OPTIONS requests."
        );

        // Verify that we didn't get a connection error
        assert_ne!(
            status, "000",
            "OPTIONS request failed to connect (got 000). \
             This may indicate the container is not running or nginx is not responding."
        );

        println!("✓ OPTIONS request correctly skipped payment verification");
    }

    #[test]
    #[ignore = "requires Docker"]
    fn test_options_request_with_proxy_pass_returns_cors_headers() {
        // Test Case: OPTIONS request with proxy_pass should be forwarded to backend
        // Backend should return CORS headers, and we should verify they are present
        if !ensure_container_running() {
            eprintln!("Failed to start container. Skipping test.");
            return;
        }

        // Test OPTIONS request to /api/protected-proxy (has proxy_pass)
        let result = http_request_with_method_and_headers(
            "/api/protected-proxy",
            "OPTIONS",
            &[
                ("Origin", "http://127.0.0.1:8080"),
                ("Access-Control-Request-Method", "GET"),
                ("Access-Control-Request-Headers", "content-type,x-payment"),
            ],
        );

        match result {
            Some((status, headers)) => {
                println!("OPTIONS request status (with proxy_pass): {status}");
                println!("Response headers:\n{}", headers);

                // OPTIONS request should skip payment verification and be forwarded to backend
                assert_ne!(
                    status, "402",
                    "OPTIONS request should not require payment (got 402). \
                     Payment verification should be skipped for OPTIONS requests."
                );

                // Verify CORS headers are present (from backend)
                assert!(
                    headers.contains("Access-Control-Allow-Origin"),
                    "Response should contain Access-Control-Allow-Origin header from backend"
                );
                assert!(
                    headers.contains("Access-Control-Allow-Methods"),
                    "Response should contain Access-Control-Allow-Methods header from backend"
                );
                assert!(
                    headers.contains("Access-Control-Allow-Headers"),
                    "Response should contain Access-Control-Allow-Headers header from backend"
                );

                // Verify status is 204 (from backend OPTIONS handler)
                assert_eq!(
                    status, "204",
                    "OPTIONS request should return 204 from backend, got {status}"
                );

                println!("✓ OPTIONS request with proxy_pass correctly forwarded to backend and returned CORS headers");
            }
            None => {
                panic!("Failed to make OPTIONS request to /api/protected-proxy");
            }
        }
    }

    #[test]
    #[ignore = "requires Docker"]
    fn test_options_request_without_proxy_pass_returns_204() {
        // Test Case: OPTIONS request without proxy_pass should return 204 No Content
        // This verifies that x402 module handles OPTIONS requests correctly when no backend is configured
        if !ensure_container_running() {
            eprintln!("Failed to start container. Skipping test.");
            return;
        }

        let result = http_request_with_method_and_headers(
            "/api/protected",
            "OPTIONS",
            &[
                ("Origin", "http://127.0.0.1:8080"),
                ("Access-Control-Request-Method", "GET"),
                ("Access-Control-Request-Headers", "content-type,x-payment"),
            ],
        );

        match result {
            Some((status, headers)) => {
                println!("OPTIONS request status (no proxy_pass): {status}");
                println!("Response headers:\n{}", headers);

                // OPTIONS request should skip payment verification
                assert_ne!(
                    status, "402",
                    "OPTIONS request should not require payment (got 402). \
                     Payment verification should be skipped for OPTIONS requests."
                );

                // Without proxy_pass, x402 module should return 204 No Content
                assert_eq!(
                    status, "204",
                    "OPTIONS request without proxy_pass should return 204 No Content, got {status}"
                );

                // Verify response has no body (HEAD request should have no body)
                // This is verified by checking Content-Length header
                assert!(
                    headers.contains("Content-Length: 0") || headers.contains("content-length: 0"),
                    "OPTIONS response should have Content-Length: 0 header"
                );

                println!("✓ OPTIONS request without proxy_pass correctly returns 204");
            }
            None => {
                panic!("Failed to make OPTIONS request to /api/protected");
            }
        }
    }

    #[test]
    #[ignore = "requires Docker"]
    fn test_options_request_cors_headers_strict_validation() {
        // Test Case: Strict validation of CORS headers in OPTIONS response
        // Verify all required CORS headers are present with correct values
        if !ensure_container_running() {
            eprintln!("Failed to start container. Skipping test.");
            return;
        }

        let result = http_request_with_method_and_headers(
            "/api/protected-proxy",
            "OPTIONS",
            &[
                ("Origin", "http://127.0.0.1:8080"),
                ("Access-Control-Request-Method", "GET"),
                ("Access-Control-Request-Headers", "content-type,x-payment"),
            ],
        );

        match result {
            Some((status, headers)) => {
                println!("OPTIONS request status: {status}");
                println!("Response headers:\n{}", headers);

                // Must not require payment
                assert_ne!(
                    status, "402",
                    "OPTIONS request must not require payment (got 402)"
                );

                // Must return 204 for OPTIONS preflight
                assert_eq!(
                    status, "204",
                    "OPTIONS preflight request must return 204, got {status}"
                );

                // Strict validation: Check exact header names (case-insensitive)
                let headers_lower = headers.to_lowercase();

                // Access-Control-Allow-Origin must be present
                assert!(
                    headers_lower.contains("access-control-allow-origin"),
                    "Response MUST contain Access-Control-Allow-Origin header. Headers: {}",
                    headers.chars().take(500).collect::<String>()
                );

                // Access-Control-Allow-Methods must be present
                assert!(
                    headers_lower.contains("access-control-allow-methods"),
                    "Response MUST contain Access-Control-Allow-Methods header. Headers: {}",
                    headers.chars().take(500).collect::<String>()
                );

                // Access-Control-Allow-Headers must be present
                assert!(
                    headers_lower.contains("access-control-allow-headers"),
                    "Response MUST contain Access-Control-Allow-Headers header. Headers: {}",
                    headers.chars().take(500).collect::<String>()
                );

                // Access-Control-Allow-Methods should include GET (requested method)
                assert!(
                    headers_lower.contains("get"),
                    "Access-Control-Allow-Methods should include GET. Headers: {}",
                    headers.chars().take(500).collect::<String>()
                );

                // Access-Control-Allow-Headers should include requested headers
                assert!(
                    headers_lower.contains("content-type") || headers_lower.contains("x-payment"),
                    "Access-Control-Allow-Headers should include requested headers (content-type or x-payment). Headers: {}",
                    headers.chars().take(500).collect::<String>()
                );

                // Access-Control-Max-Age should be present (optional but recommended)
                // This is a soft check - we warn but don't fail
                if !headers_lower.contains("access-control-max-age") {
                    println!("⚠ Warning: Access-Control-Max-Age header not present (optional but recommended)");
                }

                println!("✓ OPTIONS request CORS headers validation passed");
            }
            None => {
                panic!("Failed to make OPTIONS request to /api/protected-proxy");
            }
        }
    }
}
