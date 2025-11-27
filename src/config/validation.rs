//! Validation functions for configuration values
//!
//! These functions are used to validate configuration values before they are used.
//! They are public to allow testing.

use rust_decimal::Decimal;
use rust_x402::{Result, X402Error};
use std::str::FromStr;

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
            "Amount too large: maximum is {max_amount} USDC, got {amount}"
        )));
    }

    // Check for reasonable precision (max 6 decimal places for USDC)
    let scale = amount.scale();
    if scale > 6 {
        return Err(X402Error::config(format!(
            "Amount has too many decimal places: maximum is 6, got {scale}"
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
        format!("/{normalized}")
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
#[must_use] 
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
            || (mime.starts_with(media_type) && mime[media_type.len()..].starts_with('/'));

        if matches {
            // Parse q-value (default to 1.0)
            let mut q_value = 1.0;
            for param in parts {
                let param = param.trim();
                if let Some(stripped) = param.strip_prefix("q=") {
                    if let Ok(q) = stripped.trim().parse::<f64>() {
                        q_value = q.clamp(0.0, 1.0);
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

/// Validate X-PAYMENT header format and size
///
/// # Arguments
/// - `payment_b64`: Base64-encoded payment header value
///
/// # Returns
/// - `Ok(())` if header is valid
/// - `Err` if header format is invalid or too large
pub fn validate_payment_header(payment_b64: &str) -> Result<()> {
    // Maximum header size: 16KB (reasonable limit for Base64-encoded payment data)
    const MAX_PAYMENT_HEADER_SIZE: usize = 16 * 1024;

    if payment_b64.is_empty() {
        return Err(X402Error::config("X-PAYMENT header cannot be empty"));
    }

    if payment_b64.len() > MAX_PAYMENT_HEADER_SIZE {
        return Err(X402Error::config(format!(
            "X-PAYMENT header too large: maximum is {} bytes, got {}",
            MAX_PAYMENT_HEADER_SIZE,
            payment_b64.len()
        )));
    }

    // Validate Base64 characters (basic check)
    // Base64 alphabet: A-Z, a-z, 0-9, +, /, = (padding)
    if !payment_b64
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '=')
    {
        return Err(X402Error::config(
            "X-PAYMENT header contains invalid Base64 characters",
        ));
    }

    Ok(())
}
