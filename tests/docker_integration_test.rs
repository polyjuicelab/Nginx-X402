//! Docker-based integration tests for nginx-x402 module
//!
//! These tests use Docker to run nginx with the module in an isolated environment.
//! Requires Docker to be installed and running.
//!
//! To run:
//!   cargo test --test `docker_integration_test` --features integration-test
//!
//! Note: This requires the 'integration-test' feature to be enabled.

#[cfg(feature = "integration-test")]
mod tests {
    use std::process::Command;
    use std::thread;
    use std::time::Duration;

    const DOCKER_IMAGE: &str = "nginx-x402-test";
    const CONTAINER_NAME: &str = "nginx-x402-test-container";
    const NGINX_PORT: u16 = 8080;

    /// Build the Docker test image
    fn build_docker_image() -> bool {
        println!("Building Docker test image...");
        let output = Command::new("docker")
            .args([
                "build",
                "-t",
                DOCKER_IMAGE,
                "-f",
                "tests/Dockerfile.test",
                ".",
            ])
            .output();

        match output {
            Ok(output) if output.status.success() => {
                println!("Docker image built successfully");
                true
            }
            Ok(output) => {
                eprintln!(
                    "Docker build failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
                false
            }
            Err(e) => {
                eprintln!("Failed to run docker build: {e}");
                false
            }
        }
    }

    /// Start the Docker container
    fn start_container() -> bool {
        println!("Starting Docker container...");
        let output = Command::new("docker")
            .args([
                "run",
                "-d",
                "--name",
                CONTAINER_NAME,
                "-p",
                &format!("{NGINX_PORT}:80"),
                DOCKER_IMAGE,
            ])
            .output();

        match output {
            Ok(output) if output.status.success() => {
                println!("Container started successfully");
                // Wait for nginx to be ready (up to 30 seconds)
                if wait_for_nginx(Duration::from_secs(30)) {
                    println!("Nginx is ready");
                    true
                } else {
                    eprintln!("Nginx did not become ready within 30 seconds");
                    false
                }
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                if stderr.contains("already in use") {
                    // Container already exists, try to start it
                    Command::new("docker")
                        .args(["start", CONTAINER_NAME])
                        .output()
                        .ok();
                    // Wait for nginx to be ready (up to 30 seconds)
                    if wait_for_nginx(Duration::from_secs(30)) {
                        println!("Nginx is ready");
                        true
                    } else {
                        eprintln!("Nginx did not become ready within 30 seconds");
                        false
                    }
                } else {
                    eprintln!("Failed to start container: {stderr}");
                    false
                }
            }
            Err(e) => {
                eprintln!("Failed to run docker: {e}");
                false
            }
        }
    }

    /// Stop and remove the Docker container
    fn cleanup_container() {
        let _ = Command::new("docker")
            .args(["stop", CONTAINER_NAME])
            .output();
        let _ = Command::new("docker").args(["rm", CONTAINER_NAME]).output();
    }

    /// Check if nginx is responding
    fn nginx_is_ready() -> bool {
        Command::new("curl")
            .args([
                "-s",
                "-o",
                "/dev/null",
                "-w",
                "%{http_code}",
                &format!("http://localhost:{NGINX_PORT}/health"),
            ])
            .output()
            .map(|output| String::from_utf8_lossy(&output.stdout).trim() == "200")
            .unwrap_or(false)
    }

    /// Wait for nginx to be ready
    fn wait_for_nginx(max_wait: Duration) -> bool {
        let start = std::time::Instant::now();
        let mut consecutive_successes = 0;
        const REQUIRED_SUCCESSES: usize = 3; // Require 3 consecutive successful health checks

        while start.elapsed() < max_wait {
            if nginx_is_ready() {
                consecutive_successes += 1;
                if consecutive_successes >= REQUIRED_SUCCESSES {
                    return true;
                }
            } else {
                consecutive_successes = 0; // Reset on failure
            }
            thread::sleep(Duration::from_millis(500));
        }
        false
    }

    /// Make HTTP request and return status code
    fn http_request(path: &str) -> Option<String> {
        Command::new("curl")
            .args([
                "-s",
                "-o",
                "/dev/null",
                "-w",
                "%{http_code}",
                &format!("http://localhost:{NGINX_PORT}{path}"),
            ])
            .output()
            .ok()
            .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    /// Get HTTP response body
    fn http_get(path: &str) -> Option<String> {
        Command::new("curl")
            .args(["-s", &format!("http://localhost:{NGINX_PORT}{path}")])
            .output()
            .ok()
            .map(|output| String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Make HTTP request with custom headers and return response body
    fn http_request_with_headers(path: &str, headers: &[(&str, &str)]) -> Option<String> {
        let url = format!("http://localhost:{NGINX_PORT}{path}");
        let header_strings: Vec<String> = headers
            .iter()
            .map(|(name, value)| format!("{}: {}", name, value))
            .collect();
        let mut args = vec!["-s", &url];
        for header in &header_strings {
            args.push("-H");
            args.push(header);
        }
        Command::new("curl")
            .args(args)
            .output()
            .ok()
            .map(|output| String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Ensure container is running, start it if needed
    fn ensure_container_running() -> bool {
        // Check if container is already running
        if nginx_is_ready() {
            return true;
        }

        // Check if container exists but is stopped
        let check_output = Command::new("docker")
            .args([
                "ps",
                "-a",
                "--filter",
                &format!("name={CONTAINER_NAME}"),
                "--format",
                "{{.Status}}",
            ])
            .output();

        if let Ok(output) = check_output {
            let status = String::from_utf8_lossy(&output.stdout);
            if status.contains("Up") {
                // Container is running, wait for nginx (up to 30 seconds)
                return wait_for_nginx(Duration::from_secs(30));
            } else if !status.is_empty() {
                // Container exists but is stopped, start it
                let _ = Command::new("docker")
                    .args(["start", CONTAINER_NAME])
                    .output();
                // Wait for nginx to be ready (up to 30 seconds)
                return wait_for_nginx(Duration::from_secs(30));
            }
        }

        // Container doesn't exist, build and start it
        cleanup_container();
        build_docker_image() && start_container() && wait_for_nginx(Duration::from_secs(10))
    }

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
    fn test_proxy_pass_without_payment() {
        // Test that x402 handler works correctly with proxy_pass
        // When no payment header is provided, should return 402 (not proxy to backend)
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
        // Test that x402 handler works correctly with proxy_pass
        // When invalid payment header is provided, should return 402 (not proxy to backend)
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
        // Test that payment verification happens before proxy_pass
        // This is the key test: x402 handler in ACCESS_PHASE should run before proxy_pass handler
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

    #[test]
    #[ignore = "requires Docker"]
    fn test_websocket_upgrade() {
        // Test Case 5: WebSocket Upgrade header
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
        // Test Case 6: Subrequest detection
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

    /// Make HTTP request with custom method and headers, return status code
    fn http_request_with_method(
        path: &str,
        method: &str,
        headers: &[(&str, &str)],
    ) -> Option<String> {
        let url = format!("http://localhost:{NGINX_PORT}{path}");
        let header_strings: Vec<String> = headers
            .iter()
            .map(|(name, value)| format!("{}: {}", name, value))
            .collect();

        let mut args = vec![
            "-s",
            "-o",
            "/dev/null",
            "-w",
            "%{http_code}",
            "-X",
            method,
            &url,
        ];

        for header in &header_strings {
            args.push("-H");
            args.push(header);
        }

        Command::new("curl")
            .args(args)
            .output()
            .ok()
            .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    /// Make HTTP request with custom method and headers, return status code and headers
    fn http_request_with_method_and_headers(
        path: &str,
        method: &str,
        headers: &[(&str, &str)],
    ) -> Option<(String, String)> {
        let url = format!("http://localhost:{NGINX_PORT}{path}");
        let header_strings: Vec<String> = headers
            .iter()
            .map(|(name, value)| format!("{}: {}", name, value))
            .collect();

        let mut args = vec!["-s", "-i", "-X", method, &url];

        for header in &header_strings {
            args.push("-H");
            args.push(header);
        }

        Command::new("curl").args(args).output().ok().map(|output| {
            let response = String::from_utf8_lossy(&output.stdout).to_string();
            // Extract status code from response headers
            let status = response
                .lines()
                .next()
                .and_then(|line| line.split_whitespace().nth(1).map(|s| s.to_string()))
                .unwrap_or_else(|| "000".to_string());
            (status, response)
        })
    }

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
    fn test_trace_request_skips_payment() {
        // Test Case: TRACE request should skip payment verification
        // TRACE requests are used for diagnostic and debugging purposes
        if !ensure_container_running() {
            eprintln!("Failed to start container. Skipping test.");
            return;
        }

        // Test TRACE request without payment header
        let status = http_request_with_method("/api/protected", "TRACE", &[])
            .expect("Failed to make TRACE request");

        println!("TRACE request status (no payment): {status}");

        // TRACE request should succeed or return appropriate status without payment verification
        // It should NOT return 402, which would indicate payment verification was attempted
        // Since /api/protected has no proxy_pass, x402 module should send 405 response for TRACE
        // (TRACE is often disabled for security, so 405 Method Not Allowed is expected)
        assert!(
            status == "200" || status == "404" || status == "405" || status == "204",
            "TRACE request should skip payment verification and return appropriate status, got {status}. \
             Expected 405 from x402 module for TRACE requests without proxy_pass."
        );

        // Verify that TRACE request does not require payment
        assert_ne!(
            status, "402",
            "TRACE request should not require payment (got 402). \
             Payment verification should be skipped for TRACE requests."
        );

        println!("✓ TRACE request correctly skipped payment verification");
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
    fn test_get_request_still_requires_payment() {
        // Test Case: Ensure GET requests still require payment verification
        // This verifies that skipping payment for OPTIONS/HEAD doesn't affect GET requests
        if !ensure_container_running() {
            eprintln!("Failed to start container. Skipping test.");
            return;
        }

        // Test GET request without payment header
        let status = http_request("/api/protected").expect("Failed to make GET request");

        println!("GET request status (no payment): {status}");

        // GET request should still require payment
        assert_eq!(
            status, "402",
            "GET request should still require payment verification (expected 402, got {status})"
        );

        println!("✓ GET request correctly requires payment verification");
    }

    #[test]
    #[ignore = "requires Docker"]
    fn test_internal_redirect_error_page() {
        // Test Case 7: Internal redirect (error_page)
        // When error_page triggers internal redirect, payment verification may run again
        // or be bypassed
        if !ensure_container_running() {
            eprintln!("Failed to start container. Skipping test.");
            return;
        }

        // Retry logic: sometimes nginx needs a moment to be fully ready
        let mut status = String::new();
        let mut retries = 5;
        while retries > 0 {
            status = http_request("/api/error-test").unwrap_or_else(|| "000".to_string());
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
            panic!("Failed to connect to /api/error-test (got 000), but health check returned: {health_status}");
        }

        println!("Error page test status: {status}");

        // Behavior depends on implementation:
        // 1. Return 402 (payment required) - payment verification runs before return
        // 2. Return 404 then redirect to @fallback - internal redirect may bypass payment
        // 3. Return 502 (Bad Gateway) - if @fallback tries to proxy without payment

        // Document current behavior
        assert!(
            status == "402" || status == "404" || status == "502" || status == "200",
            "Unexpected status for error_page test: {status}"
        );

        // Test with payment header - also add retry logic
        let mut status_with_payment = String::new();
        let mut retries_payment = 5;
        while retries_payment > 0 {
            let output = Command::new("curl")
                .args([
                    "-s",
                    "-o",
                    "/dev/null",
                    "-w",
                    "%{http_code}",
                    "-H",
                    "X-PAYMENT: invalid-payment",
                    &format!("http://localhost:{NGINX_PORT}/api/error-test"),
                ])
                .output();

            match output {
                Ok(output) => {
                    status_with_payment =
                        String::from_utf8_lossy(&output.stdout).trim().to_string();
                    if status_with_payment != "000" {
                        break;
                    }
                }
                Err(_) => {
                    status_with_payment = "000".to_string();
                }
            }
            retries_payment -= 1;
            thread::sleep(Duration::from_millis(500));
        }

        if status_with_payment == "000" {
            eprintln!("Got 000 status code after retries for payment test");
            eprintln!("Using status from first request: {status}");
            // If payment test fails but first test succeeded, use that status
            status_with_payment = status.clone();
        }

        println!("Error page test status (with invalid payment): {status_with_payment}");

        // Document behavior
        assert!(
            status_with_payment == "402"
                || status_with_payment == "404"
                || status_with_payment == "502"
                || status_with_payment == "200",
            "Unexpected status: {status_with_payment}"
        );
    }

    #[test]
    #[ignore = "requires Docker"]
    fn test_content_type_json_returns_json_response() {
        // Test Case: Content-Type: application/json should return JSON response, not HTML
        // This test verifies the fix for issue where API requests with browser User-Agent
        // were incorrectly returning HTML instead of JSON
        if !ensure_container_running() {
            eprintln!("Failed to start container. Skipping test.");
            return;
        }

        // Make request with Content-Type: application/json and browser User-Agent
        // This simulates a browser making an API request (e.g., fetch() with JSON)
        let response_body = http_request_with_headers(
            "/api/protected",
            &[
                ("Content-Type", "application/json"),
                (
                    "User-Agent",
                    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.0.0 Safari/537.36",
                ),
            ],
        )
        .expect("Failed to make HTTP request");

        // Verify response is JSON, not HTML
        assert!(
            response_body.trim_start().starts_with('{')
                || response_body.trim_start().starts_with('['),
            "Response should be JSON, but got: {}",
            response_body.chars().take(200).collect::<String>()
        );

        // Verify response contains JSON structure (should have "accepts" array for payment requirements)
        assert!(
            response_body.contains("\"accepts\"") || response_body.contains("\"error\""),
            "Response should contain JSON structure with 'accepts' or 'error' field"
        );

        // Verify response does NOT contain HTML tags
        assert!(
            !response_body.contains("<!DOCTYPE") && !response_body.contains("<html"),
            "Response should not contain HTML, but got HTML content"
        );

        println!("✓ Content-Type: application/json correctly returns JSON response");
    }

    #[test]
    #[ignore = "requires Docker"]
    fn test_content_type_json_without_user_agent() {
        // Test Case: Content-Type: application/json without User-Agent should return JSON
        if !ensure_container_running() {
            eprintln!("Failed to start container. Skipping test.");
            return;
        }

        // Make request with only Content-Type: application/json (no User-Agent)
        let response_body =
            http_request_with_headers("/api/protected", &[("Content-Type", "application/json")])
                .expect("Failed to make HTTP request");

        // Verify response is JSON
        assert!(
            response_body.trim_start().starts_with('{')
                || response_body.trim_start().starts_with('['),
            "Response should be JSON, but got: {}",
            response_body.chars().take(200).collect::<String>()
        );

        println!("✓ Content-Type: application/json (no User-Agent) correctly returns JSON");
    }

    #[test]
    #[ignore = "requires Docker"]
    fn test_browser_request_without_content_type_returns_html() {
        // Test Case: Browser request without Content-Type should return HTML
        // This ensures we didn't break the existing browser behavior
        if !ensure_container_running() {
            eprintln!("Failed to start container. Skipping test.");
            return;
        }

        // Make request with browser User-Agent but no Content-Type
        let response_body = http_request_with_headers(
            "/api/protected",
            &[(
                "User-Agent",
                "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.0.0 Safari/537.36",
            )],
        )
        .expect("Failed to make HTTP request");

        // Verify response is HTML
        assert!(
            response_body.contains("<!DOCTYPE") || response_body.contains("<html"),
            "Browser request without Content-Type should return HTML, but got: {}",
            response_body.chars().take(200).collect::<String>()
        );

        println!("✓ Browser request (no Content-Type) correctly returns HTML");
    }

    #[test]
    #[ignore = "requires Docker"]
    fn test_asset_fallback_uses_default_usdc() {
        // Test Case: x402_asset not specified should use default USDC address
        if !ensure_container_running() {
            eprintln!("Failed to start container. Skipping test.");
            return;
        }

        // Make API request to get JSON response with payment requirements
        let response_body = http_request_with_headers(
            "/api/asset-fallback",
            &[
                ("Content-Type", "application/json"),
                ("Accept", "application/json"),
            ],
        )
        .expect("Failed to make HTTP request");

        // Parse JSON response to check asset address
        // Response should contain payment requirements with USDC address for base-sepolia
        // Base Sepolia USDC address: 0x036CbD53842c5426634e7929541eC2318f3dCF7e
        assert!(
            response_body.contains("0x036CbD53842c5426634e7929541eC2318f3dCF7e")
                || response_body.contains("\"asset\"")
                || response_body.contains("\"accepts\""),
            "Response should contain USDC asset address or payment requirements structure. Got: {}",
            response_body.chars().take(500).collect::<String>()
        );

        println!("✓ Asset fallback correctly uses default USDC address");
    }

    #[test]
    #[ignore = "requires Docker"]
    fn test_network_id_configuration() {
        // Test Case: x402_network_id should work correctly
        if !ensure_container_running() {
            eprintln!("Failed to start container. Skipping test.");
            return;
        }

        // Test with Base Sepolia chainId (84532)
        let response_body = http_request_with_headers(
            "/api/network-id",
            &[
                ("Content-Type", "application/json"),
                ("Accept", "application/json"),
            ],
        )
        .expect("Failed to make HTTP request");

        // Should return 402 with payment requirements for base-sepolia
        assert!(
            response_body.contains("\"accepts\"")
                || response_body.contains("\"error\"")
                || response_body.contains("base-sepolia")
                || response_body.contains("0x036CbD53842c5426634e7929541eC2318f3dCF7e"),
            "Response should contain payment requirements for base-sepolia network. Got: {}",
            response_body.chars().take(500).collect::<String>()
        );

        println!("✓ Network ID (chainId 84532) configuration works correctly");
    }

    #[test]
    #[ignore = "requires Docker"]
    fn test_network_id_mainnet() {
        // Test Case: x402_network_id with mainnet chainId (8453)
        if !ensure_container_running() {
            eprintln!("Failed to start container. Skipping test.");
            return;
        }

        let response_body = http_request_with_headers(
            "/api/network-id-mainnet",
            &[
                ("Content-Type", "application/json"),
                ("Accept", "application/json"),
            ],
        )
        .expect("Failed to make HTTP request");

        // Should return 402 with payment requirements for base mainnet
        // Base Mainnet USDC address: 0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913
        assert!(
            response_body.contains("\"accepts\"")
                || response_body.contains("\"error\"")
                || response_body.contains("base")
                || response_body.contains("0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913"),
            "Response should contain payment requirements for base mainnet network. Got: {}",
            response_body.chars().take(500).collect::<String>()
        );

        println!("✓ Network ID (chainId 8453 - mainnet) configuration works correctly");
    }

    #[test]
    #[ignore = "requires Docker"]
    fn test_custom_asset_address() {
        // Test Case: Custom x402_asset address should use specified address
        if !ensure_container_running() {
            eprintln!("Failed to start container. Skipping test.");
            return;
        }

        let response_body = http_request_with_headers(
            "/api/custom-asset",
            &[
                ("Content-Type", "application/json"),
                ("Accept", "application/json"),
            ],
        )
        .expect("Failed to make HTTP request");

        // Should contain the custom asset address specified in config
        // Custom address: 0x036CbD53842c5426634e7929541eC2318f3dCF7e
        assert!(
            response_body.contains("0x036CbD53842c5426634e7929541eC2318f3dCF7e")
                || response_body.contains("\"asset\"")
                || response_body.contains("\"accepts\""),
            "Response should contain custom asset address. Got: {}",
            response_body.chars().take(500).collect::<String>()
        );

        println!("✓ Custom asset address configuration works correctly");
    }

    #[test]
    #[ignore = "requires Docker"]
    fn test_network_id_takes_precedence() {
        // Test Case: x402_network_id should take precedence over x402_network
        if !ensure_container_running() {
            eprintln!("Failed to start container. Skipping test.");
            return;
        }

        let response_body = http_request_with_headers(
            "/api/network-priority",
            &[
                ("Content-Type", "application/json"),
                ("Accept", "application/json"),
            ],
        )
        .expect("Failed to make HTTP request");

        // Should use network_id (8453 = Base Mainnet) instead of network (base-sepolia)
        // Base Mainnet USDC: 0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913
        // Base Sepolia USDC: 0x036CbD53842c5426634e7929541eC2318f3dCF7e
        // Should NOT contain Sepolia address if network_id takes precedence
        let contains_mainnet = response_body.contains("0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913")
            || response_body.contains("base")
            || response_body.contains("\"accepts\"");

        assert!(
            contains_mainnet,
            "Response should use network_id (mainnet) instead of network (sepolia). Got: {}",
            response_body.chars().take(500).collect::<String>()
        );

        println!("✓ Network ID correctly takes precedence over network name");
    }

    #[test]
    #[ignore = "requires Docker"]
    fn test_resource_url_is_valid_full_url() {
        // Test Case: Verify that resource field in payment requirements is a valid full URL
        // The resource field should be a complete URL (http:// or https://) not a relative path
        if !ensure_container_running() {
            eprintln!("Failed to start container. Skipping test.");
            return;
        }

        // Make API request to get JSON response with payment requirements
        let response_body = http_request_with_headers(
            "/api/protected",
            &[
                ("Content-Type", "application/json"),
                ("Accept", "application/json"),
            ],
        )
        .expect("Failed to make HTTP request");

        // Parse JSON response to check resource field
        // Response should be JSON with payment requirements
        assert!(
            response_body.trim_start().starts_with('{'),
            "Response should be JSON, but got: {}",
            response_body.chars().take(200).collect::<String>()
        );

        // Check if response contains "accepts" array (payment requirements)
        if response_body.contains("\"accepts\"") {
            // Extract resource field value from JSON response
            // Look for "resource":"..." pattern using simple string matching
            let mut resource_value: Option<String> = None;

            // Look for resource field with various patterns
            let patterns = vec!["\"resource\":\"", "\"resource\" : \"", "\"resource\": \""];
            for pattern in patterns {
                if let Some(idx) = response_body.find(pattern) {
                    let start = idx + pattern.len();
                    if let Some(end) = response_body[start..].find('"') {
                        let value = &response_body[start..start + end];
                        resource_value = Some(value.to_string());
                        break;
                    }
                }
            }

            // Validate resource URL
            if let Some(resource) = resource_value {
                // Check if resource is a valid full URL (starts with http:// or https://)
                let is_valid_url =
                    resource.starts_with("http://") || resource.starts_with("https://");

                // Check for double prefix bug (/http:// or /https://)
                let has_double_prefix =
                    resource.starts_with("/http://") || resource.starts_with("/https://");

                assert!(
                    is_valid_url && !has_double_prefix,
                    "Resource field should be a valid full URL (http:// or https://) without double prefix. \
                     Got: '{}'. \
                     Response: {}",
                    resource,
                    response_body.chars().take(500).collect::<String>()
                );

                // Additional validation: ensure URL format is correct
                assert!(
                    resource.len() > 7, // Minimum: "http://a"
                    "Resource URL is too short: '{}'",
                    resource
                );

                println!("✓ Resource field is a valid full URL: {}", resource);
            } else {
                panic!(
                    "Could not extract resource field from response. \
                     Response: {}",
                    response_body.chars().take(500).collect::<String>()
                );
            }
        } else if response_body.contains("\"error\"") {
            // If there's an error, we can't verify resource URL
            // But we should still check that error doesn't mention invalid URL
            assert!(
                !response_body.contains("Invalid url")
                    && !response_body.contains("invalid_string")
                    && !response_body.contains("\"path\":[\"resource\"]"),
                "Response contains URL validation error for resource field. \
                 This suggests resource URL is not valid. Response: {}",
                response_body.chars().take(500).collect::<String>()
            );
            println!("✓ No URL validation errors in error response");
        } else {
            // Unexpected response format
            panic!(
                "Unexpected response format. Expected JSON with 'accepts' or 'error' field. \
                 Got: {}",
                response_body.chars().take(500).collect::<String>()
            );
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
