//! Header passthrough tests
//!
//! This module tests that headers from backend services are correctly passed through
//! by nginx proxy_pass, including:
//!
//! - CORS headers (Access-Control-*)
//! - Custom headers (X-* headers)
//!
//! # Background
//!
//! When nginx proxies requests to backend services using proxy_pass, the backend's
//! response headers should be passed through to the client. This is important for:
//!
//! - CORS: Backend services need to set CORS headers for cross-origin requests
//! - Custom headers: Backend services may set custom headers for API versioning,
//!   request IDs, or other metadata
//!
//! By default, nginx passes through all headers from the backend. However, we need
//! to verify that:
//!
//! 1. CORS headers are not stripped or modified
//! 2. Custom headers are preserved
//! 3. Headers work correctly even when x402 payment verification is enabled

#[cfg(feature = "integration-test")]
mod tests {
    use crate::docker_integration::common::*;
    use std::process::Command;
    use std::thread;
    use std::time::Duration;

    /// Extract header value from curl output (-i flag includes headers)
    fn get_header_value(response: &str, header_name: &str) -> Option<String> {
        for line in response.lines() {
            // Headers are case-insensitive, so we compare case-insensitively
            let line_lower = line.to_lowercase();
            let header_lower = header_name.to_lowercase();
            if line_lower.starts_with(&header_lower) {
                // Find the colon separator
                if let Some(colon_pos) = line.find(':') {
                    let value = line[colon_pos + 1..].trim();
                    return Some(value.to_string());
                }
            }
        }
        None
    }

    #[test]
    #[ignore = "requires Docker"]
    fn test_cors_headers_passthrough_with_options() {
        // Test Case: CORS headers from backend are passed through via OPTIONS request
        //
        // This test verifies that:
        // 1. OPTIONS requests skip payment verification (as per x402 module design)
        // 2. OPTIONS requests can access backend service
        // 3. Backend sets CORS headers (Access-Control-Allow-Origin, etc.)
        // 4. These headers are passed through by nginx proxy_pass (default behavior)
        // 5. Headers are not modified or stripped
        //
        // Expected behavior:
        // - OPTIONS request should return 200 (not 402, because payment is skipped)
        // - Access-Control-Allow-Origin: * should be present
        // - Access-Control-Allow-Methods should be present
        // - Access-Control-Allow-Headers should be present
        // - Access-Control-Expose-Headers should be present

        if !ensure_container_running() {
            eprintln!("Failed to start container. Skipping test.");
            return;
        }

        // Use OPTIONS method which skips payment verification
        // Make OPTIONS request with proper headers to get full response
        let url = format!("http://localhost:{NGINX_PORT}/api/protected-proxy");
        let mut args = vec!["-s", "-i", "-X", "OPTIONS"];
        args.push("-H");
        args.push("Origin: https://example.com");
        args.push("-H");
        args.push("Access-Control-Request-Method: GET");
        args.push(&url);

        let output = Command::new("curl")
            .args(args)
            .output()
            .ok()
            .map(|output| String::from_utf8_lossy(&output.stdout).to_string());

        if let Some(response_text) = output {
            // Extract status code
            let status_line = response_text.lines().next().unwrap_or("");
            let status = if status_line.contains("200") {
                "200"
            } else if status_line.contains("204") {
                "204"
            } else if status_line.contains("402") {
                "402"
            } else {
                status_line
            };

            // OPTIONS should skip payment verification, so we should get 200, not 402
            assert!(
                status == "200" || status == "204",
                "OPTIONS request should return 200/204 (payment skipped), got {status}. \
                 If status is 402, payment verification was incorrectly applied to OPTIONS request. Response: {}",
                response_text
            );
            assert_ne!(
                status, "402",
                "OPTIONS request should skip payment verification (got 402)"
            );

            // Check for CORS headers
            let allow_origin = get_header_value(&response_text, "Access-Control-Allow-Origin");
            let allow_methods = get_header_value(&response_text, "Access-Control-Allow-Methods");
            let allow_headers = get_header_value(&response_text, "Access-Control-Allow-Headers");
            let expose_headers = get_header_value(&response_text, "Access-Control-Expose-Headers");

            assert!(
                allow_origin.is_some(),
                "Access-Control-Allow-Origin header should be present in backend response. Response: {}",
                response_text
            );
            assert_eq!(
                allow_origin.unwrap(),
                "*",
                "Access-Control-Allow-Origin should be '*'"
            );

            assert!(
                allow_methods.is_some(),
                "Access-Control-Allow-Methods header should be present. Response: {}",
                response_text
            );

            assert!(
                allow_headers.is_some(),
                "Access-Control-Allow-Headers header should be present. Response: {}",
                response_text
            );

            assert!(
                expose_headers.is_some(),
                "Access-Control-Expose-Headers header should be present. Response: {}",
                response_text
            );
        } else {
            panic!("Failed to get OPTIONS response with headers");
        }
    }

    #[test]
    #[ignore = "requires Docker"]
    fn test_cors_preflight_request_with_options() {
        // Test Case: CORS preflight (OPTIONS) request handling
        //
        // This test verifies that:
        // 1. OPTIONS requests (CORS preflight) skip payment verification
        // 2. OPTIONS requests can access backend service
        // 3. CORS headers are returned in OPTIONS response from backend
        // 4. Headers are passed through by nginx (default behavior)
        //
        // Expected behavior:
        // - OPTIONS request should return 200 (not 402, because payment is skipped)
        // - CORS headers should be present in response

        if !ensure_container_running() {
            eprintln!("Failed to start container. Skipping test.");
            return;
        }

        // Make OPTIONS request (CORS preflight) with proper headers
        let url = format!("http://localhost:{NGINX_PORT}/api/protected-proxy");
        let mut args = vec!["-s", "-i", "-X", "OPTIONS"];
        args.push("-H");
        args.push("Origin: https://example.com");
        args.push("-H");
        args.push("Access-Control-Request-Method: GET");
        args.push("-H");
        args.push("Access-Control-Request-Headers: X-PAYMENT");
        args.push(&url);

        let output = Command::new("curl")
            .args(args)
            .output()
            .ok()
            .map(|output| String::from_utf8_lossy(&output.stdout).to_string());

        if let Some(response_text) = output {
            // Extract status code
            let status_line = response_text.lines().next().unwrap_or("");
            let status = if status_line.contains("200") {
                "200"
            } else if status_line.contains("204") {
                "204"
            } else if status_line.contains("402") {
                "402"
            } else {
                status_line
            };

            // OPTIONS requests should skip payment verification
            // So we should get 200 or 204, not 402
            assert!(
                status == "200" || status == "204",
                "OPTIONS request should return 200/204 (payment skipped), got {status}. Response: {}",
                response_text
            );
            assert_ne!(
                status, "402",
                "OPTIONS request should skip payment verification (got 402)"
            );

            // Check for CORS headers
            let allow_origin = get_header_value(&response_text, "Access-Control-Allow-Origin");
            let allow_methods = get_header_value(&response_text, "Access-Control-Allow-Methods");
            let allow_headers = get_header_value(&response_text, "Access-Control-Allow-Headers");

            assert!(
                allow_origin.is_some(),
                "Access-Control-Allow-Origin should be present in OPTIONS response. Response: {}",
                response_text
            );
            assert_eq!(
                allow_origin.unwrap(),
                "*",
                "Access-Control-Allow-Origin should be '*'"
            );

            assert!(
                allow_methods.is_some(),
                "Access-Control-Allow-Methods should be present in OPTIONS response. Response: {}",
                response_text
            );

            assert!(
                allow_headers.is_some(),
                "Access-Control-Allow-Headers should be present in OPTIONS response. Response: {}",
                response_text
            );
        } else {
            panic!("Failed to make OPTIONS request");
        }
    }

    #[test]
    #[ignore = "requires Docker"]
    fn test_head_request_cors_headers() {
        // Test Case: HEAD request CORS headers passthrough
        //
        // This test verifies that:
        // 1. HEAD requests skip payment verification
        // 2. HEAD requests can access backend service
        // 3. CORS headers are returned in HEAD response from backend
        // 4. Headers are passed through by nginx (default behavior)
        //
        // Expected behavior:
        // - HEAD request should return 200 (not 402, because payment is skipped)
        // - CORS headers should be present in response

        if !ensure_container_running() {
            eprintln!("Failed to start container. Skipping test.");
            return;
        }

        // Make HEAD request with proper headers
        // Retry logic: sometimes nginx needs a moment to be fully ready, especially under concurrent test execution
        let url = format!("http://localhost:{NGINX_PORT}/api/protected-proxy");
        let mut response_text_opt = None;
        let mut retries = 5;

        while retries > 0 {
            let mut args = vec!["-s", "-i", "-X", "HEAD"];
            args.push("-H");
            args.push("Origin: https://example.com");
            args.push(&url);

            let output = Command::new("curl")
                .args(args)
                .output()
                .ok()
                .map(|output| String::from_utf8_lossy(&output.stdout).to_string());

            if let Some(text) = output {
                let status_line = text.lines().next().unwrap_or("");
                // Check if we got a valid response (not empty)
                if !status_line.is_empty()
                    && (status_line.contains("HTTP")
                        || status_line.contains("200")
                        || status_line.contains("204")
                        || status_line.contains("402"))
                {
                    response_text_opt = Some(text);
                    break;
                }
            }
            retries -= 1;
            thread::sleep(Duration::from_millis(500));
        }

        if let Some(response_text) = response_text_opt {
            // Extract status code
            let status_line = response_text.lines().next().unwrap_or("");
            let status = if status_line.contains("200") {
                "200"
            } else if status_line.contains("204") {
                "204"
            } else if status_line.contains("402") {
                "402"
            } else {
                status_line
            };

            // HEAD requests should skip payment verification
            // So we should get 200 or 204, not 402
            assert!(
                status == "200" || status == "204",
                "HEAD request should return 200/204 (payment skipped), got {status}. Response: {}",
                response_text
            );
            assert_ne!(
                status, "402",
                "HEAD request should skip payment verification (got 402)"
            );

            // Check for CORS headers
            let allow_origin = get_header_value(&response_text, "Access-Control-Allow-Origin");
            let allow_methods = get_header_value(&response_text, "Access-Control-Allow-Methods");

            assert!(
                allow_origin.is_some(),
                "Access-Control-Allow-Origin should be present in HEAD response. Response: {}",
                response_text
            );
            assert_eq!(
                allow_origin.unwrap(),
                "*",
                "Access-Control-Allow-Origin should be '*'"
            );

            assert!(
                allow_methods.is_some(),
                "Access-Control-Allow-Methods should be present in HEAD response. Response: {}",
                response_text
            );
        } else {
            panic!("Failed to make HEAD request");
        }
    }

    #[test]
    #[ignore = "requires Docker"]
    fn test_custom_headers_passthrough_with_options() {
        // Test Case: Custom headers from backend are passed through via OPTIONS request
        //
        // This test verifies that:
        // 1. OPTIONS requests skip payment verification
        // 2. Backend sets custom headers (X-Custom-Response-Header, etc.)
        // 3. These headers are passed through by nginx proxy_pass (default behavior)
        // 4. Headers are not modified or stripped
        //
        // Expected behavior:
        // - OPTIONS request should return 200 (not 402)
        // - X-Custom-Response-Header should be present
        // - X-Another-Custom-Header should be present
        // - X-Backend-Version should be present

        if !ensure_container_running() {
            eprintln!("Failed to start container. Skipping test.");
            return;
        }

        // Use OPTIONS method which skips payment verification
        let url = format!("http://localhost:{NGINX_PORT}/api/protected-proxy");
        let mut args = vec!["-s", "-i", "-X", "OPTIONS"];
        args.push("-H");
        args.push("Origin: https://example.com");
        args.push(&url);

        let output = Command::new("curl")
            .args(args)
            .output()
            .ok()
            .map(|output| String::from_utf8_lossy(&output.stdout).to_string());

        if let Some(response_text) = output {
            // Extract status code
            let status_line = response_text.lines().next().unwrap_or("");
            let status = if status_line.contains("200") {
                "200"
            } else if status_line.contains("204") {
                "204"
            } else if status_line.contains("402") {
                "402"
            } else {
                status_line
            };

            // OPTIONS should skip payment verification
            assert!(
                status == "200" || status == "204",
                "OPTIONS request should return 200/204 (payment skipped), got {status}. Response: {}",
                response_text
            );
            assert_ne!(
                status, "402",
                "OPTIONS request should skip payment verification (got 402)"
            );

            // Check for custom headers
            let custom_header = get_header_value(&response_text, "X-Custom-Response-Header");
            let another_header = get_header_value(&response_text, "X-Another-Custom-Header");
            let backend_version = get_header_value(&response_text, "X-Backend-Version");

            assert!(
                custom_header.is_some(),
                "X-Custom-Response-Header should be present in backend response. Response: {}",
                response_text
            );
            assert_eq!(
                custom_header.unwrap(),
                "custom-value-123",
                "X-Custom-Response-Header should be 'custom-value-123'"
            );

            assert!(
                another_header.is_some(),
                "X-Another-Custom-Header should be present. Response: {}",
                response_text
            );
            assert_eq!(
                another_header.unwrap(),
                "another-value-456",
                "X-Another-Custom-Header should be 'another-value-456'"
            );

            assert!(
                backend_version.is_some(),
                "X-Backend-Version should be present. Response: {}",
                response_text
            );
            assert_eq!(
                backend_version.unwrap(),
                "1.0.0",
                "X-Backend-Version should be '1.0.0'"
            );
        } else {
            panic!("Failed to get OPTIONS response");
        }
    }

    #[test]
    #[ignore = "requires Docker"]
    fn test_cors_and_custom_headers_together_with_options() {
        // Test Case: Both CORS and custom headers are passed through together via OPTIONS
        //
        // This test verifies that:
        // 1. OPTIONS requests skip payment verification
        // 2. When backend sets both CORS and custom headers
        // 3. All headers are passed through correctly by nginx (default behavior)
        // 4. No headers are lost or modified
        //
        // Expected behavior:
        // - OPTIONS request should return 200 (not 402)
        // - Both CORS headers and custom headers should be present
        // - Headers should have correct values

        if !ensure_container_running() {
            eprintln!("Failed to start container. Skipping test.");
            return;
        }

        // Make OPTIONS request which skips payment verification
        let url = format!("http://localhost:{NGINX_PORT}/api/protected-proxy");
        let mut args = vec!["-s", "-i", "-X", "OPTIONS"];
        args.push("-H");
        args.push("Origin: https://example.com");
        args.push("-H");
        args.push("Access-Control-Request-Method: GET");
        args.push(&url);

        let output = Command::new("curl")
            .args(args)
            .output()
            .ok()
            .map(|output| String::from_utf8_lossy(&output.stdout).to_string());

        if let Some(response_text) = output {
            // Extract status code
            let status_line = response_text.lines().next().unwrap_or("");
            let status = if status_line.contains("200") {
                "200"
            } else if status_line.contains("204") {
                "204"
            } else if status_line.contains("402") {
                "402"
            } else {
                status_line
            };

            // OPTIONS should skip payment verification
            assert!(
                status == "200" || status == "204",
                "OPTIONS request should return 200/204 (payment skipped), got {status}. Response: {}",
                response_text
            );
            assert_ne!(
                status, "402",
                "OPTIONS request should skip payment verification (got 402)"
            );

            // Check CORS headers
            let allow_origin = get_header_value(&response_text, "Access-Control-Allow-Origin");
            let expose_headers = get_header_value(&response_text, "Access-Control-Expose-Headers");

            // Check custom headers
            let custom_header = get_header_value(&response_text, "X-Custom-Response-Header");
            let backend_version = get_header_value(&response_text, "X-Backend-Version");

            assert!(
                allow_origin.is_some(),
                "Access-Control-Allow-Origin should be present. Response: {}",
                response_text
            );
            assert_eq!(
                allow_origin.unwrap(),
                "*",
                "Access-Control-Allow-Origin should be '*'"
            );

            assert!(
                expose_headers.is_some(),
                "Access-Control-Expose-Headers should be present. Response: {}",
                response_text
            );
            // Expose-Headers should mention our custom headers
            let expose_headers_value = expose_headers.unwrap();
            assert!(
                expose_headers_value.contains("X-Custom-Response-Header"),
                "Access-Control-Expose-Headers should include X-Custom-Response-Header. Response: {}",
                response_text
            );

            assert!(
                custom_header.is_some(),
                "X-Custom-Response-Header should be present. Response: {}",
                response_text
            );
            assert_eq!(
                custom_header.unwrap(),
                "custom-value-123",
                "X-Custom-Response-Header should be 'custom-value-123'"
            );

            assert!(
                backend_version.is_some(),
                "X-Backend-Version should be present. Response: {}",
                response_text
            );
            assert_eq!(
                backend_version.unwrap(),
                "1.0.0",
                "X-Backend-Version should be '1.0.0'"
            );
        } else {
            panic!("Failed to make OPTIONS request");
        }
    }

    #[test]
    #[ignore = "requires Docker"]
    fn test_nginx_and_backend_custom_headers_together() {
        // Test Case: Both nginx-configured headers and backend headers are present
        //
        // This test verifies that:
        // 1. Nginx can add custom headers via add_header directive
        // 2. Backend can set custom headers
        // 3. Both types of headers can coexist in the response
        // 4. Headers are not overwritten or lost
        //
        // Expected behavior:
        // - X-NGINX-TEST header (from nginx add_header) should be present
        // - X-BACKEND-TEST header (from backend) should be present
        // - Both headers should have correct values
        // - Other headers should still be present

        if !ensure_container_running() {
            eprintln!("Failed to start container. Skipping test.");
            return;
        }

        // Use OPTIONS method which skips payment verification
        // Retry logic: sometimes nginx needs a moment to be fully ready, especially under concurrent test execution
        let url = format!("http://localhost:{NGINX_PORT}/api/protected-proxy");
        let mut response_text_opt = None;
        let mut retries = 5;

        while retries > 0 {
            let mut args = vec!["-s", "-i", "-X", "OPTIONS"];
            args.push("-H");
            args.push("Origin: https://example.com");
            args.push(&url);

            let output = Command::new("curl")
                .args(args)
                .output()
                .ok()
                .map(|output| String::from_utf8_lossy(&output.stdout).to_string());

            if let Some(text) = output {
                let status_line = text.lines().next().unwrap_or("");
                // Check if we got a valid response (not empty)
                if !status_line.is_empty()
                    && (status_line.contains("HTTP")
                        || status_line.contains("200")
                        || status_line.contains("204")
                        || status_line.contains("402"))
                {
                    response_text_opt = Some(text);
                    break;
                }
            }
            retries -= 1;
            thread::sleep(Duration::from_millis(500));
        }

        if let Some(response_text) = response_text_opt {
            // Extract status code
            let status_line = response_text.lines().next().unwrap_or("");
            let status = if status_line.contains("200") {
                "200"
            } else if status_line.contains("204") {
                "204"
            } else if status_line.contains("402") {
                "402"
            } else {
                status_line
            };

            // OPTIONS should skip payment verification
            assert!(
                status == "200" || status == "204",
                "OPTIONS request should return 200/204 (payment skipped), got {status}. Response: {}",
                response_text
            );
            assert_ne!(
                status, "402",
                "OPTIONS request should skip payment verification (got 402)"
            );

            // Check for nginx-configured header
            let nginx_header = get_header_value(&response_text, "X-NGINX-TEST");
            assert!(
                nginx_header.is_some(),
                "X-NGINX-TEST header (from nginx add_header) should be present. Response: {}",
                response_text
            );
            assert_eq!(
                nginx_header.unwrap(),
                "nginx-header-value",
                "X-NGINX-TEST should be 'nginx-header-value'"
            );

            // Check for backend header
            let backend_header = get_header_value(&response_text, "X-BACKEND-TEST");
            assert!(
                backend_header.is_some(),
                "X-BACKEND-TEST header (from backend) should be present. Response: {}",
                response_text
            );
            assert_eq!(
                backend_header.unwrap(),
                "backend-header-value",
                "X-BACKEND-TEST should be 'backend-header-value'"
            );

            // Verify both headers exist together
            let nginx_count = response_text.matches("X-NGINX-TEST").count();
            let backend_count = response_text.matches("X-BACKEND-TEST").count();

            assert!(
                nginx_count >= 1,
                "X-NGINX-TEST should appear at least once in response"
            );
            assert!(
                backend_count >= 1,
                "X-BACKEND-TEST should appear at least once in response"
            );

            // Also verify other headers are still present
            let custom_header = get_header_value(&response_text, "X-Custom-Response-Header");
            let allow_origin = get_header_value(&response_text, "Access-Control-Allow-Origin");

            assert!(
                custom_header.is_some(),
                "X-Custom-Response-Header should still be present. Response: {}",
                response_text
            );
            assert!(
                allow_origin.is_some(),
                "Access-Control-Allow-Origin should still be present. Response: {}",
                response_text
            );
        } else {
            panic!("Failed to make OPTIONS request");
        }
    }

    #[test]
    #[ignore = "requires Docker"]
    fn test_nginx_and_backend_custom_headers_with_head() {
        // Test Case: Both nginx-configured headers and backend headers with HEAD method
        //
        // This test verifies that HEAD requests also preserve both nginx and backend headers
        //
        // Expected behavior:
        // - HEAD request should return 200 (not 402)
        // - X-NGINX-TEST header (from nginx) should be present
        // - X-BACKEND-TEST header (from backend) should be present

        if !ensure_container_running() {
            eprintln!("Failed to start container. Skipping test.");
            return;
        }

        // Use HEAD method which skips payment verification
        // Retry logic: sometimes nginx needs a moment to be fully ready, especially under concurrent test execution
        let url = format!("http://localhost:{NGINX_PORT}/api/protected-proxy");
        let mut response_text_opt = None;
        let mut retries = 5;

        while retries > 0 {
            let mut args = vec!["-s", "-i", "-X", "HEAD"];
            args.push("-H");
            args.push("Origin: https://example.com");
            args.push(&url);

            let output = Command::new("curl")
                .args(args)
                .output()
                .ok()
                .map(|output| String::from_utf8_lossy(&output.stdout).to_string());

            if let Some(text) = output {
                let status_line = text.lines().next().unwrap_or("");
                // Check if we got a valid response (not empty)
                if !status_line.is_empty()
                    && (status_line.contains("HTTP")
                        || status_line.contains("200")
                        || status_line.contains("204")
                        || status_line.contains("402"))
                {
                    response_text_opt = Some(text);
                    break;
                }
            }
            retries -= 1;
            thread::sleep(Duration::from_millis(500));
        }

        if let Some(response_text) = response_text_opt {
            // Extract status code
            let status_line = response_text.lines().next().unwrap_or("");
            let status = if status_line.contains("200") {
                "200"
            } else if status_line.contains("204") {
                "204"
            } else if status_line.contains("402") {
                "402"
            } else {
                status_line
            };

            // HEAD should skip payment verification
            assert!(
                status == "200" || status == "204",
                "HEAD request should return 200/204 (payment skipped), got {status}. Response: {}",
                response_text
            );
            assert_ne!(
                status, "402",
                "HEAD request should skip payment verification (got 402)"
            );

            // Check for nginx-configured header
            let nginx_header = get_header_value(&response_text, "X-NGINX-TEST");
            assert!(
                nginx_header.is_some(),
                "X-NGINX-TEST header (from nginx add_header) should be present in HEAD response. Response: {}",
                response_text
            );
            assert_eq!(
                nginx_header.unwrap(),
                "nginx-header-value",
                "X-NGINX-TEST should be 'nginx-header-value'"
            );

            // Check for backend header
            let backend_header = get_header_value(&response_text, "X-BACKEND-TEST");
            assert!(
                backend_header.is_some(),
                "X-BACKEND-TEST header (from backend) should be present in HEAD response. Response: {}",
                response_text
            );
            assert_eq!(
                backend_header.unwrap(),
                "backend-header-value",
                "X-BACKEND-TEST should be 'backend-header-value'"
            );
        } else {
            panic!("Failed to make HEAD request");
        }
    }
}
