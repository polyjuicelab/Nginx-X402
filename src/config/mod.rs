//! Configuration types for Nginx x402 module

mod validation;

pub use validation::{
    parse_accept_priority, validate_amount, validate_ethereum_address, validate_network,
    validate_payment_header, validate_resource_path, validate_url,
};

use rust_decimal::Decimal;
use rust_x402::types::PaymentRequirements;
use rust_x402::{Result, X402Error};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// Configuration for Nginx x402 module
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NginxX402Config {
    /// Payment amount in decimal units
    pub amount: Decimal,
    /// Recipient wallet address
    pub pay_to: String,
    /// Payment description
    pub description: Option<String>,
    /// Facilitator URL
    pub facilitator_url: String,
    /// Whether this is a testnet
    pub testnet: bool,
    /// Network identifier (e.g., "base-sepolia")
    pub network: Option<String>,
    /// Resource URL (if different from request)
    pub resource: Option<String>,
}

impl Default for NginxX402Config {
    fn default() -> Self {
        Self {
            amount: Decimal::from_str("0.0001").unwrap(),
            pay_to: String::new(),
            description: None,
            facilitator_url: "https://x402.org/facilitator".to_string(),
            testnet: true,
            network: None,
            resource: None,
        }
    }
}

impl NginxX402Config {
    /// Create a new configuration
    pub fn new(amount: Decimal, pay_to: impl Into<String>) -> Self {
        Self {
            amount,
            pay_to: pay_to.into(),
            ..Default::default()
        }
    }

    /// Create payment requirements from this config
    pub fn create_payment_requirements(&self, request_uri: &str) -> Result<PaymentRequirements> {
        let network = if let Some(ref net) = self.network {
            net.as_str()
        } else if self.testnet {
            rust_x402::types::networks::BASE_SEPOLIA
        } else {
            rust_x402::types::networks::BASE_MAINNET
        };

        let usdc_address =
            rust_x402::types::networks::get_usdc_address(network).ok_or_else(|| {
                X402Error::NetworkNotSupported {
                    network: network.to_string(),
                }
            })?;

        let resource = if let Some(ref resource_url) = self.resource {
            resource_url.clone()
        } else {
            request_uri.to_string()
        };

        let max_amount_required = (self.amount * Decimal::from(1_000_000u64))
            .normalize()
            .to_string();

        let mut requirements = PaymentRequirements::new(
            rust_x402::types::schemes::EXACT,
            network,
            max_amount_required,
            usdc_address,
            self.pay_to.to_lowercase(),
            resource,
            self.description.as_deref().unwrap_or("Payment required"),
        );

        let network_enum = if self.testnet {
            rust_x402::types::Network::Testnet
        } else {
            rust_x402::types::Network::Mainnet
        };
        requirements.set_usdc_info(network_enum)?;

        Ok(requirements)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_creation() {
        let config = NginxX402Config::new(
            Decimal::from_str("0.0001").unwrap(),
            "0x209693Bc6afc0C5328bA36FaF03C514EF312287C",
        );
        assert_eq!(
            config.amount,
            Decimal::from_str("0.0001").unwrap(),
            "Amount should match input"
        );
        assert_eq!(
            config.pay_to, "0x209693Bc6afc0C5328bA36FaF03C514EF312287C",
            "Pay-to address should match input"
        );
        assert!(config.testnet, "Should default to testnet");
    }

    #[test]
    fn test_config_default() {
        let config = NginxX402Config::default();
        assert_eq!(
            config.amount,
            Decimal::from_str("0.0001").unwrap(),
            "Default amount should be 0.0001"
        );
        assert!(config.pay_to.is_empty(), "Default pay_to should be empty");
        assert!(config.testnet, "Default should be testnet");
        assert_eq!(
            config.facilitator_url, "https://x402.org/facilitator",
            "Default facilitator URL should be set"
        );
    }

    #[test]
    fn test_create_payment_requirements() {
        let config = NginxX402Config::new(
            Decimal::from_str("0.0001").unwrap(),
            "0x209693Bc6afc0C5328bA36FaF03C514EF312287C",
        );
        let requirements = config
            .create_payment_requirements("/test")
            .expect("Should create requirements successfully");
        assert_eq!(requirements.scheme, "exact", "Scheme should be exact");
        assert_eq!(
            requirements.network, "base-sepolia",
            "Network should be base-sepolia for testnet"
        );
        assert_eq!(
            requirements.resource, "/test",
            "Resource should match input"
        );
        assert!(
            !requirements.pay_to.is_empty(),
            "Pay-to address should not be empty"
        );
    }

    #[test]
    fn test_create_payment_requirements_with_custom_resource() {
        let mut config = NginxX402Config::new(
            Decimal::from_str("0.0001").unwrap(),
            "0x209693Bc6afc0C5328bA36FaF03C514EF312287C",
        );
        config.resource = Some("/custom/resource".to_string());
        let requirements = config
            .create_payment_requirements("/requested")
            .expect("Should create requirements successfully");
        assert_eq!(
            requirements.resource, "/custom/resource",
            "Should use custom resource instead of request URI"
        );
    }

    #[test]
    fn test_create_payment_requirements_with_description() {
        let mut config = NginxX402Config::new(
            Decimal::from_str("0.0001").unwrap(),
            "0x209693Bc6afc0C5328bA36FaF03C514EF312287C",
        );
        config.description = Some("Custom payment description".to_string());
        let requirements = config
            .create_payment_requirements("/test")
            .expect("Should create requirements successfully");
        assert_eq!(
            requirements.description, "Custom payment description",
            "Description should match input"
        );
    }

    #[test]
    fn test_create_payment_requirements_without_description() {
        let config = NginxX402Config::new(
            Decimal::from_str("0.0001").unwrap(),
            "0x209693Bc6afc0C5328bA36FaF03C514EF312287C",
        );
        let requirements = config
            .create_payment_requirements("/test")
            .expect("Should create requirements successfully");
        assert_eq!(
            requirements.description, "Payment required",
            "Should use default description"
        );
    }

    #[test]
    fn test_create_payment_requirements_mainnet() {
        let mut config = NginxX402Config::new(
            Decimal::from_str("0.0001").unwrap(),
            "0x209693Bc6afc0C5328bA36FaF03C514EF312287C",
        );
        config.testnet = false;
        let requirements = config
            .create_payment_requirements("/test")
            .expect("Should create requirements successfully");
        assert_eq!(
            requirements.network, "base",
            "Network should be base for mainnet"
        );
    }

    #[test]
    fn test_create_payment_requirements_with_explicit_network() {
        let mut config = NginxX402Config::new(
            Decimal::from_str("0.0001").unwrap(),
            "0x209693Bc6afc0C5328bA36FaF03C514EF312287C",
        );
        config.network = Some("base-sepolia".to_string());
        let requirements = config
            .create_payment_requirements("/test")
            .expect("Should create requirements successfully");
        assert_eq!(
            requirements.network, "base-sepolia",
            "Network should match explicit setting"
        );
    }

    #[test]
    fn test_create_payment_requirements_different_amounts() {
        let amounts = vec!["0.0001", "0.001", "0.01", "1.0", "100.0"];

        for amount_str in amounts {
            let amount = Decimal::from_str(amount_str)
                .unwrap_or_else(|_| panic!("Should parse amount: {}", amount_str));
            let config = NginxX402Config::new(amount, "0x209693Bc6afc0C5328bA36FaF03C514EF312287C");
            let requirements = config
                .create_payment_requirements("/test")
                .unwrap_or_else(|_| {
                    panic!("Should create requirements for amount: {}", amount_str)
                });
            assert!(
                !requirements.max_amount_required.is_empty(),
                "max_amount_required should not be empty for amount: {}",
                amount_str
            );
        }
    }

    #[test]
    fn test_create_payment_requirements_pay_to_lowercase() {
        let config = NginxX402Config::new(
            Decimal::from_str("0.0001").unwrap(),
            "0x209693Bc6afc0C5328bA36FaF03C514EF312287C", // Mixed case
        );
        let requirements = config
            .create_payment_requirements("/test")
            .expect("Should create requirements successfully");
        assert_eq!(
            requirements.pay_to, "0x209693bc6afc0c5328ba36faf03c514ef312287c",
            "Pay-to address should be lowercase"
        );
    }

    #[test]
    fn test_create_payment_requirements_unsupported_network() {
        let mut config = NginxX402Config::new(
            Decimal::from_str("0.0001").unwrap(),
            "0x209693Bc6afc0C5328bA36FaF03C514EF312287C",
        );
        config.network = Some("unsupported-network".to_string());
        let result = config.create_payment_requirements("/test");
        assert!(
            result.is_err(),
            "Should return error for unsupported network"
        );
        match result {
            Err(X402Error::NetworkNotSupported { network }) => {
                assert_eq!(
                    network, "unsupported-network",
                    "Error should contain network name"
                );
            }
            _ => panic!("Should return NetworkNotSupported error"),
        }
    }

    #[test]
    fn test_create_payment_requirements_empty_pay_to() {
        let config = NginxX402Config::new(
            Decimal::from_str("0.0001").unwrap(),
            "", // Empty pay_to
        );
        let requirements = config
            .create_payment_requirements("/test")
            .expect("Should create requirements even with empty pay_to");
        assert_eq!(requirements.pay_to, "", "Pay-to should be empty string");
    }

    #[test]
    fn test_create_payment_requirements_zero_amount() {
        let config = NginxX402Config::new(
            Decimal::from_str("0").unwrap(),
            "0x209693Bc6afc0C5328bA36FaF03C514EF312287C",
        );
        let requirements = config
            .create_payment_requirements("/test")
            .expect("Should create requirements with zero amount");
        assert_eq!(
            requirements.max_amount_required, "0",
            "max_amount_required should be 0"
        );
    }

    #[test]
    fn test_create_payment_requirements_very_large_amount() {
        let config = NginxX402Config::new(
            Decimal::from_str("1000000.0").unwrap(),
            "0x209693Bc6afc0C5328bA36FaF03C514EF312287C",
        );
        let requirements = config
            .create_payment_requirements("/test")
            .expect("Should create requirements with large amount");
        assert!(
            !requirements.max_amount_required.is_empty(),
            "max_amount_required should not be empty"
        );
        // Verify it's a valid number
        let max_amount: u64 = requirements
            .max_amount_required
            .parse()
            .expect("max_amount_required should be a valid number");
        assert!(
            max_amount > 0,
            "max_amount_required should be greater than 0"
        );
    }
}
