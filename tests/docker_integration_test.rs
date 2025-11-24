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
                eprintln!("Failed to run docker build: {}", e);
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
                &format!("{}:80", NGINX_PORT),
                DOCKER_IMAGE,
            ])
            .output();

        match output {
            Ok(output) if output.status.success() => {
                println!("Container started successfully");
                // Wait for nginx to be ready
                thread::sleep(Duration::from_secs(2));
                true
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                if stderr.contains("already in use") {
                    // Container already exists, try to start it
                    Command::new("docker")
                        .args(["start", CONTAINER_NAME])
                        .output()
                        .ok();
                    thread::sleep(Duration::from_secs(2));
                    true
                } else {
                    eprintln!("Failed to start container: {}", stderr);
                    false
                }
            }
            Err(e) => {
                eprintln!("Failed to run docker: {}", e);
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
                &format!("http://localhost:{}/health", NGINX_PORT),
            ])
            .output()
            .map(|output| String::from_utf8_lossy(&output.stdout).trim() == "200")
            .unwrap_or(false)
    }

    /// Wait for nginx to be ready
    fn wait_for_nginx(max_wait: Duration) -> bool {
        let start = std::time::Instant::now();
        while start.elapsed() < max_wait {
            if nginx_is_ready() {
                return true;
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
                &format!("http://localhost:{}{}", NGINX_PORT, path),
            ])
            .output()
            .ok()
            .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    /// Get HTTP response body
    fn http_get(path: &str) -> Option<String> {
        Command::new("curl")
            .args(["-s", &format!("http://localhost:{}{}", NGINX_PORT, path)])
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
                &format!("name={}", CONTAINER_NAME),
                "--format",
                "{{.Status}}",
            ])
            .output();

        if let Ok(output) = check_output {
            let status = String::from_utf8_lossy(&output.stdout);
            if status.contains("Up") {
                // Container is running, wait for nginx
                return wait_for_nginx(Duration::from_secs(10));
            } else if !status.is_empty() {
                // Container exists but is stopped, start it
                let _ = Command::new("docker")
                    .args(["start", CONTAINER_NAME])
                    .output();
                return wait_for_nginx(Duration::from_secs(10));
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
    #[ignore]
    fn test_402_response() {
        if !ensure_container_running() {
            eprintln!("Failed to start container. Skipping test.");
            return;
        }

        let status = http_request("/api/protected").expect("Failed to make HTTP request");

        assert_eq!(status, "402", "Expected 402 response, got {}", status);
    }

    #[test]
    #[ignore]
    fn test_health_endpoint() {
        if !ensure_container_running() {
            eprintln!("Failed to start container. Skipping test.");
            return;
        }

        let status = http_request("/health").expect("Failed to make HTTP request");

        assert_eq!(status, "200", "Expected 200 response, got {}", status);
    }

    #[test]
    #[ignore]
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
}
