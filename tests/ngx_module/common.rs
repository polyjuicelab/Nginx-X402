//! Common test utilities and helpers

use rust_decimal::Decimal;
use rust_x402::types::PaymentRequirements;
use std::str::FromStr;

/// Test configuration structure that mirrors `ParsedX402Config`
/// but doesn't depend on ngx-rust types
#[allow(dead_code)]
pub struct TestConfig {
    pub enabled: bool,
    pub amount: Option<Decimal>,
    pub pay_to: Option<String>,
    pub facilitator_url: Option<String>,
    pub testnet: bool,
    pub description: Option<String>,
    pub network: Option<String>,
    pub resource: Option<String>,
}

impl TestConfig {
    pub fn new() -> Self {
        Self {
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
}

impl Default for TestConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper function that mirrors `create_requirements` logic
pub fn create_requirements_test(
    config: &TestConfig,
    resource: &str,
) -> Result<PaymentRequirements, String> {
    create_requirements_test_with_mime(config, resource, None)
}

/// Helper function that mirrors `create_requirements` logic with mimeType support
pub fn create_requirements_test_with_mime(
    config: &TestConfig,
    resource: &str,
    mime_type: Option<&str>,
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
        .ok_or_else(|| format!("Network not supported: {network}"))?;

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

    // Note: mime_type parameter is accepted but not yet used
    // This is a placeholder for future mimeType support in rust_x402
    let _ = mime_type;

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
