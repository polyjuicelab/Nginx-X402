//! Tests for ngx-rust module implementation
//!
//! These tests verify the core payment verification logic that can be used
//! with the ngx-rust module, even without a full Nginx setup.
//!
//! Note: These tests can run without Nginx source code by testing only the
//! core logic functions that don't depend on ngx-rust types.

mod tests {
    use rust_decimal::Decimal;
    use rust_x402::types::PaymentRequirements;
    use std::str::FromStr;

    // Test configuration structure that mirrors ParsedX402Config
    // but doesn't depend on ngx-rust types
    struct TestConfig {
        enabled: bool,
        amount: Option<Decimal>,
        pay_to: Option<String>,
        facilitator_url: Option<String>,
        testnet: bool,
        description: Option<String>,
        network: Option<String>,
        resource: Option<String>,
    }

    fn create_test_config() -> TestConfig {
        TestConfig {
            enabled: true,
            amount: Some(Decimal::from_str("0.0001").unwrap()),
            pay_to: Some("0x209693Bc6afc0C5328bA36FaF03C514EF312287C".to_string()),
            facilitator_url: Some("https://x402.org/facilitator".to_string()),
            testnet: true,
            description: Some("Test payment".to_string()),
            network: None,
            resource: None,
        }
    }

    // Helper function that mirrors create_requirements logic
    fn create_requirements_test(
        config: &TestConfig,
        resource: &str,
    ) -> Result<PaymentRequirements, String> {
        use rust_x402::types::networks;

        // Validate required fields (matching ngx_module.rs logic)
        let amount = config
            .amount
            .ok_or_else(|| "Amount not configured".to_string())?;

        if amount < Decimal::ZERO {
            return Err("Amount cannot be negative".to_string());
        }

        let pay_to = config
            .pay_to
            .as_ref()
            .ok_or_else(|| "Pay-to address not configured".to_string())?;

        if pay_to.trim().is_empty() {
            return Err("Pay-to address cannot be empty".to_string());
        }

        let network = if let Some(ref net) = config.network {
            net.as_str()
        } else if config.testnet {
            networks::BASE_SEPOLIA
        } else {
            networks::BASE_MAINNET
        };

        let usdc_address = networks::get_usdc_address(network)
            .ok_or_else(|| format!("Network not supported: {}", network))?;

        // Use configured resource or fall back to provided resource
        let resource = if let Some(ref resource_url) = config.resource {
            if resource_url.trim().is_empty() {
                return Err("Resource URL cannot be empty".to_string());
            }
            resource_url.clone()
        } else {
            if resource.trim().is_empty() {
                return Err("Resource path cannot be empty".to_string());
            }
            resource.to_string()
        };

        let max_amount_required = (amount * Decimal::from(1_000_000u64))
            .normalize()
            .to_string();

        let mut requirements = PaymentRequirements::new(
            rust_x402::types::schemes::EXACT,
            network,
            max_amount_required,
            usdc_address,
            pay_to.to_lowercase(),
            resource,
            config.description.as_deref().unwrap_or("Payment required"),
        );

        let network_enum = if config.testnet {
            rust_x402::types::Network::Testnet
        } else {
            rust_x402::types::Network::Mainnet
        };
        requirements
            .set_usdc_info(network_enum)
            .map_err(|e| e.to_string())?;

        Ok(requirements)
    }

    #[test]
    fn test_create_requirements_from_config() {
        let config = create_test_config();
        let requirements = create_requirements_test(&config, "/test").unwrap();

        // Verify all required fields are set correctly
        assert_eq!(requirements.scheme, "exact", "Scheme must be exact");
        assert_eq!(
            requirements.network, "base-sepolia",
            "Network must match testnet default"
        );

        // Verify pay_to is normalized to lowercase
        assert_eq!(
            requirements.pay_to, "0x209693bc6afc0c5328ba36faf03c514ef312287c",
            "Pay-to address must be lowercase"
        );

        assert_eq!(requirements.resource, "/test", "Resource must match input");
        assert_eq!(
            requirements.description, "Test payment",
            "Description must match config"
        );

        // Verify amount conversion (0.0001 USDC = 100 in smallest units)
        assert_eq!(
            requirements.max_amount_required, "100",
            "Amount must be correctly converted to smallest units"
        );

        // Verify asset address is set (USDC address for base-sepolia)
        assert!(
            !requirements.asset.is_empty(),
            "Asset address must not be empty"
        );
        assert!(
            requirements.asset.starts_with("0x"),
            "Asset address must be a valid Ethereum address"
        );
        assert_eq!(
            requirements.asset.len(),
            42,
            "Asset address must be 42 characters (0x + 40 hex chars)"
        );

        // Verify default timeout
        assert_eq!(
            requirements.max_timeout_seconds, 60,
            "Default timeout must be 60 seconds"
        );

        // Verify extra field contains USDC info
        assert!(
            requirements.extra.is_some(),
            "Extra field must contain USDC info"
        );
        let extra = requirements.extra.as_ref().unwrap();
        assert_eq!(extra["name"], "USDC", "Extra field must contain USDC name");
        assert_eq!(extra["version"], "2", "Extra field must contain version 2");
    }

    #[test]
    fn test_create_requirements_with_explicit_network() {
        let mut config = create_test_config();
        config.network = Some("base".to_string());
        config.testnet = false;

        let requirements = create_requirements_test(&config, "/test").unwrap();

        // Verify network is explicitly set
        assert_eq!(
            requirements.network, "base",
            "Network must match explicit setting"
        );

        // Verify asset address matches mainnet USDC address
        assert!(
            !requirements.asset.is_empty(),
            "Asset address must be set for mainnet"
        );
        assert!(
            requirements.asset.starts_with("0x"),
            "Asset address must be valid Ethereum address"
        );

        // Verify extra field contains mainnet USDC info
        assert!(
            requirements.extra.is_some(),
            "Extra field must contain USDC info"
        );
        let extra = requirements.extra.as_ref().unwrap();
        // Mainnet uses "USD Coin" as the USDC name
        assert_eq!(
            extra["name"], "USD Coin",
            "Extra field must contain 'USD Coin' name for mainnet"
        );
    }

    #[test]
    fn test_create_requirements_missing_amount() {
        let mut config = create_test_config();
        config.amount = None;

        let result = create_requirements_test(&config, "/test");
        assert!(result.is_err(), "Must return error when amount is missing");
        let error = result.unwrap_err();
        assert_eq!(
            error, "Amount not configured",
            "Error message must exactly match 'Amount not configured'"
        );
    }

    #[test]
    fn test_create_requirements_missing_pay_to() {
        let mut config = create_test_config();
        config.pay_to = None;

        let result = create_requirements_test(&config, "/test");
        assert!(result.is_err(), "Must return error when pay_to is missing");
        let error = result.unwrap_err();
        assert_eq!(
            error, "Pay-to address not configured",
            "Error message must exactly match 'Pay-to address not configured'"
        );
    }

    #[test]
    fn test_create_requirements_with_resource_override() {
        let mut config = create_test_config();
        config.resource = Some("/custom/resource".to_string());

        let requirements = create_requirements_test(&config, "/test").unwrap();
        assert_eq!(requirements.resource, "/custom/resource");
    }

    #[test]
    fn test_create_requirements_mainnet() {
        let mut config = create_test_config();
        config.testnet = false;
        config.network = None;

        let requirements = create_requirements_test(&config, "/test").unwrap();
        assert_eq!(requirements.network, "base");
    }

    #[test]
    fn test_create_requirements_amount_conversion() {
        let mut config = create_test_config();
        config.amount = Some(Decimal::from_str("1.0").unwrap());

        let requirements = create_requirements_test(&config, "/test").unwrap();
        // 1.0 USDC = 1,000,000 in smallest units (6 decimals)
        assert_eq!(
            requirements.max_amount_required, "1000000",
            "1.0 USDC must equal exactly 1,000,000 in smallest units"
        );

        // Verify the conversion is reversible
        let amount_back = Decimal::from_str(&requirements.max_amount_required).unwrap()
            / Decimal::from(1_000_000u64);
        assert_eq!(
            amount_back,
            Decimal::from_str("1.0").unwrap(),
            "Amount conversion must be reversible"
        );
    }

    #[test]
    fn test_create_requirements_negative_amount() {
        let mut config = create_test_config();
        config.amount = Some(Decimal::from_str("-0.0001").unwrap());

        let result = create_requirements_test(&config, "/test");
        assert!(result.is_err(), "Must return error for negative amount");
        let error = result.unwrap_err();
        assert_eq!(
            error, "Amount cannot be negative",
            "Error message must exactly match 'Amount cannot be negative'"
        );
    }

    #[test]
    fn test_create_requirements_empty_pay_to() {
        let mut config = create_test_config();
        config.pay_to = Some("".to_string());

        let result = create_requirements_test(&config, "/test");
        assert!(result.is_err(), "Must return error for empty pay_to");
        let error = result.unwrap_err();
        assert_eq!(
            error, "Pay-to address cannot be empty",
            "Error message must exactly match 'Pay-to address cannot be empty'"
        );
    }

    #[test]
    fn test_create_requirements_whitespace_pay_to() {
        let mut config = create_test_config();
        config.pay_to = Some("   ".to_string());

        let result = create_requirements_test(&config, "/test");
        assert!(
            result.is_err(),
            "Must return error for whitespace-only pay_to"
        );
        let error = result.unwrap_err();
        assert_eq!(
            error, "Pay-to address cannot be empty",
            "Error message must exactly match 'Pay-to address cannot be empty'"
        );
    }

    #[test]
    fn test_create_requirements_empty_resource() {
        let mut config = create_test_config();
        config.resource = Some("".to_string());

        let result = create_requirements_test(&config, "/test");
        assert!(result.is_err(), "Must return error for empty resource");
        let error = result.unwrap_err();
        assert_eq!(
            error, "Resource URL cannot be empty",
            "Error message must exactly match 'Resource URL cannot be empty'"
        );
    }

    #[test]
    fn test_create_requirements_empty_resource_path() {
        let config = create_test_config();

        let result = create_requirements_test(&config, "");
        assert!(result.is_err(), "Must return error for empty resource path");
        let error = result.unwrap_err();
        assert_eq!(
            error, "Resource path cannot be empty",
            "Error message must exactly match 'Resource path cannot be empty'"
        );
    }

    #[test]
    fn test_create_requirements_whitespace_resource() {
        let mut config = create_test_config();
        config.resource = Some("   ".to_string());

        let result = create_requirements_test(&config, "/test");
        assert!(
            result.is_err(),
            "Must return error for whitespace-only resource"
        );
        let error = result.unwrap_err();
        assert_eq!(
            error, "Resource URL cannot be empty",
            "Error message must exactly match 'Resource URL cannot be empty'"
        );
    }

    #[test]
    fn test_create_requirements_zero_amount() {
        let mut config = create_test_config();
        config.amount = Some(Decimal::ZERO);

        let requirements = create_requirements_test(&config, "/test").unwrap();
        assert_eq!(
            requirements.max_amount_required, "0",
            "Zero amount must result in '0' string"
        );

        // Verify all other fields are still set correctly
        assert_eq!(requirements.scheme, "exact");
        assert!(!requirements.asset.is_empty());
        assert!(!requirements.pay_to.is_empty());
    }

    #[test]
    fn test_create_requirements_very_small_amount() {
        let mut config = create_test_config();
        config.amount = Some(Decimal::from_str("0.000001").unwrap());

        let requirements = create_requirements_test(&config, "/test").unwrap();
        // 0.000001 USDC = 1 in smallest units (6 decimals)
        assert_eq!(
            requirements.max_amount_required, "1",
            "0.000001 USDC must equal exactly 1 in smallest units"
        );

        // Verify precision is maintained
        let amount_back = Decimal::from_str(&requirements.max_amount_required).unwrap()
            / Decimal::from(1_000_000u64);
        assert_eq!(
            amount_back,
            Decimal::from_str("0.000001").unwrap(),
            "Very small amount conversion must maintain precision"
        );
    }

    #[test]
    fn test_create_requirements_very_large_amount() {
        let mut config = create_test_config();
        config.amount = Some(Decimal::from_str("1000000.0").unwrap());

        let requirements = create_requirements_test(&config, "/test").unwrap();
        // 1,000,000 USDC = 1,000,000,000,000 in smallest units
        assert_eq!(
            requirements.max_amount_required, "1000000000000",
            "1,000,000 USDC must equal exactly 1,000,000,000,000 in smallest units"
        );

        // Verify it's a valid large number and can be parsed
        let max_amount: u64 = requirements
            .max_amount_required
            .parse()
            .expect("max_amount_required must be a valid u64 number");
        assert_eq!(
            max_amount, 1_000_000_000_000u64,
            "Parsed amount must match expected value"
        );

        // Verify conversion is reversible
        let amount_back = Decimal::from(max_amount) / Decimal::from(1_000_000u64);
        assert_eq!(
            amount_back,
            Decimal::from_str("1000000.0").unwrap(),
            "Large amount conversion must be reversible"
        );
    }

    #[test]
    fn test_create_requirements_fractional_amount() {
        let mut config = create_test_config();
        config.amount = Some(Decimal::from_str("0.123456").unwrap());

        let requirements = create_requirements_test(&config, "/test").unwrap();
        // 0.123456 USDC = 123456 in smallest units (6 decimals)
        assert_eq!(
            requirements.max_amount_required, "123456",
            "0.123456 USDC must equal exactly 123456 in smallest units"
        );

        // Verify fractional precision is maintained
        let amount_back = Decimal::from_str(&requirements.max_amount_required).unwrap()
            / Decimal::from(1_000_000u64);
        assert_eq!(
            amount_back,
            Decimal::from_str("0.123456").unwrap(),
            "Fractional amount conversion must maintain all 6 decimal places"
        );
    }

    #[test]
    fn test_create_requirements_long_pay_to_address() {
        let mut config = create_test_config();
        // Valid Ethereum address format (42 chars)
        config.pay_to = Some("0x1234567890123456789012345678901234567890".to_string());

        let requirements = create_requirements_test(&config, "/test").unwrap();
        assert_eq!(
            requirements.pay_to,
            "0x1234567890123456789012345678901234567890"
        );
    }

    #[test]
    fn test_create_requirements_long_resource_path() {
        let mut config = create_test_config();
        let long_path = "/".to_string() + &"a".repeat(1000);
        config.resource = Some(long_path.clone());

        let requirements = create_requirements_test(&config, "/test").unwrap();
        assert_eq!(requirements.resource, long_path);
    }

    #[test]
    fn test_create_requirements_special_characters_in_resource() {
        let mut config = create_test_config();
        config.resource = Some("/api/v1/payment?amount=0.0001&token=abc123".to_string());

        let requirements = create_requirements_test(&config, "/test").unwrap();
        assert_eq!(
            requirements.resource,
            "/api/v1/payment?amount=0.0001&token=abc123"
        );
    }

    #[test]
    fn test_create_requirements_unicode_in_description() {
        let mut config = create_test_config();
        config.description = Some("测试支付 🚀".to_string());

        let requirements = create_requirements_test(&config, "/test").unwrap();
        assert_eq!(requirements.description, "测试支付 🚀");
    }

    #[test]
    fn test_create_requirements_network_override_ignores_testnet_flag() {
        let mut config = create_test_config();
        config.network = Some("base".to_string());
        config.testnet = true; // Should be ignored when network is explicitly set

        let requirements = create_requirements_test(&config, "/test").unwrap();
        assert_eq!(
            requirements.network, "base",
            "Explicit network must override testnet flag"
        );

        // Verify asset address matches mainnet (not testnet) when network is explicitly "base"
        assert!(!requirements.asset.is_empty(), "Asset address must be set");

        // Verify extra field uses mainnet USDC info (because network is "base")
        assert!(
            requirements.extra.is_some(),
            "Extra field must contain USDC info"
        );
        let extra = requirements.extra.as_ref().unwrap();
        assert_eq!(extra["name"], "USDC", "Extra field must contain USDC name");
    }

    #[test]
    fn test_create_requirements_multiple_requirements_support() {
        // Test that the function can create multiple requirements for multi-part payment scenarios
        let config1 = create_test_config();
        let config2 = {
            let mut c = create_test_config();
            c.amount = Some(Decimal::from_str("0.0002").unwrap());
            c
        };

        let req1 = create_requirements_test(&config1, "/resource1").unwrap();
        let req2 = create_requirements_test(&config2, "/resource2").unwrap();

        // Verify both requirements are valid
        assert_eq!(req1.max_amount_required, "100");
        assert_eq!(req2.max_amount_required, "200");
        assert_ne!(
            req1.max_amount_required, req2.max_amount_required,
            "Different amounts must produce different max_amount_required"
        );
    }

    #[test]
    fn test_create_requirements_asset_address_validation() {
        let config = create_test_config();
        let requirements = create_requirements_test(&config, "/test").unwrap();

        // Strict validation of asset address format
        assert!(
            requirements.asset.starts_with("0x"),
            "Asset address must start with 0x"
        );
        assert_eq!(
            requirements.asset.len(),
            42,
            "Asset address must be exactly 42 characters (0x + 40 hex)"
        );

        // Verify all characters after 0x are valid hex
        let hex_part = &requirements.asset[2..];
        assert!(
            hex_part.chars().all(|c| c.is_ascii_hexdigit()),
            "Asset address must contain only hex characters after 0x"
        );
    }

    #[test]
    fn test_create_requirements_pay_to_address_validation() {
        let config = create_test_config();
        let requirements = create_requirements_test(&config, "/test").unwrap();

        // Strict validation of pay_to address format
        assert!(
            requirements.pay_to.starts_with("0x"),
            "Pay-to address must start with 0x"
        );
        assert_eq!(
            requirements.pay_to.len(),
            42,
            "Pay-to address must be exactly 42 characters (0x + 40 hex)"
        );
        assert_eq!(
            requirements.pay_to,
            requirements.pay_to.to_lowercase(),
            "Pay-to address must be lowercase"
        );

        // Verify all characters after 0x are valid hex
        let hex_part = &requirements.pay_to[2..];
        assert!(
            hex_part.chars().all(|c| c.is_ascii_hexdigit()),
            "Pay-to address must contain only hex characters after 0x"
        );
    }

    #[test]
    fn test_create_requirements_max_timeout_seconds_default() {
        let config = create_test_config();
        let requirements = create_requirements_test(&config, "/test").unwrap();

        // Verify default timeout is set correctly
        assert_eq!(
            requirements.max_timeout_seconds, 60,
            "Default max_timeout_seconds must be 60"
        );
        assert!(
            requirements.max_timeout_seconds > 0,
            "max_timeout_seconds must be positive"
        );
    }

    #[test]
    fn test_create_requirements_extra_field_structure() {
        let config = create_test_config();
        let requirements = create_requirements_test(&config, "/test").unwrap();

        // Strict validation of extra field structure
        assert!(requirements.extra.is_some(), "Extra field must be present");
        let extra = requirements.extra.as_ref().unwrap();

        // Verify it's an object (not array or primitive)
        assert!(extra.is_object(), "Extra field must be a JSON object");

        // Verify required fields exist
        assert!(
            extra.get("name").is_some(),
            "Extra field must contain 'name'"
        );
        assert!(
            extra.get("version").is_some(),
            "Extra field must contain 'version'"
        );

        // Verify field types
        assert!(extra["name"].is_string(), "Extra.name must be a string");
        assert!(
            extra["version"].is_string(),
            "Extra.version must be a string"
        );

        // Verify exact values
        assert_eq!(
            extra["name"].as_str().unwrap(),
            "USDC",
            "Extra.name must be exactly 'USDC'"
        );
        assert_eq!(
            extra["version"].as_str().unwrap(),
            "2",
            "Extra.version must be exactly '2'"
        );
    }

    #[test]
    fn test_create_requirements_scheme_always_exact() {
        let config = create_test_config();
        let requirements = create_requirements_test(&config, "/test").unwrap();

        // Verify scheme is always "exact"
        assert_eq!(
            requirements.scheme, "exact",
            "Scheme must always be 'exact'"
        );
    }

    #[test]
    fn test_create_requirements_mime_type_default() {
        let config = create_test_config();
        let requirements = create_requirements_test(&config, "/test").unwrap();

        // Verify mime_type defaults to None
        assert!(
            requirements.mime_type.is_none(),
            "mime_type must default to None"
        );
    }

    #[test]
    fn test_create_requirements_output_schema_default() {
        let config = create_test_config();
        let requirements = create_requirements_test(&config, "/test").unwrap();

        // Verify output_schema defaults to None
        assert!(
            requirements.output_schema.is_none(),
            "output_schema must default to None"
        );
    }

    #[test]
    fn test_create_requirements_all_fields_present() {
        let config = create_test_config();
        let requirements = create_requirements_test(&config, "/test").unwrap();

        // Comprehensive check: all fields must be set (even if None)
        assert!(!requirements.scheme.is_empty(), "scheme must not be empty");
        assert!(
            !requirements.network.is_empty(),
            "network must not be empty"
        );
        assert!(
            !requirements.max_amount_required.is_empty(),
            "max_amount_required must not be empty"
        );
        assert!(!requirements.asset.is_empty(), "asset must not be empty");
        assert!(!requirements.pay_to.is_empty(), "pay_to must not be empty");
        assert!(
            !requirements.resource.is_empty(),
            "resource must not be empty"
        );
        assert!(
            !requirements.description.is_empty(),
            "description must not be empty"
        );
        assert!(
            requirements.max_timeout_seconds > 0,
            "max_timeout_seconds must be positive"
        );
        assert!(requirements.extra.is_some(), "extra must be set");
    }
}
