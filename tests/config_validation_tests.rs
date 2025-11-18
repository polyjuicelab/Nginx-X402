//! Tests for configuration validation functions
//!
//! These tests verify that all configuration validation functions work correctly,
//! covering the fixes for:
//! - fix-6: Configuration validation enhancement
//! - fix-8: Amount overflow protection

mod tests {
    use rust_x402::types::networks;

    // Import the module to access validation functions
    // Since validation functions are private, we test them through the parse() method
    use nginx_x402::X402Config;

    // Helper to create a minimal X402Config for testing
    fn create_test_config() -> X402Config {
        X402Config {
            enabled: 1,
            amount_str: ngx::ffi::ngx_str_t::default(),
            pay_to_str: ngx::ffi::ngx_str_t::default(),
            facilitator_url_str: ngx::ffi::ngx_str_t::default(),
            description_str: ngx::ffi::ngx_str_t::default(),
            network_str: ngx::ffi::ngx_str_t::default(),
            resource_str: ngx::ffi::ngx_str_t::default(),
            timeout_str: ngx::ffi::ngx_str_t::default(),
            facilitator_fallback_str: ngx::ffi::ngx_str_t::default(),
        }
    }

    // Helper to create ngx_str_t from &str
    // Note: This is a simplified version for testing only
    // In real Nginx context, ngx_str_t would be created by Nginx
    // For testing, we leak the string to ensure it lives long enough
    fn ngx_string(s: &str) -> ngx::ffi::ngx_str_t {
        use std::ffi::CString;
        // Convert to CString and leak it to ensure it lives long enough for the test
        let c_str = Box::leak(Box::new(CString::new(s).unwrap_or_default()));
        let bytes = c_str.as_bytes();
        ngx::ffi::ngx_str_t {
            len: bytes.len(),
            data: bytes.as_ptr() as *mut u8,
        }
    }

    // Note: testnet field has been removed from X402Config and ParsedX402Config
    // Network is now determined by the network_str field or defaults to BASE_MAINNET

    // ============================================================================
    // fix-6: Configuration Validation Tests
    // ============================================================================

    #[test]
    fn test_validate_ethereum_address_valid() {
        let mut config = create_test_config();
        config.pay_to_str = ngx_string("0x209693Bc6afc0C5328bA36FaF03C514EF312287C");

        let result = config.parse();
        assert!(
            result.is_ok(),
            "Valid Ethereum address should parse successfully"
        );
    }

    #[test]
    fn test_validate_ethereum_address_empty() {
        let mut config = create_test_config();
        config.pay_to_str = ngx_string("");

        let result = config.parse();
        // Empty string should result in None, not an error
        let parsed = result.unwrap();
        assert!(
            parsed.pay_to.is_none(),
            "Empty address should result in None"
        );
    }

    #[test]
    fn test_validate_ethereum_address_wrong_length() {
        let mut config = create_test_config();
        // Too short
        config.pay_to_str = ngx_string("0x123");

        let result = config.parse();
        assert!(result.is_err(), "Address with wrong length should fail");
        let error = result.err().map(|e| e.to_string()).unwrap_or_default();
        assert!(
            error.contains("Invalid Ethereum address length"),
            "Error should mention length issue, got: {}",
            error
        );
    }

    #[test]
    fn test_validate_ethereum_address_no_prefix() {
        let mut config = create_test_config();
        config.pay_to_str = ngx_string("209693Bc6afc0C5328bA36FaF03C514EF312287C");

        let result = config.parse();
        assert!(result.is_err(), "Address without 0x prefix should fail");
        let error = result.err().map(|e| e.to_string()).unwrap_or_default();
        // Validation checks length first, then prefix
        assert!(
            error.contains("must start with 0x")
                || error.contains("Invalid Ethereum address length"),
            "Error should mention 0x prefix or length issue, got: {}",
            error
        );
    }

    #[test]
    fn test_validate_ethereum_address_invalid_characters() {
        let mut config = create_test_config();
        // Contains 'g' which is not a valid hex character
        config.pay_to_str = ngx_string("0x209693Bc6afc0C5328bA36FaF03C514EF312287g");

        let result = config.parse();
        assert!(
            result.is_err(),
            "Address with invalid characters should fail"
        );
        let error = result.err().map(|e| e.to_string()).unwrap_or_default();
        assert!(
            error.contains("invalid characters") || error.contains("must be hex"),
            "Error should mention invalid characters, got: {}",
            error
        );
    }

    #[test]
    fn test_validate_ethereum_address_uppercase_prefix() {
        let mut config = create_test_config();
        config.pay_to_str = ngx_string("0X209693Bc6afc0C5328bA36FaF03C514EF312287C");

        let result = config.parse();
        assert!(
            result.is_ok(),
            "Address with uppercase 0X prefix should be valid"
        );
    }

    #[test]
    fn test_validate_url_valid_http() {
        let mut config = create_test_config();
        config.facilitator_url_str = ngx_string("http://example.com/facilitator");

        let result = config.parse();
        assert!(result.is_ok(), "Valid HTTP URL should parse successfully");
    }

    #[test]
    fn test_validate_url_valid_https() {
        let mut config = create_test_config();
        config.facilitator_url_str = ngx_string("https://x402.org/facilitator");

        let result = config.parse();
        assert!(result.is_ok(), "Valid HTTPS URL should parse successfully");
    }

    #[test]
    fn test_validate_url_empty() {
        let mut config = create_test_config();
        config.facilitator_url_str = ngx_string("");

        let result = config.parse();
        // Empty string should result in None, not an error
        let parsed = result.unwrap();
        assert!(
            parsed.facilitator_url.is_none(),
            "Empty URL should result in None"
        );
    }

    #[test]
    fn test_validate_url_no_scheme() {
        let mut config = create_test_config();
        config.facilitator_url_str = ngx_string("example.com/facilitator");

        let result = config.parse();
        assert!(result.is_err(), "URL without scheme should fail");
        let error = result.err().map(|e| e.to_string()).unwrap_or_default();
        assert!(
            error.contains("http://") || error.contains("https://"),
            "Error should mention URL scheme, got: {}",
            error
        );
    }

    #[test]
    fn test_validate_url_too_short() {
        let mut config = create_test_config();
        config.facilitator_url_str = ngx_string("http://");

        let result = config.parse();
        assert!(result.is_err(), "URL that's too short should fail");
        let error = result.err().map(|e| e.to_string()).unwrap_or_default();
        assert!(
            error.contains("too short"),
            "Error should mention URL length, got: {}",
            error
        );
    }

    #[test]
    fn test_validate_url_with_whitespace() {
        let mut config = create_test_config();
        config.facilitator_url_str = ngx_string("https://example.com/with space");

        let result = config.parse();
        assert!(result.is_err(), "URL with whitespace should fail");
        let error = result.err().map(|e| e.to_string()).unwrap_or_default();
        assert!(
            error.contains("whitespace") || error.contains("invalid"),
            "Error should mention whitespace, got: {}",
            error
        );
    }

    #[test]
    fn test_validate_network_valid() {
        let mut config = create_test_config();
        config.network_str = ngx_string("base-sepolia");

        let result = config.parse();
        assert!(result.is_ok(), "Valid network should parse successfully");
    }

    #[test]
    fn test_validate_network_empty() {
        let mut config = create_test_config();
        config.network_str = ngx_string("");

        let result = config.parse();
        // Empty string should result in None, not an error
        let parsed = result.unwrap();
        assert!(
            parsed.network.is_none(),
            "Empty network should result in None"
        );
    }

    #[test]
    fn test_validate_network_unsupported() {
        let mut config = create_test_config();
        config.network_str = ngx_string("unsupported-network");

        let result = config.parse();
        assert!(result.is_err(), "Unsupported network should fail");
        let error = result.err().map(|e| e.to_string()).unwrap_or_default();
        assert!(
            error.contains("Unsupported network") || error.contains("unsupported-network"),
            "Error should mention unsupported network, got: {}",
            error
        );
        // Error should list supported networks
        assert!(
            error.contains("base-sepolia") || error.contains("base"),
            "Error should list supported networks, got: {}",
            error
        );
    }

    #[test]
    fn test_validate_network_all_supported() {
        let supported = networks::all_supported();
        for network in supported {
            let mut config = create_test_config();
            config.network_str = ngx_string(network);

            let result = config.parse();
            assert!(
                result.is_ok(),
                "Supported network '{}' should parse successfully",
                network
            );
        }
    }

    // ============================================================================
    // fix-8: Amount Overflow Protection Tests
    // ============================================================================

    #[test]
    fn test_validate_amount_valid() {
        let mut config = create_test_config();
        config.amount_str = ngx_string("0.0001");

        let result = config.parse();
        assert!(result.is_ok(), "Valid amount should parse successfully");
    }

    #[test]
    fn test_validate_amount_zero() {
        let mut config = create_test_config();
        config.amount_str = ngx_string("0");

        let result = config.parse();
        assert!(result.is_ok(), "Zero amount should be valid");
    }

    #[test]
    fn test_validate_amount_negative() {
        let mut config = create_test_config();
        config.amount_str = ngx_string("-0.0001");

        let result = config.parse();
        assert!(result.is_err(), "Negative amount should fail");
        let error = result.err().map(|e| e.to_string()).unwrap_or_default();
        assert!(
            error.contains("negative") || error.contains("cannot be negative"),
            "Error should mention negative amount, got: {}",
            error
        );
    }

    #[test]
    fn test_validate_amount_too_large() {
        let mut config = create_test_config();
        // 1 billion + 1
        config.amount_str = ngx_string("1000000001");

        let result = config.parse();
        assert!(result.is_err(), "Amount exceeding maximum should fail");
        let error = result.err().map(|e| e.to_string()).unwrap_or_default();
        assert!(
            error.contains("too large") || error.contains("maximum"),
            "Error should mention amount limit, got: {}",
            error
        );
    }

    #[test]
    fn test_validate_amount_maximum_allowed() {
        let mut config = create_test_config();
        // Exactly 1 billion (maximum allowed)
        config.amount_str = ngx_string("1000000000");

        let result = config.parse();
        assert!(result.is_ok(), "Maximum allowed amount should be valid");
    }

    #[test]
    fn test_validate_amount_too_many_decimals() {
        let mut config = create_test_config();
        // 7 decimal places (max is 6)
        config.amount_str = ngx_string("0.0000001");

        let result = config.parse();
        assert!(
            result.is_err(),
            "Amount with too many decimal places should fail"
        );
        let error = result.err().map(|e| e.to_string()).unwrap_or_default();
        assert!(
            error.contains("decimal places") || error.contains("precision"),
            "Error should mention decimal places, got: {}",
            error
        );
    }

    #[test]
    fn test_validate_amount_max_decimals() {
        let mut config = create_test_config();
        // Exactly 6 decimal places (maximum allowed)
        config.amount_str = ngx_string("0.123456");

        let result = config.parse();
        assert!(
            result.is_ok(),
            "Amount with maximum decimal places should be valid"
        );
    }

    #[test]
    fn test_validate_amount_invalid_format() {
        let mut config = create_test_config();
        config.amount_str = ngx_string("not-a-number");

        let result = config.parse();
        assert!(result.is_err(), "Invalid amount format should fail");
        let error = result.err().map(|e| e.to_string()).unwrap_or_default();
        assert!(
            error.contains("Invalid amount format") || error.contains("parse"),
            "Error should mention invalid format, got: {}",
            error
        );
    }

    #[test]
    fn test_validate_amount_very_small() {
        let mut config = create_test_config();
        // Smallest valid amount with 6 decimals
        config.amount_str = ngx_string("0.000001");

        let result = config.parse();
        assert!(
            result.is_ok(),
            "Very small valid amount should parse successfully"
        );
    }

    #[test]
    fn test_validate_amount_large_with_decimals() {
        let mut config = create_test_config();
        // Large amount with decimals
        config.amount_str = ngx_string("999999.999999");

        let result = config.parse();
        assert!(
            result.is_ok(),
            "Large amount with valid decimals should parse successfully"
        );
    }

    // ============================================================================
    // Integration Tests: Multiple Validation Failures
    // ============================================================================

    #[test]
    fn test_multiple_validation_failures() {
        let mut config = create_test_config();
        // Set multiple invalid values
        config.amount_str = ngx_string("-1");
        config.pay_to_str = ngx_string("invalid-address");
        config.facilitator_url_str = ngx_string("not-a-url");
        config.network_str = ngx_string("unsupported");

        let result = config.parse();
        assert!(
            result.is_err(),
            "Multiple validation failures should result in error"
        );
        // Should fail on the first validation error (amount)
        let error = result.err().map(|e| e.to_string()).unwrap_or_default();
        assert!(
            error.contains("negative") || error.contains("Amount"),
            "Should fail on first validation error, got: {}",
            error
        );
    }

    #[test]
    fn test_all_valid_config() {
        let mut config = create_test_config();
        config.enabled = 1;
        config.amount_str = ngx_string("0.0001");
        config.pay_to_str = ngx_string("0x209693Bc6afc0C5328bA36FaF03C514EF312287C");
        config.facilitator_url_str = ngx_string("https://x402.org/facilitator");
        // testnet field removed - network is determined by network_str
        config.description_str = ngx_string("Test payment");
        config.network_str = ngx_string("base-sepolia");
        config.resource_str = ngx_string("/test");

        let result = config.parse();
        assert!(
            result.is_ok(),
            "All valid configuration should parse successfully"
        );
        let parsed = result.unwrap();
        assert_eq!(parsed.enabled, true);
        assert!(parsed.amount.is_some());
        assert!(parsed.pay_to.is_some());
        assert!(parsed.facilitator_url.is_some());
        // testnet field removed - network is determined by network_str or defaults to BASE_MAINNET
        // Verify network is set correctly instead
        assert_eq!(parsed.network, Some("base-sepolia".to_string()));
        assert!(parsed.description.is_some());
        assert!(parsed.network.is_some());
        assert!(parsed.resource.is_some());
    }
}
