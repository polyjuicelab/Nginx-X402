//! Configuration types for Nginx x402 module

use rust_decimal::Decimal;
use rust_x402::types::PaymentRequirements;
use rust_x402::{Result, X402Error};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// Validation functions for configuration values
///
/// These functions are used to validate configuration values before they are used.
/// They are public to allow testing.
pub mod validation {
    use super::*;

    /// Validate Ethereum address format
    ///
    /// # Arguments
    /// - `address`: Address string to validate
    ///
    /// # Returns
    /// - `Ok(())` if address is valid
    /// - `Err` if address format is invalid
    pub fn validate_ethereum_address(address: &str) -> Result<()> {
        if address.is_empty() {
            return Err(X402Error::config("Ethereum address cannot be empty"));
        }

        // Check length: 0x + 40 hex characters = 42 characters
        if address.len() != 42 {
            return Err(X402Error::config(format!(
                "Invalid Ethereum address length: expected 42 characters, got {}",
                address.len()
            )));
        }

        // Check prefix
        if !address.starts_with("0x") && !address.starts_with("0X") {
            return Err(X402Error::config("Ethereum address must start with 0x"));
        }

        // Check hex characters
        let hex_part = &address[2..];
        if !hex_part.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(X402Error::config(
                "Ethereum address contains invalid characters (must be hex)",
            ));
        }

        Ok(())
    }

    /// Validate URL format
    ///
    /// # Arguments
    /// - `url`: URL string to validate
    ///
    /// # Returns
    /// - `Ok(())` if URL is valid
    /// - `Err` if URL format is invalid
    pub fn validate_url(url: &str) -> Result<()> {
        if url.is_empty() {
            return Err(X402Error::config("URL cannot be empty"));
        }

        // Check for valid URL scheme
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err(X402Error::config("URL must start with http:// or https://"));
        }

        // Basic URL format validation
        if url.len() < 8 {
            return Err(X402Error::config("URL is too short"));
        }

        // Check for invalid characters (basic check)
        if url.contains(' ') || url.contains('\n') || url.contains('\r') || url.contains('\t') {
            return Err(X402Error::config(
                "URL contains invalid whitespace characters",
            ));
        }

        Ok(())
    }

    /// Validate network name
    ///
    /// # Arguments
    /// - `network`: Network name to validate
    ///
    /// # Returns
    /// - `Ok(())` if network is supported
    /// - `Err` if network is not supported
    pub fn validate_network(network: &str) -> Result<()> {
        use rust_x402::types::networks;

        if network.is_empty() {
            return Err(X402Error::config("Network name cannot be empty"));
        }

        if !networks::is_supported(network) {
            return Err(X402Error::config(format!(
                "Unsupported network: {}. Supported networks: {}",
                network,
                networks::all_supported().join(", ")
            )));
        }

        Ok(())
    }

    /// Validate amount range
    ///
    /// # Arguments
    /// - `amount`: Amount to validate
    ///
    /// # Returns
    /// - `Ok(())` if amount is valid
    /// - `Err` if amount is out of range
    pub fn validate_amount(amount: Decimal) -> Result<()> {
        // Maximum amount: 1 billion USDC (1,000,000,000)
        let max_amount = Decimal::from_str("1000000000")
            .map_err(|_| X402Error::config("Internal error: failed to parse max amount"))?;
        // Minimum amount: 0 (zero is allowed for testing)
        let min_amount = Decimal::ZERO;

        if amount < min_amount {
            return Err(X402Error::config("Amount cannot be negative"));
        }

        if amount > max_amount {
            return Err(X402Error::config(format!(
                "Amount too large: maximum is {} USDC, got {}",
                max_amount, amount
            )));
        }

        // Check for reasonable precision (max 6 decimal places for USDC)
        let scale = amount.scale();
        if scale > 6 {
            return Err(X402Error::config(format!(
                "Amount has too many decimal places: maximum is 6, got {}",
                scale
            )));
        }

        Ok(())
    }

    /// Validate and sanitize resource path to prevent path traversal attacks
    ///
    /// # Arguments
    /// - `resource`: Resource path to validate
    ///
    /// # Returns
    /// - `Ok(String)` if path is valid and sanitized
    /// - `Err` if path contains dangerous patterns
    ///
    /// # Security
    /// This function prevents:
    /// - Path traversal attacks (../, ..\\)
    /// - Null bytes
    /// - Control characters
    /// - Excessive path length
    pub fn validate_resource_path(resource: &str) -> Result<String> {
        // Check for empty path
        if resource.trim().is_empty() {
            return Err(X402Error::config("Resource path cannot be empty"));
        }

        // Check for path traversal patterns
        if resource.contains("..") {
            return Err(X402Error::config(
                "Resource path contains invalid characters",
            ));
        }

        // Check for null bytes
        if resource.contains('\0') {
            return Err(X402Error::config(
                "Resource path contains invalid characters",
            ));
        }

        // Check for excessive length (prevent DoS)
        const MAX_PATH_LENGTH: usize = 2048;
        if resource.len() > MAX_PATH_LENGTH {
            return Err(X402Error::config("Resource path is too long"));
        }

        // Check for control characters (except newline, tab, carriage return which might be in URLs)
        if resource
            .chars()
            .any(|c| c.is_control() && c != '\n' && c != '\r' && c != '\t')
        {
            return Err(X402Error::config(
                "Resource path contains invalid characters",
            ));
        }

        // Normalize path: remove leading/trailing whitespace
        let normalized = resource.trim().to_string();

        // Ensure path starts with / (for absolute paths)
        // This prevents relative path issues
        let sanitized = if normalized.starts_with('/') {
            normalized
        } else {
            format!("/{}", normalized)
        };

        Ok(sanitized)
    }

    /// Parse Accept header to get priority (q-value) for a specific media type
    ///
    /// # Arguments
    /// - `accept_header`: Accept header value
    /// - `media_type`: Media type to check (e.g., "text/html", "application/json")
    ///
    /// # Returns
    /// - Priority value (0.0 to 1.0), defaulting to 0.0 if not found
    ///
    /// # Examples
    /// - `text/html, application/json;q=0.9` -> returns 1.0 for "text/html", 0.9 for "application/json"
    /// - `*/*;q=0.8` -> returns 0.8 for any media type
    pub fn parse_accept_priority(accept_header: &str, media_type: &str) -> f64 {
        // Split by comma and parse each media range
        for part in accept_header.split(',') {
            let part = part.trim();

            // Split media type and parameters
            let mut parts = part.split(';');
            let mime = parts.next().unwrap_or("").trim();

            // Check if this matches our target media type
            let matches = mime == media_type
                || (media_type == "*/*" && mime == "*/*")
                || (mime.starts_with(media_type) && mime[media_type.len()..].starts_with("/"));

            if matches {
                // Parse q-value (default to 1.0)
                let mut q_value = 1.0;
                for param in parts {
                    let param = param.trim();
                    if param.starts_with("q=") {
                        if let Ok(q) = param[2..].trim().parse::<f64>() {
                            q_value = q.max(0.0).min(1.0);
                        }
                    }
                }
                return q_value;
            }
        }

        // Check for wildcard match
        if media_type != "*/*" {
            return parse_accept_priority(accept_header, "*/*");
        }

        0.0
    }
}

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
