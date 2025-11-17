//! Configuration types for Nginx x402 module

use rust_x402::types::PaymentRequirements;
use rust_x402::{Result, X402Error};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::ffi::CStr;
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

        let usdc_address = rust_x402::types::networks::get_usdc_address(network).ok_or_else(|| {
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

    /// Create from C strings (for FFI)
    ///
    /// # Safety
    ///
    /// This function is unsafe because it dereferences raw pointers.
    /// The caller must ensure that all pointers are valid and point to
    /// null-terminated C strings.
    pub unsafe fn from_c_strings(
        amount: *const std::ffi::c_char,
        pay_to: *const std::ffi::c_char,
        facilitator_url: *const std::ffi::c_char,
        testnet: bool,
    ) -> Result<Self> {
        let amount_str = CStr::from_ptr(amount)
            .to_str()
            .map_err(|_| X402Error::config("Invalid amount string"))?;
        let amount_decimal = Decimal::from_str(amount_str)
            .map_err(|e| X402Error::config(format!("Invalid amount: {}", e)))?;

        let pay_to_str = CStr::from_ptr(pay_to)
            .to_str()
            .map_err(|_| X402Error::config("Invalid pay_to string"))?;

        let facilitator_url_str = if facilitator_url.is_null() {
            "https://x402.org/facilitator".to_string()
        } else {
            CStr::from_ptr(facilitator_url)
                .to_str()
                .map_err(|_| X402Error::config("Invalid facilitator_url string"))?
                .to_string()
        };

        Ok(Self {
            amount: amount_decimal,
            pay_to: pay_to_str.to_string(),
            facilitator_url: facilitator_url_str,
            testnet,
            ..Default::default()
        })
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
        assert_eq!(config.amount, Decimal::from_str("0.0001").unwrap());
        assert_eq!(config.pay_to, "0x209693Bc6afc0C5328bA36FaF03C514EF312287C");
    }

    #[test]
    fn test_create_payment_requirements() {
        let config = NginxX402Config::new(
            Decimal::from_str("0.0001").unwrap(),
            "0x209693Bc6afc0C5328bA36FaF03C514EF312287C",
        );
        let requirements = config.create_payment_requirements("/test").unwrap();
        assert_eq!(requirements.scheme, "exact");
        assert_eq!(requirements.network, "base-sepolia");
    }
}

