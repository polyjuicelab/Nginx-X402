//! Integration tests for nginx-x402 module
//!
//! These tests require a built module and a running nginx instance.
//! They can be run manually or in CI/CD with Docker.
//!
//! To run manually:
//!   1. Build the module: cargo build --release
//!   2. Set up nginx with the module loaded
//!   3. Run: cargo test --test integration_test

#[cfg(test)]
mod tests {
    use std::process::Command;
    use std::time::Duration;
    use std::thread;

    /// Test helper to check if nginx is running
    fn nginx_is_running() -> bool {
        Command::new("curl")
            .args(&["-s", "-o", "/dev/null", "-w", "%{http_code}", "http://localhost:8080/health"])
            .output()
            .map(|output| {
                String::from_utf8_lossy(&output.stdout) == "200"
            })
            .unwrap_or(false)
    }

    /// Test helper to wait for nginx to be ready
    fn wait_for_nginx(max_wait: Duration) -> bool {
        let start = std::time::Instant::now();
        while start.elapsed() < max_wait {
            if nginx_is_running() {
                return true;
            }
            thread::sleep(Duration::from_millis(100));
        }
        false
    }

    #[test]
    #[ignore] // Ignore by default - requires nginx setup
    fn test_module_loads() {
        // This test verifies that the module can be loaded by nginx
        // It requires:
        // 1. Module built: cargo build --release
        // 2. Nginx configured with: load_module /path/to/libnginx_x402.so;
        // 3. Nginx running: sudo nginx
        
        let output = Command::new("nginx")
            .arg("-t")
            .output()
            .expect("Failed to run nginx -t");
        
        assert!(
            output.status.success(),
            "Nginx configuration test failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    #[ignore]
    fn test_402_response_without_payment() {
        // Test that a request without X-PAYMENT header returns 402
        if !wait_for_nginx(Duration::from_secs(5)) {
            eprintln!("Nginx is not running. Skipping test.");
            return;
        }

        let output = Command::new("curl")
            .args(&[
                "-s", "-o", "/dev/null", "-w", "%{http_code}",
                "http://localhost:8080/api/protected"
            ])
            .output()
            .expect("Failed to run curl");

        let status_code = String::from_utf8_lossy(&output.stdout);
        assert_eq!(status_code.trim(), "402", "Expected 402, got {}", status_code);
    }

    #[test]
    #[ignore]
    fn test_public_endpoint_accessible() {
        // Test that public endpoints (without x402) are accessible
        if !wait_for_nginx(Duration::from_secs(5)) {
            eprintln!("Nginx is not running. Skipping test.");
            return;
        }

        let output = Command::new("curl")
            .args(&[
                "-s", "-o", "/dev/null", "-w", "%{http_code}",
                "http://localhost:8080/health"
            ])
            .output()
            .expect("Failed to run curl");

        let status_code = String::from_utf8_lossy(&output.stdout);
        assert_eq!(status_code.trim(), "200", "Expected 200, got {}", status_code);
    }

    #[test]
    #[ignore]
    fn test_metrics_endpoint() {
        // Test that metrics endpoint is accessible
        if !wait_for_nginx(Duration::from_secs(5)) {
            eprintln!("Nginx is not running. Skipping test.");
            return;
        }

        let output = Command::new("curl")
            .args(&[
                "-s",
                "http://localhost:8080/metrics"
            ])
            .output()
            .expect("Failed to run curl");

        let metrics = String::from_utf8_lossy(&output.stdout);
        assert!(
            metrics.contains("x402_requests_total") || metrics.contains("# HELP"),
            "Metrics endpoint should return Prometheus metrics"
        );
    }
}

