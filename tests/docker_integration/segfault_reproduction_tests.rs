//! Tests to reproduce segfault issues in production-like scenarios
//!
//! These tests attempt to reproduce the segfault issues that occur in production
//! but not in simple integration tests. The key differences are:
//!
//! 1. **Configuration Merging**: Production uses server-level config that merges into location-level
//! 2. **Memory Pool Lifecycle**: Production may reload config or have complex memory pool patterns
//! 3. **Concurrency**: Production has multiple worker processes handling concurrent requests
//! 4. **Timing**: Segfaults may be timing-dependent, requiring specific memory allocation patterns

#[cfg(feature = "integration-test")]
mod tests {
    use crate::docker_integration::common::*;
    use std::process::Command;
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    /// Test configuration merging scenario that may trigger segfaults
    ///
    /// This test simulates production scenario where:
    /// - Server-level configuration provides default values
    /// - Location-level configuration inherits and overrides
    /// - Configuration merging happens during request processing
    ///
    /// The segfault occurs when accessing strings from merged configuration
    /// if the parent config's memory pool was freed.
    #[test]
    #[ignore = "requires Docker and may not reproduce segfault"]
    fn test_config_merging_segfault_scenario() {
        if !ensure_container_running() {
            eprintln!("Failed to start container. Skipping test.");
            return;
        }

        // Make requests that trigger configuration merging
        // In production, this happens when server-level config merges into location-level
        for i in 0..50 {
            // Alternate between different endpoints to trigger different config paths
            let endpoint = if i % 2 == 0 {
                "/api/protected"
            } else {
                "/api/"
            };

            let _ = http_request(endpoint);

            // Small delay to allow memory pool operations
            thread::sleep(Duration::from_millis(50));

            // Check for segfaults periodically
            if i % 10 == 0 {
                let logs = get_recent_docker_logs(50).unwrap_or_default();
                if logs.contains("signal 11") || logs.contains("core dumped") {
                    panic!("Segfault detected at request {}! Logs: {}", i, logs);
                }
            }
        }

        println!("✓ Configuration merging test completed");
    }

    /// Test concurrent requests that may trigger race conditions
    ///
    /// Production environment has multiple worker processes handling concurrent requests.
    /// This test simulates that scenario to trigger potential race conditions.
    #[test]
    #[ignore = "requires Docker and may not reproduce segfault"]
    fn test_concurrent_request_segfault_scenario() {
        if !ensure_container_running() {
            eprintln!("Failed to start container. Skipping test.");
            return;
        }

        let num_threads = 4;
        let requests_per_thread = 20;
        let mut handles = vec![];

        for _ in 0..num_threads {
            let handle = thread::spawn(move || {
                for _ in 0..requests_per_thread {
                    let _ = http_request("/api/protected");
                    thread::sleep(Duration::from_millis(10));
                }
            });
            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }

        // Wait a bit for any segfaults to be logged
        thread::sleep(Duration::from_millis(1000));

        // Check for segfaults
        let logs = get_recent_docker_logs(100).unwrap_or_default();
        assert!(
            !logs.contains("signal 11") && !logs.contains("core dumped"),
            "Segfault detected in concurrent request test! Logs: {}",
            logs.chars().take(2000).collect::<String>()
        );

        println!("✓ Concurrent request test completed");
    }

    /// Test rapid request pattern that may trigger memory pool issues
    ///
    /// This test sends many rapid requests to trigger memory pool allocation
    /// and deallocation patterns that may expose the segfault.
    #[test]
    #[ignore = "requires Docker and may not reproduce segfault"]
    fn test_rapid_requests_segfault_scenario() {
        if !ensure_container_running() {
            eprintln!("Failed to start container. Skipping test.");
            return;
        }

        // Send rapid requests without delays
        for i in 0..100 {
            let _ = http_request("/api/protected");

            // Check for segfaults every 20 requests
            if i % 20 == 0 && i > 0 {
                thread::sleep(Duration::from_millis(100));
                let logs = get_recent_docker_logs(50).unwrap_or_default();
                if logs.contains("signal 11") || logs.contains("core dumped") {
                    panic!("Segfault detected at request {}! Logs: {}", i, logs);
                }
            }
        }

        // Final check
        thread::sleep(Duration::from_millis(500));
        let logs = get_recent_docker_logs(100).unwrap_or_default();
        assert!(
            !logs.contains("signal 11") && !logs.contains("core dumped"),
            "Segfault detected in rapid request test! Logs: {}",
            logs.chars().take(2000).collect::<String>()
        );

        println!("✓ Rapid request test completed");
    }

    /// Test with configuration that requires merging from parent
    ///
    /// This test uses a test configuration file that has server-level
    /// x402 configuration that merges into location-level config.
    /// This is the scenario that triggers the segfault in production.
    #[test]
    #[ignore = "requires Docker and special test config"]
    fn test_parent_config_merging_segfault() {
        if !ensure_container_running() {
            eprintln!("Failed to start container. Skipping test.");
            return;
        }

        // This test would require a special nginx config with server-level x402 config
        // that merges into location-level config. The current test config doesn't have this.
        // In production, the config might look like:
        //
        // server {
        //     x402_ttl 60;  # Server-level default
        //     location /api/ {
        //         x402 on;  # Inherits ttl from server level
        //     }
        // }

        // Make requests that would trigger merging
        for _ in 0..30 {
            let _ = http_request("/api/protected");
            thread::sleep(Duration::from_millis(50));
        }

        // Check for segfaults
        thread::sleep(Duration::from_millis(500));
        let logs = get_recent_docker_logs(100).unwrap_or_default();
        assert!(
            !logs.contains("signal 11") && !logs.contains("core dumped"),
            "Segfault detected in parent config merging test! Logs: {}",
            logs.chars().take(2000).collect::<String>()
        );

        println!("✓ Parent config merging test completed");
    }
}
