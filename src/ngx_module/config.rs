//! Configuration types for the Nginx module

use crate::ngx_module::error::{ConfigError, Result};
use ngx::core::NgxStr;
use ngx::ffi::ngx_str_t;
use rust_decimal::Decimal;
use std::str::FromStr;
use std::time::Duration;

/// Module configuration (raw strings from Nginx config)
///
/// # Memory Layout
///
/// This struct must use C-compatible memory layout (`#[repr(C)]`) because:
/// 1. It's allocated by nginx using `ngx_pcalloc` (C memory allocator)
/// 2. It's accessed via raw pointers from nginx's configuration system
/// 3. Field order and padding must match what nginx expects
/// 4. Adding new fields (like `ttl_str`) must not break existing memory layout
#[repr(C)]
#[derive(Clone, Default)]
pub struct X402Config {
    pub enabled: ngx::ffi::ngx_flag_t,
    pub amount_str: ngx_str_t,
    pub pay_to_str: ngx_str_t,
    pub facilitator_url_str: ngx_str_t,
    pub description_str: ngx_str_t,
    pub network_str: ngx_str_t,
    pub network_id_str: ngx_str_t, // Chain ID (e.g., "8453", "84532")
    pub resource_str: ngx_str_t,
    pub asset_str: ngx_str_t, // Custom token/contract address (e.g., "0x...")
    pub asset_decimals_str: ngx_str_t, // Token decimals (e.g., "6" for USDC, "18" for most ERC-20)
    pub timeout_str: ngx_str_t, // Timeout in seconds (e.g., "10")
    pub facilitator_fallback_str: ngx_str_t, // Fallback mode: "error" or "pass"
    pub ttl_str: ngx_str_t,   // TTL for payment authorization validity in seconds (e.g., "60")
}

/// Facilitator fallback mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FacilitatorFallback {
    /// Return 500 error when facilitator fails
    Error,
    /// Pass through (act as if middleware doesn't exist) when facilitator fails
    Pass,
}

/// Parsed configuration
pub struct ParsedX402Config {
    pub enabled: bool,
    pub amount: Option<Decimal>,
    pub pay_to: Option<String>,
    pub facilitator_url: Option<String>,
    pub description: Option<String>,
    pub network: Option<String>,
    pub network_id: Option<u64>, // Chain ID (e.g., 8453, 84532)
    pub resource: Option<String>,
    pub asset: Option<String>, // Custom token/contract address (overrides USDC default)
    pub asset_decimals: Option<u8>, // Token decimals (default: 6 for USDC, typically 18 for ERC-20)
    pub timeout: Option<Duration>, // Timeout for facilitator requests
    pub facilitator_fallback: FacilitatorFallback, // Fallback behavior when facilitator fails
    pub ttl: Option<u32>,      // TTL for payment authorization validity in seconds (default: 60)
}

impl X402Config {
    /// Parse raw config strings into typed values
    ///
    /// Converts Nginx configuration strings into typed values, handling empty strings
    /// and invalid formats gracefully. Validates all configuration values.
    ///
    /// # Returns
    /// - `Ok(ParsedX402Config)` with parsed and validated values (None for empty strings)
    /// - `Err` if parsing or validation fails
    ///
    /// # Note
    /// Empty strings are converted to `None` rather than causing errors.
    /// This allows the module to work with optional configuration directives.
    pub fn parse(&self) -> Result<ParsedX402Config> {
        let amount = if self.amount_str.len == 0 {
            None
        } else {
            let ngx_str = unsafe { NgxStr::from_ngx_str(self.amount_str) };
            let amount_str = ngx_str
                .to_str()
                .map_err(|_| ConfigError::from("Invalid amount string encoding"))?;

            let amount = Decimal::from_str(amount_str)
                .map_err(|e| ConfigError::from(format!("Invalid amount format: {e}")))?;

            // Validate amount range and format
            crate::config::validate_amount(amount).map_err(|e| ConfigError::from(e.to_string()))?;

            Some(amount)
        };

        let pay_to = if self.pay_to_str.len == 0 {
            None
        } else {
            let ngx_str = unsafe { NgxStr::from_ngx_str(self.pay_to_str) };
            let pay_to_str = ngx_str
                .to_str()
                .map_err(|_| ConfigError::from("Invalid pay_to string encoding"))?;

            // Validate Ethereum address format
            crate::config::validate_ethereum_address(pay_to_str)
                .map_err(|e| ConfigError::from(e.to_string()))?;

            Some(pay_to_str.to_string())
        };

        let facilitator_url = if self.facilitator_url_str.len == 0 {
            None
        } else {
            let ngx_str = unsafe { NgxStr::from_ngx_str(self.facilitator_url_str) };
            let url_str = ngx_str
                .to_str()
                .map_err(|_| ConfigError::from("Invalid facilitator_url string encoding"))?;

            // Validate URL format
            crate::config::validate_url(url_str).map_err(|e| ConfigError::from(e.to_string()))?;

            Some(url_str.to_string())
        };

        let description = if self.description_str.len == 0 {
            None
        } else {
            let ngx_str = unsafe { NgxStr::from_ngx_str(self.description_str) };
            ngx_str.to_str().ok().map(std::string::ToString::to_string)
        };

        // Parse network_id (chainId) - takes precedence over network name
        let network_id = if self.network_id_str.len == 0 {
            None
        } else {
            let ngx_str = unsafe { NgxStr::from_ngx_str(self.network_id_str) };
            let network_id_str = ngx_str
                .to_str()
                .map_err(|_| ConfigError::from("Invalid network_id string encoding"))?;

            let chain_id = network_id_str
                .parse::<u64>()
                .map_err(|e| ConfigError::from(format!("Invalid network_id format: {e}")))?;

            // Validate chainId and convert to network name
            crate::config::chain_id_to_network(chain_id)
                .map_err(|e| ConfigError::from(e.to_string()))?;

            Some(chain_id)
        };

        // Parse network name (only if network_id is not provided)
        let network = if network_id.is_some() {
            // If network_id is provided, ignore network_str
            None
        } else if self.network_str.len == 0 {
            None
        } else {
            let ngx_str = unsafe { NgxStr::from_ngx_str(self.network_str) };
            let network_str = ngx_str
                .to_str()
                .map_err(|_| ConfigError::from("Invalid network string encoding"))?;

            // Validate network name
            crate::config::validate_network(network_str)
                .map_err(|e| ConfigError::from(e.to_string()))?;

            Some(network_str.to_string())
        };

        let resource = if self.resource_str.len == 0 {
            None
        } else {
            let ngx_str = unsafe { NgxStr::from_ngx_str(self.resource_str) };
            ngx_str.to_str().ok().map(std::string::ToString::to_string)
        };

        let asset = if self.asset_str.len == 0 {
            None
        } else {
            let ngx_str = unsafe { NgxStr::from_ngx_str(self.asset_str) };
            let asset_str = ngx_str
                .to_str()
                .map_err(|_| ConfigError::from("Invalid asset string encoding"))?;

            // Validate Ethereum address format
            crate::config::validate_ethereum_address(asset_str)
                .map_err(|e| ConfigError::from(e.to_string()))?;

            Some(asset_str.to_string())
        };

        // Parse asset decimals (token precision)
        let asset_decimals = if self.asset_decimals_str.len == 0 {
            None
        } else {
            let ngx_str = unsafe { NgxStr::from_ngx_str(self.asset_decimals_str) };
            let decimals_str = ngx_str
                .to_str()
                .map_err(|_| ConfigError::from("Invalid asset_decimals string encoding"))?;

            let decimals = decimals_str
                .parse::<u8>()
                .map_err(|e| ConfigError::from(format!("Invalid asset_decimals format: {e}")))?;

            // Validate decimals range (typically 0-18 for ERC-20 tokens, max 28 for Decimal support)
            // Most tokens use 18 decimals, but we allow up to 28 (Rust Decimal max precision)
            if decimals > 28 {
                return Err(ConfigError::from(
                    "asset_decimals must be at most 28 (Decimal max precision)",
                ));
            }

            Some(decimals)
        };

        // Parse timeout (in seconds)
        let timeout = if self.timeout_str.len == 0 {
            None
        } else {
            let ngx_str = unsafe { NgxStr::from_ngx_str(self.timeout_str) };
            let timeout_str = ngx_str
                .to_str()
                .map_err(|_| ConfigError::from("Invalid timeout string encoding"))?;

            let timeout_secs = timeout_str
                .parse::<u64>()
                .map_err(|e| ConfigError::from(format!("Invalid timeout format: {e}")))?;

            // Validate timeout range (1 second to 300 seconds / 5 minutes)
            // Note: This timeout is for facilitator service requests only, not for Nginx HTTP requests.
            // Nginx HTTP timeouts (proxy_read_timeout, etc.) are configured separately in nginx.conf.
            if timeout_secs < 1 {
                return Err(ConfigError::from("Timeout must be at least 1 second"));
            }
            if timeout_secs > 300 {
                return Err(ConfigError::from(
                    "Timeout must be at most 300 seconds (5 minutes)",
                ));
            }

            Some(Duration::from_secs(timeout_secs))
        };

        // Parse facilitator fallback mode
        let facilitator_fallback = if self.facilitator_fallback_str.len == 0 {
            FacilitatorFallback::Error // Default: return error
        } else {
            let ngx_str = unsafe { NgxStr::from_ngx_str(self.facilitator_fallback_str) };
            let fallback_str = ngx_str
                .to_str()
                .map_err(|_| ConfigError::from("Invalid facilitator_fallback string encoding"))?;

            match fallback_str.to_lowercase().as_str() {
                "error" | "500" => FacilitatorFallback::Error,
                "pass" | "bypass" | "through" => FacilitatorFallback::Pass,
                _ => {
                    return Err(ConfigError::from(
                        "Invalid facilitator_fallback value. Must be 'error' or 'pass'",
                    ));
                }
            }
        };

        // Parse TTL (payment authorization validity time)
        let ttl = if self.ttl_str.len == 0 {
            None // Use default (60) from PaymentRequirements
        } else {
            let ngx_str = unsafe { NgxStr::from_ngx_str(self.ttl_str) };
            let ttl_str = ngx_str
                .to_str()
                .map_err(|_| ConfigError::from("Invalid ttl string encoding"))?;

            let ttl_value = ttl_str
                .parse::<u32>()
                .map_err(|e| ConfigError::from(format!("Invalid ttl format: {e}")))?;

            // Validate TTL range (1 second to 3600 seconds / 1 hour)
            // This controls the maximum time window for payment authorization validity
            if ttl_value < 1 {
                return Err(ConfigError::from("ttl must be at least 1 second"));
            }
            if ttl_value > 3600 {
                return Err(ConfigError::from(
                    "ttl must be at most 3600 seconds (1 hour)",
                ));
            }

            Some(ttl_value)
        };

        Ok(ParsedX402Config {
            enabled: self.enabled != 0,
            amount,
            pay_to,
            facilitator_url,
            description,
            network,
            network_id,
            resource,
            asset,
            asset_decimals,
            timeout,
            facilitator_fallback,
            ttl,
        })
    }
}
