//! Tests for timeout functionality
//!
//! These tests verify that timeout handling works correctly,
//! covering the fix for:
//! - fix-4: Network request timeout control

mod tests {
    use rust_x402::facilitator::FacilitatorClient;
    use rust_x402::types::{FacilitatorConfig, PaymentRequirements};
    use std::time::Duration;

    #[allow(dead_code)]
    fn create_test_requirements() -> PaymentRequirements {
        PaymentRequirements::new(
            "exact",
            "base-sepolia",
            "100",
            "0x036CbD53842c5426634e7929541eC2318f3dCF7e",
            "0x209693bc6afc0c5328ba36faf03c514ef312287c",
            "/test",
            "Test payment",
        )
    }

    #[tokio::test]
    async fn test_facilitator_config_with_timeout() {
        // Test that FacilitatorConfig can be created with timeout
        let config = FacilitatorConfig::new("https://x402.org/facilitator")
            .with_timeout(Duration::from_secs(5));

        assert!(config.timeout.is_some(), "Timeout should be set");
        assert_eq!(
            config.timeout.unwrap(),
            Duration::from_secs(5),
            "Timeout should be exactly 5 seconds"
        );
    }

    #[tokio::test]
    async fn test_facilitator_config_without_timeout() {
        // Test that FacilitatorConfig can be created without timeout
        let config = FacilitatorConfig::new("https://x402.org/facilitator");

        assert!(
            config.timeout.is_none(),
            "Timeout should not be set by default"
        );
    }

    #[tokio::test]
    async fn test_facilitator_client_uses_timeout() {
        // Test that FacilitatorClient respects the timeout configuration
        let config = FacilitatorConfig::new("https://x402.org/facilitator")
            .with_timeout(Duration::from_millis(100)); // Very short timeout

        let client_result = FacilitatorClient::new(config);
        assert!(
            client_result.is_ok(),
            "Client should be created successfully with timeout"
        );

        // The timeout is applied at the HTTP client level, so we can't easily test
        // it without making actual network requests. This test verifies the
        // configuration is accepted.
    }

    #[tokio::test]
    async fn test_timeout_duration_values() {
        // Test various timeout values
        let timeout_values = vec![
            Duration::from_millis(1),
            Duration::from_millis(100),
            Duration::from_secs(1),
            Duration::from_secs(5),
            Duration::from_secs(10),
            Duration::from_secs(30),
            Duration::from_secs(60),
        ];

        for timeout in timeout_values {
            let config =
                FacilitatorConfig::new("https://x402.org/facilitator").with_timeout(timeout);

            assert!(
                config.timeout.is_some(),
                "Timeout should be set for duration {:?}",
                timeout
            );
            assert_eq!(
                config.timeout.unwrap(),
                timeout,
                "Timeout should match input duration"
            );
        }
    }
}
