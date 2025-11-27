//! Unit tests for validation functions
//!
//! These tests verify validation functions directly without requiring ngx-rust.
//! This allows testing the validation logic without needing Nginx source code.

mod tests {
    use rust_decimal::Decimal;
    use rust_x402::types::networks;
    use std::str::FromStr;

    use nginx_x402::{validate_amount, validate_ethereum_address, validate_network, validate_url};

    // ============================================================================
    // fix-6: Configuration Validation Tests (Direct Function Tests)
    // ============================================================================

    #[test]
    fn test_validate_ethereum_address_valid_cases() {
        let valid_addresses = vec![
            "0x209693Bc6afc0C5328bA36FaF03C514EF312287C",
            "0x209693bc6afc0c5328ba36faf03c514ef312287c",
            "0X209693Bc6afc0C5328bA36FaF03C514EF312287C",
            "0x0000000000000000000000000000000000000000",
            "0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF",
        ];

        for address in valid_addresses {
            let result = validate_ethereum_address(address);
            assert!(
                result.is_ok(),
                "Valid address '{address}' should pass validation"
            );
        }
    }

    #[test]
    fn test_validate_ethereum_address_invalid_cases() {
        let invalid_cases = vec![
            ("", "Ethereum address cannot be empty"),
            ("0x123", "Invalid Ethereum address length"),
            // Address without 0x prefix will fail length check first (40 chars vs 42)
            (
                "209693Bc6afc0C5328bA36FaF03C514EF312287C",
                "Invalid Ethereum address length",
            ),
            (
                "0x209693Bc6afc0C5328bA36FaF03C514EF312287g",
                "invalid characters",
            ),
            (
                "0x209693Bc6afc0C5328bA36FaF03C514EF312287",
                "Invalid Ethereum address length",
            ),
            (
                "0x209693Bc6afc0C5328bA36FaF03C514EF312287CC",
                "Invalid Ethereum address length",
            ),
        ];

        for (address, expected_error) in invalid_cases {
            let result = validate_ethereum_address(address);
            assert!(
                result.is_err(),
                "Invalid address '{address}' should fail validation"
            );
            let error = result.unwrap_err().to_string();
            assert!(
                error.contains(expected_error),
                "Error for '{address}' should contain '{expected_error}', got: {error}"
            );
        }
    }

    #[test]
    fn test_validate_url_valid_cases() {
        let valid_urls = vec![
            "http://example.com",
            "https://x402.org/facilitator",
            "http://localhost:8080/api",
            "https://subdomain.example.com/path/to/resource",
        ];

        for url in valid_urls {
            let result = validate_url(url);
            assert!(result.is_ok(), "Valid URL '{url}' should pass validation");
        }
    }

    #[test]
    fn test_validate_url_invalid_cases() {
        let invalid_cases = vec![
            ("", "URL cannot be empty"),
            ("example.com", "must start with http:// or https://"),
            ("ftp://example.com", "must start with http:// or https://"),
            ("http://", "too short"),
            ("https://example.com/with space", "whitespace"),
            ("https://example.com/with\nnewline", "whitespace"),
            ("https://example.com/with\ttab", "whitespace"),
        ];

        for (url, expected_error) in invalid_cases {
            let result = validate_url(url);
            assert!(
                result.is_err(),
                "Invalid URL '{url}' should fail validation"
            );
            let error = result.unwrap_err().to_string();
            assert!(
                error.contains(expected_error),
                "Error for '{url}' should contain '{expected_error}', got: {error}"
            );
        }
    }

    #[test]
    fn test_validate_network_valid_cases() {
        let supported = networks::all_supported();
        for network in supported {
            let result = validate_network(network);
            assert!(
                result.is_ok(),
                "Supported network '{network}' should pass validation"
            );
        }
    }

    #[test]
    fn test_validate_network_invalid_cases() {
        let invalid_cases = vec![
            ("", "Network name cannot be empty"),
            ("unsupported-network", "Unsupported network"),
            ("ethereum", "Unsupported network"),
            ("polygon", "Unsupported network"),
        ];

        for (network, expected_error) in invalid_cases {
            let result = validate_network(network);
            assert!(
                result.is_err(),
                "Unsupported network '{network}' should fail validation"
            );
            let error = result.unwrap_err().to_string();
            assert!(
                error.contains(expected_error),
                "Error for '{network}' should contain '{expected_error}', got: {error}"
            );
        }
    }

    // ============================================================================
    // fix-8: Amount Overflow Protection Tests (Direct Function Tests)
    // ============================================================================

    #[test]
    fn test_validate_amount_valid_cases() {
        let valid_amounts = vec![
            Decimal::ZERO,
            Decimal::from_str("0.000001").unwrap(),
            Decimal::from_str("0.0001").unwrap(),
            Decimal::from_str("0.123456").unwrap(),
            Decimal::from_str("1.0").unwrap(),
            Decimal::from_str("100.0").unwrap(),
            Decimal::from_str("999999.999999").unwrap(),
            Decimal::from_str("1000000000").unwrap(), // Maximum allowed
        ];

        for amount in valid_amounts {
            let result = validate_amount(amount);
            assert!(
                result.is_ok(),
                "Valid amount '{amount}' should pass validation"
            );
        }
    }

    #[test]
    fn test_validate_amount_negative() {
        let negative_amounts = vec![
            Decimal::from_str("-0.0001").unwrap(),
            Decimal::from_str("-1").unwrap(),
            Decimal::from_str("-1000000").unwrap(),
        ];

        for amount in negative_amounts {
            let result = validate_amount(amount);
            assert!(
                result.is_err(),
                "Negative amount '{amount}' should fail validation"
            );
            let error = result.unwrap_err().to_string();
            assert!(
                error.contains("negative") || error.contains("cannot be negative"),
                "Error should mention negative, got: {error}"
            );
        }
    }

    #[test]
    fn test_validate_amount_too_large() {
        let too_large = vec![
            Decimal::from_str("1000000001").unwrap(), // 1 billion + 1
            Decimal::from_str("2000000000").unwrap(), // 2 billion
            Decimal::from_str("9999999999").unwrap(), // 10 billion
        ];

        for amount in too_large {
            let result = validate_amount(amount);
            assert!(
                result.is_err(),
                "Amount '{amount}' exceeding maximum should fail validation"
            );
            let error = result.unwrap_err().to_string();
            assert!(
                error.contains("too large") || error.contains("maximum"),
                "Error should mention amount limit, got: {error}"
            );
        }
    }

    #[test]
    fn test_validate_amount_too_many_decimals() {
        // Create amounts with more than 6 decimal places
        let too_many_decimals = vec![
            Decimal::from_str("0.0000001").unwrap(),  // 7 decimals
            Decimal::from_str("0.00000001").unwrap(), // 8 decimals
            Decimal::from_str("1.1234567").unwrap(),  // 7 decimals
        ];

        for amount in too_many_decimals {
            let result = validate_amount(amount);
            assert!(
                result.is_err(),
                "Amount '{amount}' with too many decimals should fail validation"
            );
            let error = result.unwrap_err().to_string();
            assert!(
                error.contains("decimal places") || error.contains("precision"),
                "Error should mention decimal places, got: {error}"
            );
        }
    }

    #[test]
    fn test_validate_amount_max_decimals() {
        // Test that exactly 6 decimals is allowed
        let max_decimals = Decimal::from_str("0.123456").unwrap();
        let result = validate_amount(max_decimals);
        assert!(
            result.is_ok(),
            "Amount with exactly 6 decimal places should be valid"
        );
    }

    #[test]
    fn test_validate_amount_boundary_values() {
        // Test boundary values
        let max_allowed = Decimal::from_str("1000000000").unwrap();
        let result = validate_amount(max_allowed);
        assert!(result.is_ok(), "Maximum allowed amount should be valid");

        let just_over_max = Decimal::from_str("1000000000.000001").unwrap();
        let result = validate_amount(just_over_max);
        assert!(result.is_err(), "Amount just over maximum should fail");
    }
}
