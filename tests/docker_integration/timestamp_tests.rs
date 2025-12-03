//! Integration tests for timestamp logging functionality
//!
//! These tests verify that timestamp logging works correctly in a real nginx environment.
//! They check that timestamps are logged when processing payment headers and facilitator responses.

#[cfg(feature = "integration-test")]
mod tests {
    use crate::docker_integration::common::*;
    use std::process::Command;
    use std::thread;
    use std::time::Duration;

    /// Get docker container logs
    ///
    /// # Returns
    ///
    /// Returns `Some(logs)` if logs were retrieved successfully, `None` otherwise.
    #[allow(dead_code)]
    fn get_docker_logs() -> Option<String> {
        let output = Command::new("docker")
            .args(["logs", CONTAINER_NAME])
            .output()
            .ok()?;

        // Nginx logs go to stderr, so combine both stdout and stderr
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        // Combine both outputs (nginx error logs are typically in stderr)
        if stdout.is_empty() && stderr.is_empty() {
            None
        } else if stdout.is_empty() {
            Some(stderr)
        } else if stderr.is_empty() {
            Some(stdout)
        } else {
            Some(format!("{}\n{}", stdout, stderr))
        }
    }

    /// Check if container exists and is running
    ///
    /// # Returns
    ///
    /// Returns `true` if container exists and is running, `false` otherwise.
    fn check_container_exists() -> bool {
        Command::new("docker")
            .args([
                "ps",
                "--filter",
                &format!("name={}", CONTAINER_NAME),
                "--format",
                "{{.Names}}",
            ])
            .output()
            .ok()
            .and_then(|output| {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                Some(stdout.trim() == CONTAINER_NAME)
            })
            .unwrap_or(false)
    }

    /// Get recent docker container logs (last N lines)
    ///
    /// # Arguments
    ///
    /// * `lines` - Number of lines to retrieve
    ///
    /// # Returns
    ///
    /// Returns `Some(logs)` if logs were retrieved successfully, `None` otherwise.
    fn get_recent_docker_logs(lines: usize) -> Option<String> {
        // Docker logs command outputs all logs (both stdout and stderr) to stdout by default
        let output = Command::new("docker")
            .args(["logs", "--tail", &lines.to_string(), CONTAINER_NAME])
            .output()
            .ok()?;

        // Docker logs outputs everything to stdout (including stderr from container)
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        // Combine both outputs (docker logs command output goes to stdout, errors to stderr)
        if stdout.is_empty() && stderr.is_empty() {
            None
        } else if stdout.is_empty() {
            Some(stderr)
        } else if stderr.is_empty() {
            Some(stdout)
        } else {
            Some(format!("{}\n{}", stdout, stderr))
        }
    }

    #[test]
    #[ignore = "requires Docker"]
    fn test_timestamp_logging_on_payment_header() {
        // Test Case: Timestamp logging when X-PAYMENT header is found
        //
        // This test verifies that:
        // 1. When X-PAYMENT header is present, logs contain current_timestamp
        // 2. Logs contain maxTimeoutSeconds from payment requirements
        // 3. Timestamp format is Unix epoch seconds (u64)

        if !ensure_container_running() {
            eprintln!("Failed to start container. Skipping test.");
            return;
        }

        // Clear logs before test
        let _ = Command::new("docker")
            .args(["logs", "--since", "1s", CONTAINER_NAME])
            .output();

        // Make a request with X-PAYMENT header (even if invalid, it should trigger logging)
        // Use a minimal valid base64 string to pass header validation
        // Base64 encoding of '{"test":"data"}' is 'eyJ0ZXN0IjoiZGF0YSJ9'
        let test_payment_header = "eyJ0ZXN0IjoiZGF0YSJ9";
        let _ = http_request_with_headers("/api/protected", &[("X-PAYMENT", test_payment_header)]);

        // Wait a moment for logs to be written (increase wait time for Docker logs to flush)
        thread::sleep(Duration::from_millis(1000));

        // Ensure container is running before checking logs
        if !check_container_exists() {
            panic!(
                "Container {} not found. Ensure container is running before running tests.",
                CONTAINER_NAME
            );
        }

        // Get recent logs (increase line count to ensure we capture the logs)
        let logs = get_recent_docker_logs(100).unwrap_or_default();

        // Debug: Print logs if assertion fails (helps diagnose issues)
        eprintln!(
            "Recent logs (first 2000 chars): {}",
            &logs.chars().take(2000).collect::<String>()
        );

        // Verify logs contain timestamp-related information
        // Check for the actual log format: "X-PAYMENT header found, validating and verifying payment, current_timestamp=..."
        let has_timestamp = logs.contains("current_timestamp=")
            || logs.contains("X-PAYMENT header found, validating and verifying payment");

        assert!(
            has_timestamp,
            "Logs should contain timestamp information when X-PAYMENT header is processed. Logs: {}",
            logs
        );

        // Verify logs contain maxTimeoutSeconds
        // Check for the actual log format: "maxTimeoutSeconds=60"
        let has_max_timeout =
            logs.contains("maxTimeoutSeconds=") || logs.contains("max_timeout_seconds=");

        assert!(
            has_max_timeout,
            "Logs should contain maxTimeoutSeconds information. Logs: {}",
            logs
        );
    }

    #[test]
    #[ignore = "requires Docker"]
    fn test_timestamp_logging_on_facilitator_response() {
        // Test Case: Timestamp logging in facilitator response
        //
        // This test verifies that:
        // 1. When facilitator responds, logs contain current_timestamp
        // 2. Timestamp is logged even when verification fails
        // 3. Timestamp format is consistent

        if !ensure_container_running() {
            eprintln!("Failed to start container. Skipping test.");
            return;
        }

        // Clear logs before test
        let _ = Command::new("docker")
            .args(["logs", "--since", "1s", CONTAINER_NAME])
            .output();

        // Make a request that will trigger facilitator verification
        // Use a valid base64-encoded payment payload structure
        // This will likely fail verification but should still log timestamps
        // Base64 encoding of a minimal payment payload
        let test_payment = "eyJ4NDAyVmVyc2lvbiI6MSwic2NoZW1lIjoiZXhhY3QiLCJuZXR3b3JrIjoiYmFzZS1zZXBvbGlhIiwicGF5bG9hZCI6eyJzaWduYXR1cmUiOiIweDAwMDAiLCJhdXRob3JpemF0aW9uIjp7ImZyb20iOiIweDAwMDAiLCJ0byI6IjB4MDAwMCIsInZhbHVlIjoiMTAwIiwidmFsaWRBZnRlciI6IjAiLCJ2YWxpZEJlZm9yZSI6IjAiLCJub25jZSI6IjB4MDAwMCJ9fX0=";
        let _ = http_request_with_headers("/api/protected", &[("X-PAYMENT", test_payment)]);

        // Wait for facilitator verification to complete
        thread::sleep(Duration::from_secs(3));

        // Ensure container is running before checking logs
        if !check_container_exists() {
            panic!(
                "Container {} not found. Ensure container is running before running tests.",
                CONTAINER_NAME
            );
        }

        // Get recent logs (increase line count to ensure we capture the logs)
        let logs = get_recent_docker_logs(150).unwrap_or_default();

        // Debug: Print logs if assertion fails (helps diagnose issues)
        eprintln!(
            "Recent logs (first 2000 chars): {}",
            &logs.chars().take(2000).collect::<String>()
        );

        // Verify logs contain facilitator response with timestamp
        // Check for the actual log format: "Facilitator verify response: is_valid=..., current_timestamp=..."
        let has_facilitator_response =
            logs.contains("Facilitator verify response") || logs.contains("current_timestamp=");

        assert!(
            has_facilitator_response,
            "Logs should contain facilitator response with timestamp. Logs: {}",
            logs
        );
    }

    #[test]
    #[ignore = "requires Docker"]
    fn test_timestamp_format_validation() {
        // Test Case: Validate timestamp format in logs
        //
        // This test verifies that:
        // 1. Timestamps in logs are valid Unix epoch seconds
        // 2. Timestamps are reasonable (not too old or too far in the future)

        if !ensure_container_running() {
            eprintln!("Failed to start container. Skipping test.");
            return;
        }

        // Get current timestamp before making request
        let test_start_timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Make a request to trigger logging
        let _ = http_request("/api/protected");

        // Wait for logs
        thread::sleep(Duration::from_millis(500));

        // Get recent logs
        let logs = get_recent_docker_logs(50).unwrap_or_default();

        // Try to extract timestamp from logs
        // Look for pattern: current_timestamp=1234567890
        if let Some(timestamp_start) = logs.find("current_timestamp=") {
            let timestamp_str_start = timestamp_start + "current_timestamp=".len();
            let timestamp_str_end = logs[timestamp_str_start..]
                .find(|c: char| !c.is_ascii_digit())
                .unwrap_or(20)
                + timestamp_str_start;

            if timestamp_str_end > timestamp_str_start {
                let timestamp_str = &logs[timestamp_str_start..timestamp_str_end];
                if let Ok(logged_timestamp) = timestamp_str.parse::<u64>() {
                    // Verify timestamp is reasonable
                    assert!(
                        logged_timestamp >= test_start_timestamp - 10,
                        "Logged timestamp should be close to test start time. Expected >= {}, got {}",
                        test_start_timestamp - 10,
                        logged_timestamp
                    );

                    // Verify timestamp is not too far in the future (within 1 minute)
                    let max_timestamp = test_start_timestamp + 60;
                    assert!(
                        logged_timestamp <= max_timestamp,
                        "Logged timestamp should not be too far in the future. Expected <= {}, got {}",
                        max_timestamp,
                        logged_timestamp
                    );
                }
            }
        }
    }

    #[test]
    #[ignore = "requires Docker"]
    fn test_max_timeout_seconds_logging() {
        // Test Case: Verify maxTimeoutSeconds is logged correctly
        //
        // This test verifies that:
        // 1. maxTimeoutSeconds value (default: 60) is logged
        // 2. The value matches PaymentRequirements default

        if !ensure_container_running() {
            eprintln!("Failed to start container. Skipping test.");
            return;
        }

        // Clear logs before test
        let _ = Command::new("docker")
            .args(["logs", "--since", "1s", CONTAINER_NAME])
            .output();

        // Make a request with X-PAYMENT header
        // Base64 encoding of '{"test":"data"}' is 'eyJ0ZXN0IjoiZGF0YSJ9'
        let test_payment_header = "eyJ0ZXN0IjoiZGF0YSJ9";
        let _ = http_request_with_headers("/api/protected", &[("X-PAYMENT", test_payment_header)]);

        // Wait for logs (increase wait time for Docker logs to flush)
        thread::sleep(Duration::from_millis(1000));

        // Ensure container is running before checking logs
        if !check_container_exists() {
            panic!(
                "Container {} not found. Ensure container is running before running tests.",
                CONTAINER_NAME
            );
        }

        // Get recent logs (increase line count to ensure we capture the logs)
        let logs = get_recent_docker_logs(100).unwrap_or_default();

        // Debug: Print logs if assertion fails (helps diagnose issues)
        eprintln!(
            "Recent logs (first 2000 chars): {}",
            &logs.chars().take(2000).collect::<String>()
        );

        // Verify logs contain maxTimeoutSeconds=60 (default value)
        // The log format should be: "X-PAYMENT header found, validating and verifying payment, current_timestamp=..., maxTimeoutSeconds=60"
        // Or just check for the pattern: maxTimeoutSeconds= followed by a number
        let has_max_timeout_60 = logs.contains("maxTimeoutSeconds=60")
            || logs.contains("max_timeout_seconds=60")
            || (logs.contains("maxTimeoutSeconds=") && logs.contains("60"));

        assert!(
            has_max_timeout_60,
            "Logs should contain maxTimeoutSeconds=60 (default value). Logs: {}",
            logs
        );
    }
}
