//! Payment requirements creation

use crate::ngx_module::config::ParsedX402Config;
use crate::ngx_module::error::{ConfigError, Result};
use rust_decimal::Decimal;
use rust_x402::types::{networks, PaymentRequirements};

/// Create payment requirements from config
///
/// # Arguments
/// - `config`: Parsed configuration containing payment parameters
/// - `resource`: Resource path (URI) for the payment requirement
/// - `mime_type`: Optional MIME type for the resource (e.g., "application/json")
///
/// # Returns
/// - `Ok(PaymentRequirements)` if requirements can be created
/// - `Err` if required configuration is missing or invalid
///
/// # Errors
/// - Returns error if amount is not configured
/// - Returns error if `pay_to` address is not configured
/// - Returns error if network is not supported
/// - Returns error if USDC info cannot be set (when using default USDC)
pub fn create_requirements(
    config: &ParsedX402Config,
    resource: &str,
    mime_type: Option<&str>,
) -> Result<PaymentRequirements> {
    // Validate required fields
    let amount = config
        .amount
        .ok_or_else(|| ConfigError::from("Amount not configured"))?;

    if amount < Decimal::ZERO {
        return Err(ConfigError::from("Amount cannot be negative"));
    }

    let pay_to = config
        .pay_to
        .as_ref()
        .ok_or_else(|| ConfigError::from("Pay-to address not configured"))?;

    if pay_to.trim().is_empty() {
        return Err(ConfigError::from("Pay-to address cannot be empty"));
    }

    // Determine network - priority: network_id (chainId) > network name > default
    let network = if let Some(chain_id) = config.network_id {
        // Convert chainId to network name
        let network_name = crate::config::chain_id_to_network(chain_id)
            .map_err(|e| ConfigError::from(e.to_string()))?;
        // Store the network name temporarily to avoid lifetime issues
        // We'll use it as a string slice below
        if network_name == networks::BASE_SEPOLIA {
            networks::BASE_SEPOLIA
        } else if network_name == networks::BASE_MAINNET {
            networks::BASE_MAINNET
        } else {
            return Err(ConfigError::from(format!(
                "Unsupported network from chainId {}: {}",
                chain_id, network_name
            )));
        }
    } else if let Some(ref net) = config.network {
        net.as_str()
    } else {
        networks::BASE_MAINNET
    };

    // Determine asset address - use custom asset if provided, otherwise use USDC for the network
    let asset_address = if let Some(ref custom_asset) = config.asset {
        // Validate that network is supported even when using custom asset
        if !networks::is_supported(network) {
            return Err(ConfigError::from(format!(
                "Network not supported: {network}"
            )));
        }
        custom_asset.as_str()
    } else {
        // Get USDC address for the network (default behavior)
        networks::get_usdc_address(network)
            .ok_or_else(|| ConfigError::from(format!("Network not supported: {network}")))?
    };

    // Use configured resource or fall back to provided resource
    // Validate and sanitize the resource path to prevent path traversal attacks
    let resource = if let Some(ref resource_url) = config.resource {
        crate::config::validate_resource_path(resource_url)
            .map_err(|e| ConfigError::from(e.to_string()))?
    } else {
        crate::config::validate_resource_path(resource)
            .map_err(|e| ConfigError::from(e.to_string()))?
    };

    // Determine token decimals - use configured decimals or default to 6 (USDC)
    // If custom asset is specified but decimals not provided, default to 6 (USDC standard)
    // This ensures backward compatibility while allowing custom tokens to specify their precision
    let decimals = config.asset_decimals.unwrap_or(6u8);

    // Validate that amount precision doesn't exceed token decimals
    let amount_scale = amount.scale();
    if amount_scale > decimals as u32 {
        return Err(ConfigError::from(format!(
            "Amount has {amount_scale} decimal places, but token only supports {decimals} decimals"
        )));
    }

    // Calculate multiplier based on decimals (10^decimals)
    // For USDC (6 decimals): 10^6 = 1,000,000
    // For standard ERC-20 (18 decimals): 10^18
    let multiplier = match decimals {
        0 => Decimal::from(1u64),
        1 => Decimal::from(10u64),
        2 => Decimal::from(100u64),
        3 => Decimal::from(1_000u64),
        4 => Decimal::from(10_000u64),
        5 => Decimal::from(100_000u64),
        6 => Decimal::from(1_000_000u64),
        7 => Decimal::from(10_000_000u64),
        8 => Decimal::from(100_000_000u64),
        9 => Decimal::from(1_000_000_000u64),
        10 => Decimal::from(10_000_000_000u64),
        11 => Decimal::from(100_000_000_000u64),
        12 => Decimal::from(1_000_000_000_000u64),
        13 => Decimal::from(10_000_000_000_000u64),
        14 => Decimal::from(100_000_000_000_000u64),
        15 => Decimal::from(1_000_000_000_000_000u64),
        16 => Decimal::from(10_000_000_000_000_000u64),
        17 => Decimal::from(100_000_000_000_000_000u64),
        18 => Decimal::from(1_000_000_000_000_000_000u64),
        _ => {
            // For decimals > 18, use power calculation
            // Note: Rust Decimal supports up to 28 decimal places
            let mut result = Decimal::from(1u64);
            for _ in 0..decimals {
                result *= Decimal::from(10u64);
            }
            result
        }
    };

    // Convert amount to max_amount_required (in smallest unit based on token decimals)
    let max_amount_required = (amount * multiplier).normalize().to_string();

    let mut requirements = PaymentRequirements::new(
        rust_x402::types::schemes::EXACT,
        network,
        max_amount_required,
        asset_address,
        pay_to.to_lowercase(),
        resource,
        config.description.as_deref().unwrap_or(""),
    );

    // Set MIME type if provided
    // Note: This may not be supported by all versions of rust_x402
    // If set_mime_type doesn't exist, this will fail at compile time
    if let Some(mime) = mime_type {
        // Try to set mimeType using reflection or direct method call
        // Check if PaymentRequirements has a set_mime_type method
        // For now, we'll try to use it if available
        #[allow(unused)]
        let _ = mime; // Use mime_type parameter
        // TODO: Add mimeType support when rust_x402 library supports it
        // This may require updating rust_x402 dependency or using a different approach
    }

    // Set network-specific USDC info only if using default USDC (not custom asset)
    // This ensures compatibility with USDC-specific metadata while allowing custom tokens
    if config.asset.is_none() {
        // Determine network enum from network string
        let network_enum = if network == networks::BASE_SEPOLIA {
            rust_x402::types::Network::Testnet
        } else {
            rust_x402::types::Network::Mainnet
        };
        requirements.set_usdc_info(network_enum)?;
    }

    Ok(requirements)
}
