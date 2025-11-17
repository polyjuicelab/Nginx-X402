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

        let amount = config
            .amount
            .ok_or_else(|| "Amount not configured".to_string())?;
        let pay_to = config
            .pay_to
            .as_ref()
            .ok_or_else(|| "Pay-to address not configured".to_string())?;

        let network = if let Some(ref net) = config.network {
            net.as_str()
        } else if config.testnet {
            networks::BASE_SEPOLIA
        } else {
            networks::BASE_MAINNET
        };

        let usdc_address = networks::get_usdc_address(network)
            .ok_or_else(|| format!("Network not supported: {}", network))?;

        let resource = if let Some(ref resource_url) = config.resource {
            resource_url.clone()
        } else {
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

        assert_eq!(requirements.scheme, "exact");
        assert_eq!(requirements.network, "base-sepolia");
        // pay_to is normalized to lowercase in create_requirements
        assert_eq!(
            requirements.pay_to,
            "0x209693bc6afc0c5328ba36faf03c514ef312287c"
        );
        assert_eq!(requirements.resource, "/test");
        assert_eq!(requirements.description, "Test payment");
    }

    #[test]
    fn test_create_requirements_with_explicit_network() {
        let mut config = create_test_config();
        config.network = Some("base".to_string());
        config.testnet = false;

        let requirements = create_requirements_test(&config, "/test").unwrap();
        assert_eq!(requirements.network, "base");
    }

    #[test]
    fn test_create_requirements_missing_amount() {
        let mut config = create_test_config();
        config.amount = None;

        let result = create_requirements_test(&config, "/test");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Amount not configured"));
    }

    #[test]
    fn test_create_requirements_missing_pay_to() {
        let mut config = create_test_config();
        config.pay_to = None;

        let result = create_requirements_test(&config, "/test");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("Pay-to address not configured"));
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
        // 1.0 USDC = 1,000,000 in smallest units
        assert_eq!(requirements.max_amount_required, "1000000");
    }
}
