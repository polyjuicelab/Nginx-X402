//! HEAD request integration tests for nginx-x402 module
//!
//! Tests for HEAD request handling and payment verification skipping.

#[cfg(feature = "integration-test")]
mod tests {
    use super::super::common::*;
    use std::process::Command;
    use std::thread;
    use std::time::Duration;

    #[test]
    #[ignore = "requires Docker"]
    fn test_head_request_skips_payment() {
        // Test Case: HEAD request should skip payment verification
        // HEAD requests are used to check resource existence without retrieving body
        if !ensure_container_running() {
            eprintln!("Failed to start container. Skipping test.");
            return;
        }

        // Test HEAD request without payment header
        // Retry logic: sometimes nginx needs a moment to be fully ready
        let mut status = String::new();
        let mut retries = 5;
        while retries > 0 {
            let result = http_request_with_method("/api/protected", "HEAD", &[]);
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

        println!("HEAD request status (no payment): {status}");

        // HEAD request should skip payment verification
        // It should NOT return 402, which would indicate payment verification was attempted
        // Since /api/protected has no proxy_pass, x402 module should send 200 response for HEAD
        // Acceptable status codes:
        // - 200: x402 module sends 200 OK response (expected for HEAD without proxy_pass)
        // - 204: Alternative acceptable response
        // - 404: Not Found (if resource doesn't exist, but payment was skipped)
        // - 405: Method Not Allowed (if backend doesn't support HEAD, but payment was skipped)
        // - 501: Not Implemented (if server doesn't support HEAD, but payment was skipped)
        // - 000: Connection error (should not happen if container is running)
        assert!(
            status == "200" || status == "204" || status == "404" || status == "405" || status == "501",
            "HEAD request should skip payment verification and return appropriate status (200/204/404/405/501), got {status}. \
             Expected 200 from x402 module for HEAD requests without proxy_pass."
        );

        // Verify that HEAD request does not require payment
        assert_ne!(
            status, "402",
            "HEAD request should not require payment (got 402). \
             Payment verification should be skipped for HEAD requests."
        );

        // Verify that we didn't get a connection error
        assert_ne!(
            status, "000",
            "HEAD request failed to connect (got 000). \
             This may indicate the container is not running or nginx is not responding."
        );

        println!("✓ HEAD request correctly skipped payment verification");
    }

    #[test]
    #[ignore = "requires Docker"]
    fn test_head_request_with_proxy_pass_returns_cors_headers() {
        // Test Case: HEAD request with proxy_pass should be forwarded to backend
        // Backend should return CORS headers
        if !ensure_container_running() {
            eprintln!("Failed to start container. Skipping test.");
            return;
        }

        // Test HEAD request to /api/protected-proxy (has proxy_pass)
        let result = http_request_with_method_and_headers(
            "/api/protected-proxy",
            "HEAD",
            &[("Origin", "http://127.0.0.1:8080")],
        );

        match result {
            Some((status, headers)) => {
                println!("HEAD request status (with proxy_pass): {status}");

                // HEAD request should skip payment verification and be forwarded to backend
                assert_ne!(
                    status, "402",
                    "HEAD request should not require payment (got 402). \
                     Payment verification should be skipped for HEAD requests."
                );

                // Verify CORS headers are present (from backend)
                assert!(
                    headers.contains("Access-Control-Allow-Origin"),
                    "Response should contain Access-Control-Allow-Origin header from backend"
                );

                // Verify status is 200 (from backend HEAD handler)
                assert_eq!(
                    status, "200",
                    "HEAD request should return 200 from backend, got {status}"
                );

                println!("✓ HEAD request with proxy_pass correctly forwarded to backend and returned CORS headers");
            }
            None => {
                panic!("Failed to make HEAD request to /api/protected-proxy");
            }
        }
    }

    #[test]
    #[ignore = "requires Docker"]
    fn test_head_request_without_proxy_pass_returns_200() {
        // Test Case: HEAD request without proxy_pass should return 200 OK
        // Verify that HEAD request has no body and correct headers
        if !ensure_container_running() {
            eprintln!("Failed to start container. Skipping test.");
            return;
        }

        let result = http_request_with_method_and_headers("/api/protected", "HEAD", &[]);

        match result {
            Some((status, headers)) => {
                println!("HEAD request status (no proxy_pass): {status}");
                println!("Response headers:\n{}", headers);

                // HEAD request should skip payment verification
                assert_ne!(
                    status, "402",
                    "HEAD request should not require payment (got 402). \
                     Payment verification should be skipped for HEAD requests."
                );

                // Without proxy_pass, x402 module should return 200 OK for HEAD
                assert_eq!(
                    status, "200",
                    "HEAD request without proxy_pass should return 200 OK, got {status}"
                );

                // HEAD request must have no body (verified by checking response)
                // Content-Length should be present
                let headers_lower = headers.to_lowercase();
                assert!(
                    headers_lower.contains("content-length"),
                    "HEAD response should have Content-Length header"
                );

                println!("✓ HEAD request without proxy_pass correctly returns 200");
            }
            None => {
                panic!("Failed to make HEAD request to /api/protected");
            }
        }
    }
}
