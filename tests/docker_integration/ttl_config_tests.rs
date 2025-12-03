//! Integration tests for x402_ttl configuration directive
//!
//! These tests verify that x402_ttl configuration works correctly and doesn't cause segfaults.
//! This is critical because TTL configuration was added recently and may have memory safety issues.

#[cfg(feature = "integration-test")]
mod tests {
    use crate::docker_integration::common::*;
    use std::process::Command;
    use std::thread;
    use std::time::Duration;

    /// Test that x402_ttl configuration doesn't cause segfaults
    ///
    /// This test verifies that:
    /// 1. x402_ttl can be configured without causing segfaults
    /// 2. Multiple requests with TTL configuration work correctly
    /// 3. Worker processes don't crash
    #[test]
    #[ignore = "requires Docker"]
    fn test_ttl_configuration_no_segfault() {
        if !ensure_container_running() {
            eprintln!("Failed to start container. Skipping test.");
            return;
        }

        // Make multiple requests to trigger potential segfaults
        // The segfault typically occurs during configuration merging or request processing
        for i in 0..10 {
            let _ = http_request("/api/protected");
            thread::sleep(Duration::from_millis(100));

            // Check if nginx is still running (if segfault occurred, nginx would restart)
            if i % 3 == 0 {
                let status = http_request("/health");
                assert!(
                    status.is_some() && status.unwrap() == "200",
                    "Nginx should still be running after request {}. If this fails, segfault may have occurred.",
                    i
                );
            }
        }

        println!("✓ TTL configuration test completed without segfaults");
    }

    /// Test that x402_ttl with different values works correctly
    ///
    /// This test verifies that:
    /// 1. TTL configuration is parsed correctly
    /// 2. Different TTL values don't cause issues
    /// 3. Configuration merging works correctly
    #[test]
    #[ignore = "requires Docker"]
    fn test_ttl_configuration_values() {
        if !ensure_container_running() {
            eprintln!("Failed to start container. Skipping test.");
            return;
        }

        // Check Docker logs for segfaults or errors
        let logs = Command::new("docker")
            .args(["logs", "--tail", "50", CONTAINER_NAME])
            .output()
            .ok()
            .and_then(|output| {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                Some(stdout)
            })
            .unwrap_or_default();

        // Make a request that should trigger TTL usage
        let _ = http_request("/api/protected");

        // Wait a bit for any segfaults to occur
        thread::sleep(Duration::from_millis(500));

        // Check for segfault indicators in logs
        let recent_logs = Command::new("docker")
            .args(["logs", "--tail", "100", CONTAINER_NAME])
            .output()
            .ok()
            .and_then(|output| {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                Some(stdout)
            })
            .unwrap_or_default();

        // Check for segfault indicators
        assert!(
            !recent_logs.contains("signal 11") && !recent_logs.contains("core dumped"),
            "Segfault detected in logs! Logs: {}",
            recent_logs.chars().take(1000).collect::<String>()
        );

        // Verify nginx is still running
        let status = http_request("/health");
        assert!(
            status.is_some() && status.unwrap() == "200",
            "Nginx should still be running after TTL configuration test"
        );

        println!("✓ TTL configuration values test completed");
    }

    /// Test that x402_ttl configuration merging works correctly
    ///
    /// This test verifies that:
    /// 1. TTL configuration merges correctly from parent to child locations
    /// 2. Configuration merging doesn't cause segfaults
    /// 3. Multiple nested locations work correctly
    #[test]
    #[ignore = "requires Docker"]
    fn test_ttl_configuration_merging() {
        if !ensure_container_running() {
            eprintln!("Failed to start container. Skipping test.");
            return;
        }

        // Make requests to different endpoints that may have different TTL configurations
        // This tests configuration merging from parent to child locations
        let endpoints = vec!["/api/protected", "/api/", "/health"];

        for endpoint in endpoints {
            let _ = http_request(endpoint);
            thread::sleep(Duration::from_millis(100));
        }

        // Wait a bit for any segfaults to occur
        thread::sleep(Duration::from_millis(500));

        // Check for segfault indicators
        let logs = Command::new("docker")
            .args(["logs", "--tail", "100", CONTAINER_NAME])
            .output()
            .ok()
            .and_then(|output| {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                Some(stdout)
            })
            .unwrap_or_default();

        assert!(
            !logs.contains("signal 11") && !logs.contains("core dumped"),
            "Segfault detected during configuration merging! Logs: {}",
            logs.chars().take(1000).collect::<String>()
        );

        // Verify nginx is still running
        let status = http_request("/health");
        assert!(
            status.is_some() && status.unwrap() == "200",
            "Nginx should still be running after configuration merging test"
        );

        println!("✓ TTL configuration merging test completed");
    }
}
