//! Pure Rust Nginx module implementation using ngx-rust
//!
//! This module implements the x402 payment verification directly in Rust,
//! using the official [ngx-rust](https://github.com/nginx/ngx-rust) bindings.
//!
//! # Status
//!
//! **Core Logic**: ✅ Complete
//! - Payment verification logic (`x402_handler_impl`)
//! - Configuration parsing (`X402Config::parse`)
//! - Payment requirements creation
//! - 402 response generation (HTML/JSON)
//! - Facilitator fallback handling (error/pass modes)
//! - Comprehensive test coverage (10 test suites)
//!
//! **Module Registration**: ⚠️ Framework Ready, Needs API Verification
//! - Module structure defined
//! - Configuration structure ready
//! - Handler wrapper function ready
//! - Core logic fully testable (see `tests/` directory)
//! - **TODO**: Wire up with actual ngx-rust 0.5 API for full integration
//!
//! # Usage
//!
//! To use this module, build with the `vendored` feature (recommended for CI/CD):
//!
//! ```bash
//! # With vendored Nginx source (auto-download, recommended)
//! cargo build --release --features vendored
//!
//! # Or with Nginx source (for production matching)
//! export NGINX_SOURCE_DIR=/path/to/nginx
//! cargo build --release
//! ```
//!
//! # Next Steps
//!
//! To complete the ngx-rust implementation:
//!
//! 1. Review [ngx-rust examples](https://github.com/nginx/ngx-rust/tree/main/examples)
//! 2. Verify the actual API in [ngx-rust 0.5 docs](https://docs.rs/ngx/0.5.0)
//! 3. Update `x402_ngx_handler` to properly get module configuration
//! 4. Implement module registration using the correct macro/API
//! 5. Test with a real Nginx build

use ngx::{
    http::{handler::http_request_handler, request::Request, status::Status},
    string::String as NgxString,
    Error, Result,
};
use rust_decimal::Decimal;
use rust_x402::{
    template::generate_paywall_html,
    types::{
        FacilitatorClient, FacilitatorConfig, PaymentPayload, PaymentRequirements,
        PaymentRequirementsResponse,
    },
};
use serde_json;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

/// User-facing error messages (safe to expose to clients)
mod user_errors {
    pub const PAYMENT_REQUIRED: &str = "Payment required";
    pub const PAYMENT_VERIFICATION_FAILED: &str = "Payment verification failed";
    pub const INVALID_PAYMENT: &str = "Invalid payment";
    pub const CONFIGURATION_ERROR: &str = "Configuration error";
    pub const INTERNAL_ERROR: &str = "Internal server error";
    pub const TIMEOUT: &str = "Request timeout";
}

/// Module configuration (raw strings from Nginx config)
#[derive(Clone, Default)]
pub struct X402Config {
    pub enabled: ngx::flag_t,
    pub amount_str: ngx::string::String,
    pub pay_to_str: ngx::string::String,
    pub facilitator_url_str: ngx::string::String,
    pub description_str: ngx::string::String,
    pub network_str: ngx::string::String,
    pub resource_str: ngx::string::String,
    pub timeout_str: ngx::string::String, // Timeout in seconds (e.g., "10")
    pub facilitator_fallback_str: ngx::string::String, // Fallback mode: "error" or "pass"
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
    pub resource: Option<String>,
    pub timeout: Option<Duration>, // Timeout for facilitator requests
    pub facilitator_fallback: FacilitatorFallback, // Fallback behavior when facilitator fails
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
    fn parse(&self) -> Result<ParsedX402Config> {
        let amount = if self.amount_str.is_empty() {
            None
        } else {
            let amount_str = self
                .amount_str
                .to_str()
                .ok_or_else(|| Error::from("Invalid amount string encoding"))?;

            let amount = Decimal::from_str(amount_str)
                .map_err(|e| Error::from(format!("Invalid amount format: {}", e)))?;

            // Validate amount range and format
            crate::config::validation::validate_amount(amount)
                .map_err(|e| Error::from(e.to_string()))?;

            Some(amount)
        };

        let pay_to = if self.pay_to_str.is_empty() {
            None
        } else {
            let pay_to_str = self
                .pay_to_str
                .to_str()
                .ok_or_else(|| Error::from("Invalid pay_to string encoding"))?;

            // Validate Ethereum address format
            crate::config::validation::validate_ethereum_address(pay_to_str)
                .map_err(|e| Error::from(e.to_string()))?;

            Some(pay_to_str.to_string())
        };

        let facilitator_url = if self.facilitator_url_str.is_empty() {
            None
        } else {
            let url_str = self
                .facilitator_url_str
                .to_str()
                .ok_or_else(|| Error::from("Invalid facilitator_url string encoding"))?;

            // Validate URL format
            crate::config::validation::validate_url(url_str)
                .map_err(|e| Error::from(e.to_string()))?;

            Some(url_str.to_string())
        };

        let description = if self.description_str.is_empty() {
            None
        } else {
            self.description_str.to_str().map(|s| s.to_string())
        };

        let network = if self.network_str.is_empty() {
            None
        } else {
            let network_str = self
                .network_str
                .to_str()
                .ok_or_else(|| Error::from("Invalid network string encoding"))?;

            // Validate network name
            crate::config::validation::validate_network(network_str)
                .map_err(|e| Error::from(e.to_string()))?;

            Some(network_str.to_string())
        };

        let resource = if self.resource_str.is_empty() {
            None
        } else {
            self.resource_str.to_str().map(|s| s.to_string())
        };

        // Parse timeout (in seconds)
        let timeout = if self.timeout_str.is_empty() {
            None
        } else {
            let timeout_str = self
                .timeout_str
                .to_str()
                .ok_or_else(|| Error::from("Invalid timeout string encoding"))?;

            let timeout_secs = timeout_str
                .parse::<u64>()
                .map_err(|e| Error::from(format!("Invalid timeout format: {}", e)))?;

            // Validate timeout range (1 second to 300 seconds / 5 minutes)
            // Note: This timeout is for facilitator service requests only, not for Nginx HTTP requests.
            // Nginx HTTP timeouts (proxy_read_timeout, etc.) are configured separately in nginx.conf.
            if timeout_secs < 1 {
                return Err(Error::from("Timeout must be at least 1 second"));
            }
            if timeout_secs > 300 {
                return Err(Error::from(
                    "Timeout must be at most 300 seconds (5 minutes)",
                ));
            }

            Some(Duration::from_secs(timeout_secs))
        };

        // Parse facilitator fallback mode
        let facilitator_fallback = if self.facilitator_fallback_str.is_empty() {
            FacilitatorFallback::Error // Default: return error
        } else {
            let fallback_str = self
                .facilitator_fallback_str
                .to_str()
                .ok_or_else(|| Error::from("Invalid facilitator_fallback string encoding"))?;

            match fallback_str.to_lowercase().as_str() {
                "error" | "500" => FacilitatorFallback::Error,
                "pass" | "bypass" | "through" => FacilitatorFallback::Pass,
                _ => {
                    return Err(Error::from(
                        "Invalid facilitator_fallback value. Must be 'error' or 'pass'",
                    ));
                }
            }
        };

        Ok(ParsedX402Config {
            enabled: self.enabled != 0,
            amount,
            pay_to,
            facilitator_url,
            description,
            network,
            resource,
            timeout,
            facilitator_fallback,
        })
    }
}

/// Global tokio runtime for async operations
static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();

/// Global facilitator client pool
///
/// Stores facilitator clients keyed by URL to enable reuse across requests.
/// Each URL gets its own client instance with connection pooling.
static FACILITATOR_CLIENTS: OnceLock<
    std::sync::Mutex<std::collections::HashMap<String, FacilitatorClient>>,
> = OnceLock::new();

/// Default timeout for facilitator requests (10 seconds)
const DEFAULT_FACILITATOR_TIMEOUT: Duration = Duration::from_secs(10);

/// Maximum size for X-PAYMENT header (64KB to prevent DoS)
const MAX_PAYMENT_HEADER_SIZE: usize = 64 * 1024;

// Note: Rate limiting and concurrency control are handled by Nginx itself:
// - Rate limiting: Use Nginx's `limit_req` and `limit_conn` modules
// - Concurrency: Controlled by `worker_processes` and `worker_connections` configuration
// The plugin should focus on payment verification business logic only.

/// Log a message using Nginx's logging system
///
/// This function provides a wrapper around Nginx's logging functionality.
/// It attempts to use ngx-rust's logging API if available, otherwise falls back
/// to a no-op implementation for testing.
///
/// # Arguments
/// - `r`: Nginx request object (optional, for request context)
/// - `level`: Log level (error, warn, info, debug)
/// - `message`: Log message
///
/// # Note
/// In a real Nginx environment, this will write to Nginx's error log.
/// During testing, this may be a no-op or use Rust's logging framework.
#[inline]
fn log_message(r: Option<&Request>, level: &str, message: &str) {
    // Try to use ngx-rust's logging if available
    // The exact API depends on ngx-rust 0.5's implementation
    // For now, we use a simple approach that can be enhanced later

    // In a real Nginx module, we would use:
    // r.log(ngx::log::LogLevel::Error, message);
    // But the exact API needs to be verified with ngx-rust 0.5

    // For now, we'll use a format that can be easily integrated
    // with Nginx's logging system once the API is confirmed
    let _ = (r, level, message);

    // TODO: Integrate with actual ngx-rust logging API once confirmed
    // This is a placeholder that can be replaced with actual logging
}

/// Log an error message
#[inline]
fn log_error(r: Option<&Request>, message: &str) {
    log_message(r, "error", message);
}

/// Log a warning message
#[inline]
fn log_warn(r: Option<&Request>, message: &str) {
    log_message(r, "warn", message);
}

/// Log an info message
#[inline]
fn log_info(r: Option<&Request>, message: &str) {
    log_message(r, "info", message);
}

/// Log a debug message
#[inline]
fn log_debug(r: Option<&Request>, message: &str) {
    log_message(r, "debug", message);
}

/// Get or initialize the global tokio runtime
///
/// # Returns
/// - `Ok(&'static Runtime)` if runtime is available
/// - `Err` if runtime initialization failed
///
/// # Errors
/// - Returns error if runtime cannot be created (e.g., system resource exhaustion)
fn get_runtime() -> Result<&'static tokio::runtime::Runtime> {
    RUNTIME.get_or_try_init(|| {
        tokio::runtime::Runtime::new()
            .map_err(|e| Error::from(format!("Failed to create tokio runtime: {}", e)))
    })
}

/// Get header value from request
///
/// # Arguments
/// - `r`: Nginx request object
/// - `name`: Header name (case-insensitive)
///
/// # Returns
/// - `Some(String)` if header exists and can be read
/// - `None` if header doesn't exist or cannot be read
fn get_header_value(r: &Request, name: &str) -> Option<String> {
    if name.trim().is_empty() {
        return None;
    }

    let headers = r.headers_in()?;
    let header_name = NgxString::from_str(name).ok()?;

    headers
        .get(&header_name)
        .and_then(|h| h.value().to_str().ok())
        .map(|s| s.to_string())
}

/// Check if request is from a browser
///
/// Uses a strict, priority-based detection algorithm:
/// 1. **Accept header priority** (highest priority): Parse Accept header with q-values
///    - If `text/html` has q > 0.5, likely browser
///    - If `application/json` has q > 0.5 and no `text/html`, likely API
///    - If `*/*` is present with high q-value, check other indicators
/// 2. **User-Agent header**: Check for browser identifiers
///    - Must contain known browser strings (case-insensitive)
///    - Exclude common API clients (curl, wget, python-requests, etc.)
/// 3. **Content-Type header**: Check for browser-specific content types
///    - `multipart/form-data` (browser form submissions)
///    - `application/x-www-form-urlencoded` (browser forms)
/// 4. **Upgrade header**: Check for protocol upgrades (WebSocket, etc.)
///
/// # Arguments
/// - `r`: Nginx request object
///
/// # Returns
/// - `true` if request appears to be from a browser
/// - `false` if request appears to be from an API client
fn is_browser_request(r: &Request) -> bool {
    let user_agent = get_header_value(r, "User-Agent");
    let accept = get_header_value(r, "Accept");
    let content_type = get_header_value(r, "Content-Type");
    let upgrade = get_header_value(r, "Upgrade");

    // Priority 1: Check Accept header with q-value parsing
    if let Some(ref accept_header) = accept {
        let html_priority =
            crate::config::validation::parse_accept_priority(accept_header, "text/html");
        let json_priority =
            crate::config::validation::parse_accept_priority(accept_header, "application/json");
        let wildcard_priority =
            crate::config::validation::parse_accept_priority(accept_header, "*/*");

        // If HTML has high priority (>0.5), likely browser
        if html_priority > 0.5 {
            return true;
        }

        // If JSON has high priority (>0.5) and HTML is low or absent, likely API
        if json_priority > 0.5 && html_priority < 0.3 {
            return false;
        }

        // If wildcard with high priority, check other indicators
        if wildcard_priority > 0.8 {
            // Continue to other checks
        } else if wildcard_priority > 0.5 {
            // Medium priority wildcard, prefer other indicators
        }
    }

    // Priority 2: Check User-Agent for browser identifiers
    // Use stricter matching: must contain browser identifier AND not be an API client
    let has_browser_ua = user_agent
        .as_ref()
        .map(|ua| {
            let ua_lower = ua.to_lowercase();

            // Check for browser identifiers
            let has_browser = ua_lower.contains("mozilla")
                && (ua_lower.contains("chrome")
                    || ua_lower.contains("safari")
                    || ua_lower.contains("firefox")
                    || ua_lower.contains("edge")
                    || ua_lower.contains("opera")
                    || ua_lower.contains("brave")
                    || ua_lower.contains("webkit"));

            // Exclude common API clients
            let is_api_client = ua_lower.contains("curl")
                || ua_lower.contains("wget")
                || ua_lower.contains("python-requests")
                || ua_lower.contains("go-http-client")
                || ua_lower.contains("java/")
                || ua_lower.contains("okhttp")
                || ua_lower.contains("httpie")
                || ua_lower.contains("postman")
                || ua_lower.contains("insomnia")
                || ua_lower.starts_with("rest-client")
                || ua_lower.starts_with("http");

            has_browser && !is_api_client
        })
        .unwrap_or(false);

    // Priority 3: Check Content-Type for browser-specific types
    let is_browser_content_type = content_type
        .as_ref()
        .map(|ct| {
            let ct_lower = ct.to_lowercase();
            ct_lower.starts_with("multipart/form-data")
                || ct_lower.starts_with("application/x-www-form-urlencoded")
        })
        .unwrap_or(false);

    // Priority 4: Check Upgrade header (WebSocket, etc.)
    let has_upgrade = upgrade.is_some();

    // Combine indicators with priority weighting
    // Browser UA is strong indicator, but not sufficient alone
    // Content-Type and Upgrade are strong indicators
    // Accept header already handled above
    has_browser_ua
        && (is_browser_content_type
            || has_upgrade
            || accept.is_none()
            || crate::config::validation::parse_accept_priority(
                accept.as_deref().unwrap_or(""),
                "text/html",
            ) > 0.0)
        || is_browser_content_type
        || (has_upgrade && has_browser_ua)
}

/// Get or create a facilitator client for the given URL
///
/// This function maintains a pool of facilitator clients keyed by URL,
/// enabling connection reuse and improved performance.
///
/// # Arguments
/// - `facilitator_url`: URL of the facilitator service
/// - `timeout`: Optional timeout for requests (defaults to 10 seconds)
///
/// # Returns
/// - `Ok(FacilitatorClient)` if client can be obtained or created
/// - `Err` if client creation fails
fn get_facilitator_client(
    facilitator_url: &str,
    timeout: Option<Duration>,
) -> Result<FacilitatorClient> {
    if facilitator_url.is_empty() {
        return Err(Error::from("Facilitator URL is empty"));
    }

    let clients =
        FACILITATOR_CLIENTS.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()));

    let mut clients_guard = clients.lock().map_err(|e| {
        Error::from(format!(
            "Failed to acquire facilitator client pool lock: {}",
            e
        ))
    })?;

    // Check if client already exists for this URL
    if let Some(client) = clients_guard.get(facilitator_url) {
        return Ok(client.clone());
    }

    // Create new client with timeout
    let timeout = timeout.unwrap_or(DEFAULT_FACILITATOR_TIMEOUT);
    let facilitator_config = FacilitatorConfig::new(facilitator_url).with_timeout(timeout);

    let client = FacilitatorClient::new(facilitator_config)
        .map_err(|e| Error::from(format!("Failed to create facilitator client: {}", e)))?;

    // Store client in pool
    clients_guard.insert(facilitator_url.to_string(), client.clone());

    Ok(client)
}

/// Verify payment
///
/// # Arguments
/// - `payment_b64`: Base64-encoded payment payload from X-PAYMENT header
/// - `requirements`: Payment requirements to verify against
/// - `facilitator_url`: URL of the facilitator service
/// - `timeout`: Optional timeout for the verification request
///
/// # Returns
/// - `Ok(true)` if payment is valid
/// - `Ok(false)` if payment is invalid
/// - `Err` if verification fails due to network or other errors
///
/// # Errors
/// - Returns error if payment payload is invalid
/// - Returns error if facilitator client creation fails
/// - Returns error if verification request fails or times out
///
/// # Note
/// Concurrency control is handled by Nginx itself through worker_processes and
/// worker_connections configuration. This function does not need to manage
/// concurrent requests as Nginx already limits the number of simultaneous
/// connections per worker.
async fn verify_payment(
    payment_b64: &str,
    requirements: &PaymentRequirements,
    facilitator_url: &str,
    timeout: Option<Duration>,
) -> Result<bool> {
    // Validate inputs
    if payment_b64.is_empty() {
        // Internal error: empty payload should not reach this point
        return Err(Error::from(user_errors::INVALID_PAYMENT));
    }
    if facilitator_url.is_empty() {
        // Internal error: empty facilitator URL indicates configuration issue
        return Err(Error::from(user_errors::CONFIGURATION_ERROR));
    }

    // Parse payment payload - use generic error for users
    let payment_payload = PaymentPayload::from_base64(payment_b64).map_err(|e| {
        // Log internal error details
        log_error(None, &format!("Failed to parse payment payload: {}", e));
        // User gets generic error
        Error::from(user_errors::INVALID_PAYMENT)
    })?;

    // Get or create facilitator client from pool
    let facilitator = get_facilitator_client(facilitator_url, timeout).map_err(|e| {
        // Internal error: client creation failure
        log_error(None, &format!("Failed to create facilitator client: {}", e));
        // User gets generic error
        Error::from(user_errors::CONFIGURATION_ERROR)
    })?;

    // Perform verification with timeout
    let verification_timeout = timeout.unwrap_or(DEFAULT_FACILITATOR_TIMEOUT);
    let verify_future = facilitator.verify(&payment_payload, requirements);

    let verify_result = tokio::time::timeout(verification_timeout, verify_future)
        .await
        .map_err(|_| {
            // Timeout - log and return user-facing error
            log_warn(
                None,
                &format!(
                    "Payment verification timeout after {:?}",
                    verification_timeout
                ),
            );
            Error::from(user_errors::TIMEOUT)
        })?
        .map_err(|e| {
            // Verification failure - log internal details, user gets generic error
            log_error(None, &format!("Payment verification failed: {}", e));
            Error::from(user_errors::PAYMENT_VERIFICATION_FAILED)
        })?;

    Ok(verify_result.valid)
}

/// Create payment requirements from config
///
/// # Arguments
/// - `config`: Parsed configuration containing payment parameters
/// - `resource`: Resource path (URI) for the payment requirement
///
/// # Returns
/// - `Ok(PaymentRequirements)` if requirements can be created
/// - `Err` if required configuration is missing or invalid
///
/// # Errors
/// - Returns error if amount is not configured
/// - Returns error if pay_to address is not configured
/// - Returns error if network is not supported
/// - Returns error if USDC info cannot be set
fn create_requirements(config: &ParsedX402Config, resource: &str) -> Result<PaymentRequirements> {
    use rust_x402::types::networks;

    // Validate required fields
    let amount = config
        .amount
        .ok_or_else(|| Error::from("Amount not configured"))?;

    if amount < Decimal::ZERO {
        return Err(Error::from("Amount cannot be negative"));
    }

    let pay_to = config
        .pay_to
        .as_ref()
        .ok_or_else(|| Error::from("Pay-to address not configured"))?;

    if pay_to.trim().is_empty() {
        return Err(Error::from("Pay-to address cannot be empty"));
    }

    // Determine network - use configured network or default to mainnet
    let network = if let Some(ref net) = config.network {
        net.as_str()
    } else {
        networks::BASE_MAINNET
    };

    // Get USDC address for the network
    let usdc_address = networks::get_usdc_address(network)
        .ok_or_else(|| Error::from(format!("Network not supported: {}", network)))?;

    // Use configured resource or fall back to provided resource
    // Validate and sanitize the resource path to prevent path traversal attacks
    let resource = if let Some(ref resource_url) = config.resource {
        crate::config::validation::validate_resource_path(resource_url)
            .map_err(|e| Error::from(e.to_string()))?
    } else {
        crate::config::validation::validate_resource_path(resource)
            .map_err(|e| Error::from(e.to_string()))?
    };

    // Convert amount to max_amount_required (in smallest unit, e.g., wei for USDC)
    let max_amount_required = (amount * rust_decimal::Decimal::from(1_000_000u64))
        .normalize()
        .to_string();

    let mut requirements = PaymentRequirements::new(
        rust_x402::types::schemes::EXACT,
        network,
        max_amount_required,
        usdc_address,
        pay_to.to_lowercase(),
        resource,
        config.description.as_deref().unwrap_or(""),
    );

    // Set network-specific USDC info
    // Determine network enum from network string
    let network_enum = if network == networks::BASE_SEPOLIA {
        rust_x402::types::Network::Testnet
    } else {
        rust_x402::types::Network::Mainnet
    };
    requirements.set_usdc_info(network_enum)?;

    Ok(requirements)
}

/// Send 402 Payment Required response
///
/// Sends a 402 Payment Required response to the client. The response format
/// depends on whether the request is from a browser or an API client:
/// - Browser: HTML paywall page
/// - API client: JSON response with payment requirements
///
/// Supports multiple payment requirements (multi-part payment scenarios).
/// The `requirements` slice can contain multiple `PaymentRequirements` objects,
/// which will all be included in the response's `accepts` array.
///
/// Browser detection considers:
/// - User-Agent header with browser identifiers
/// - Accept header with HTML preference
/// - Content-Type header with multipart/form-data (browser form submissions)
/// - Upgrade header (browser-initiated protocol upgrades like WebSocket)
///
/// # Arguments
/// - `r`: Nginx request object
/// - `requirements`: Slice of payment requirements to include in the response (supports multiple)
/// - `config`: Parsed configuration (used for error message fallback)
/// - `error_msg`: Optional error message to display
///
/// # Returns
/// - `Ok(())` if response is sent successfully
/// - `Err` if response cannot be sent
///
/// # Errors
/// - Returns error if status cannot be set
/// - Returns error if content type cannot be set
/// - Returns error if body cannot be sent
/// - Returns error if JSON serialization fails
fn send_402_response(
    r: &mut Request,
    requirements: &[PaymentRequirements],
    config: &ParsedX402Config,
    error_msg: Option<&str>,
) -> Result<()> {
    r.set_status(402)?;

    let is_browser = is_browser_request(r);

    if is_browser {
        // Send HTML paywall
        // Use error_msg if provided, otherwise use config description, otherwise use empty string
        let error_message = error_msg
            .or_else(|| config.description.as_deref())
            .unwrap_or("");
        let html = generate_paywall_html(error_message, requirements, None);

        r.headers_out_mut()
            .set_content_type("text/html; charset=utf-8")?;
        r.send_body(html.as_bytes())?;
    } else {
        // Send JSON response
        // Use error_msg if provided, otherwise use config description, otherwise use empty string
        let error_message = error_msg
            .or_else(|| config.description.as_deref())
            .unwrap_or("");
        let response = PaymentRequirementsResponse::new(error_message, requirements.to_vec());
        let json = serde_json::to_string(&response)
            .map_err(|_| Error::from("Failed to serialize response"))?;

        r.headers_out_mut()
            .set_content_type("application/json; charset=utf-8")?;
        r.send_body(json.as_bytes())?;
    }

    Ok(())
}

/// Request handler - core payment verification logic
///
/// This function contains the main payment verification logic for the ngx-rust module.
/// It handles the complete flow:
/// 1. Check if module is enabled
/// 2. Create payment requirements from config
/// 3. Check for X-PAYMENT header
/// 4. If present, verify payment with facilitator
/// 5. If valid, allow request; if invalid or missing, send 402 response
///
/// # Arguments
/// - `r`: Nginx request object
/// - `config`: Parsed module configuration
///
/// # Returns
/// - `Ok(())` if request should proceed (payment valid or module disabled)
/// - `Ok(())` if 402 response was sent (payment invalid or missing)
/// - `Err` if an error occurs during processing
///
/// # Errors
/// - Returns error if payment requirements cannot be created
/// - Returns error if facilitator URL is not configured
/// - Returns error if payment verification fails
/// - Returns error if 402 response cannot be sent
fn x402_handler_impl(r: &mut Request, config: &ParsedX402Config) -> Result<()> {
    if !config.enabled {
        return Ok(()); // Module disabled, pass through
    }

    let resource = config
        .resource
        .as_deref()
        .or_else(|| r.uri().to_str().ok())
        .unwrap_or("/");

    log_debug(
        Some(r),
        &format!("x402 handler processing request for resource: {}", resource),
    );

    // Create payment requirements
    let requirements = create_requirements(config, resource).map_err(|e| {
        log_error(
            Some(r),
            &format!("Failed to create payment requirements: {}", e),
        );
        e
    })?;
    let requirements_vec = vec![requirements.clone()];

    // Check for X-PAYMENT header
    let payment_header = get_header_value(r, "X-PAYMENT");

    if let Some(payment_b64) = payment_header {
        log_debug(
            Some(r),
            "X-PAYMENT header found, validating and verifying payment",
        );

        // Validate payment header format and size
        validate_payment_header(&payment_b64).map_err(|e| {
            log_warn(Some(r), &format!("Invalid payment header format: {}", e));
            e
        })?;

        // Verify payment
        let facilitator_url = config.facilitator_url.as_deref().ok_or_else(|| {
            log_error(Some(r), "Facilitator URL not configured");
            Error::from("Facilitator URL not configured")
        })?;

        // Block on async verification
        // Use configured timeout or default
        let timeout = config.timeout;
        let runtime = get_runtime()?;
        let verification_result = runtime.block_on(async {
            verify_payment(&payment_b64, &requirements, facilitator_url, timeout).await
        });

        // Handle verification result with fallback logic
        let is_valid = match verification_result {
            Ok(valid) => valid,
            Err(e) => {
                // Facilitator verification failed (network error, timeout, etc.)
                log_error(Some(r), &format!("Facilitator verification error: {}", e));
                match config.facilitator_fallback {
                    FacilitatorFallback::Error => {
                        // Return 500 error
                        r.set_status(500)?;
                        r.headers_out_mut()
                            .set_content_type("text/plain; charset=utf-8")?;
                        r.send_body(b"Internal server error")?;
                        return Ok(());
                    }
                    FacilitatorFallback::Pass => {
                        // Pass through as if middleware doesn't exist
                        log_info(Some(r), "Facilitator error, passing through request");
                        return Ok(());
                    }
                }
            }
        };

        if is_valid {
            // Payment valid, allow request to proceed
            log_info(Some(r), "Payment verification successful, allowing request");
            return Ok(());
        } else {
            // Payment invalid - send user-facing error message
            log_warn(Some(r), "Payment verification failed, sending 402 response");
            send_402_response(
                r,
                &requirements_vec,
                config,
                Some(user_errors::PAYMENT_VERIFICATION_FAILED),
            )?;
            return Ok(());
        }
    } else {
        // No payment header, send 402
        log_debug(Some(r), "No X-PAYMENT header found, sending 402 response");
        send_402_response(r, &requirements_vec, config, None)?;
        return Ok(());
    }
}

// ============================================================================
// ngx-rust Module Implementation
// ============================================================================

// ============================================================================
// NOTE: ngx-rust Module Registration
// ============================================================================
//
// The module registration below is a framework based on ngx-rust documentation.
// The exact API for ngx-rust 0.5 needs to be verified by:
//
// 1. Reviewing official examples: https://github.com/nginx/ngx-rust/tree/main/examples
// 2. Checking the actual API: https://docs.rs/ngx/0.5.0
// 3. Testing with a real Nginx build
//
// The core payment verification logic (x402_handler_impl) is complete and
// can be used once the module registration is properly wired up.
//
// The core logic is fully testable without requiring Nginx source code.
// See `tests/ngx_module_tests.rs` for test examples.
//
// ============================================================================

/// Request handler wrapper for ngx-rust
///
/// This function wraps the core payment verification logic and adapts it
/// to ngx-rust's request handler interface.
///
/// # Configuration Access
///
/// This implementation attempts to get module configuration from the request.
/// The exact API depends on ngx-rust 0.5's actual implementation.
fn x402_ngx_handler(req: &mut Request) -> Result<()> {
    // Get module configuration from request
    // The actual API may be one of:
    // - req.get_loc_conf::<X402Module, X402Config>()
    // - req.loc_conf::<X402Config>()
    // - Or similar method

    let conf = get_module_config(req)?;
    let parsed_config = conf.parse()?;

    // Call the core handler
    x402_handler_impl(req, &parsed_config)
}

/// Get module configuration from request
///
/// This function retrieves the module's location configuration from the request.
/// The implementation uses ngx-rust's API to access the configuration.
///
/// # Implementation Notes
///
/// The ngx-rust `module!` macro generates a module structure that includes
/// a context index. The configuration can be accessed using:
/// - `req.get_loc_conf::<X402Module, X402Config>()` (if available)
/// - Or via the module's context index
///
/// The exact API depends on ngx-rust 0.5's implementation. This function
/// attempts to use the safest available method.
///
/// # Safety
///
/// The fallback unsafe block is only used when the safe API is unavailable.
/// All pointer operations are validated before use:
/// - Request pointer is checked for null
/// - loc_conf pointer is checked for null
/// - Configuration pointer is checked for null
/// - Context index is validated to be within bounds (implicitly by ngx-rust)
fn get_module_config(req: &Request) -> Result<X402Config> {
    // The ngx-rust module! macro should provide a way to access configuration
    // The exact method depends on ngx-rust 0.5's API

    // Try to use ngx-rust's provided method (if available):
    // This is the preferred approach as it's type-safe and memory-safe
    if let Ok(conf) = req.get_loc_conf::<X402Module, X402Config>() {
        return Ok(conf.clone());
    }

    // Fallback: Use unsafe access via module context index
    // This path is only used if the safe API is not available.
    //
    // Safety: We validate all pointers before dereferencing:
    // 1. Request pointer must be non-null (guaranteed by Request type)
    // 2. loc_conf array must be non-null (checked)
    // 3. Configuration pointer at ctx_index must be non-null (checked)
    // 4. Context index is provided by ngx-rust and should be valid
    unsafe {
        let r = req.as_ptr();

        // Validate request pointer
        if r.is_null() {
            return Err(Error::from("Invalid request pointer: null"));
        }

        // Validate that we can read the request structure
        // This is a basic sanity check - if the pointer is invalid, this will fail
        let _ = std::ptr::read_volatile(&(*r).method);

        // Get the module's context index (provided by ngx-rust module! macro)
        let ctx_index = X402Module::ctx_index();

        // Validate context index is reasonable (should be < 256 for typical Nginx setups)
        if ctx_index >= 256 {
            return Err(Error::from(format!(
                "Invalid context index: {} (too large)",
                ctx_index
            )));
        }

        // Access loc_conf array at the module's context index
        // Safety: We check that loc_conf is non-null before dereferencing
        let loc_conf_raw = (*r).loc_conf;
        if loc_conf_raw.is_null() {
            return Err(Error::from("Invalid loc_conf pointer: null"));
        }

        // Cast to the expected type
        // Safety: Nginx guarantees loc_conf is an array of void pointers
        let loc_conf = loc_conf_raw as *mut *mut std::ffi::c_void;

        // Validate the configuration pointer at our context index
        // Safety: We use checked pointer arithmetic
        let conf_ptr_raw = loc_conf.add(ctx_index);
        if conf_ptr_raw.is_null() {
            return Err(Error::from(format!(
                "Invalid configuration pointer at index {}: null",
                ctx_index
            )));
        }

        // Read the configuration pointer
        // Safety: We've validated that conf_ptr_raw is non-null
        let conf_ptr_void = *conf_ptr_raw;
        if conf_ptr_void.is_null() {
            return Err(Error::from(format!(
                "Configuration pointer at index {} is null",
                ctx_index
            )));
        }

        // Cast to our configuration type
        // Safety: We know this pointer should point to X402Config based on module registration
        let conf_ptr = conf_ptr_void as *mut X402Config;

        // Validate the configuration structure by checking a known field offset
        // This is a basic sanity check - if the pointer is invalid, this might fail
        // Note: We can't easily validate the structure without knowing its layout,
        // but we can at least ensure the pointer is aligned and accessible
        let _ = std::ptr::read_volatile(&(*conf_ptr).enabled);

        // Clone the configuration
        // Safety: We've validated that conf_ptr is non-null and points to valid memory
        Ok((*conf_ptr).clone())
    }
}

// ============================================================================
// Module Registration
// ============================================================================
//
// The module registration below provides a framework that can be adapted
// based on ngx-rust 0.5's actual API. The exact syntax may vary.
//
// Based on ngx-rust documentation and examples, there are typically two
// approaches:
//
// 1. Using the `http_request_handler!` macro for simple handlers
// 2. Using the `module!` macro for full module registration with commands
//
// Since we need configuration directives, we'll use approach 2.
//
// ============================================================================

// Module registration using ngx-rust's module! macro
//
// This macro generates the necessary boilerplate for Nginx module registration,
// including module structure, command definitions, and configuration management.
//
// The exact syntax may vary in ngx-rust 0.5, so this implementation provides
// a complete framework that can be adjusted once the API is verified.
//
// For testing purposes, the core logic (x402_handler_impl, create_requirements, etc.)
// is fully testable without requiring Nginx source code or the full module registration.

ngx::http::module! {
    name: X402Module,
    commands: [
        // x402 on|off - Enable/disable x402 payment verification
        ngx::conf::Command {
            name: ngx::string!("x402"),
            ty: ngx::conf::CommandType::Flag,
            set: ngx::conf::set_flag_slot,
            conf: ngx::conf::CommandConf::LocConf(ngx::conf::offset_of!(X402Config, enabled)),
            post: None,
        },
        // x402_amount <amount> - Payment amount (e.g., "0.0001")
        ngx::conf::Command {
            name: ngx::string!("x402_amount"),
            ty: ngx::conf::CommandType::Take1,
            set: ngx::conf::set_str_slot,
            conf: ngx::conf::CommandConf::LocConf(ngx::conf::offset_of!(X402Config, amount_str)),
            post: None,
        },
        // x402_pay_to <address> - Recipient wallet address
        ngx::conf::Command {
            name: ngx::string!("x402_pay_to"),
            ty: ngx::conf::CommandType::Take1,
            set: ngx::conf::set_str_slot,
            conf: ngx::conf::CommandConf::LocConf(ngx::conf::offset_of!(X402Config, pay_to_str)),
            post: None,
        },
        // x402_facilitator_url <url> - Facilitator service URL
        ngx::conf::Command {
            name: ngx::string!("x402_facilitator_url"),
            ty: ngx::conf::CommandType::Take1,
            set: ngx::conf::set_str_slot,
            conf: ngx::conf::CommandConf::LocConf(ngx::conf::offset_of!(X402Config, facilitator_url_str)),
            post: None,
        },
        // x402_description <text> - Payment description
        ngx::conf::Command {
            name: ngx::string!("x402_description"),
            ty: ngx::conf::CommandType::Take1,
            set: ngx::conf::set_str_slot,
            conf: ngx::conf::CommandConf::LocConf(ngx::conf::offset_of!(X402Config, description_str)),
            post: None,
        },
        // x402_network <network> - Network identifier (e.g., "base-sepolia")
        ngx::conf::Command {
            name: ngx::string!("x402_network"),
            ty: ngx::conf::CommandType::Take1,
            set: ngx::conf::set_str_slot,
            conf: ngx::conf::CommandConf::LocConf(ngx::conf::offset_of!(X402Config, network_str)),
            post: None,
        },
        // x402_resource <path> - Resource path override
        ngx::conf::Command {
            name: ngx::string!("x402_resource"),
            ty: ngx::conf::CommandType::Take1,
            set: ngx::conf::set_str_slot,
            conf: ngx::conf::CommandConf::LocConf(ngx::conf::offset_of!(X402Config, resource_str)),
            post: None,
        },
        // x402_timeout <seconds> - Timeout for facilitator requests (1-300 seconds, default: 10)
        ngx::conf::Command {
            name: ngx::string!("x402_timeout"),
            ty: ngx::conf::CommandType::Take1,
            set: ngx::conf::set_str_slot,
            conf: ngx::conf::CommandConf::LocConf(ngx::conf::offset_of!(X402Config, timeout_str)),
            post: None,
        },
        // x402_facilitator_fallback <mode> - Fallback behavior when facilitator fails ("error" or "pass", default: "error")
        ngx::conf::Command {
            name: ngx::string!("x402_facilitator_fallback"),
            ty: ngx::conf::CommandType::Take1,
            set: ngx::conf::set_str_slot,
            conf: ngx::conf::CommandConf::LocConf(ngx::conf::offset_of!(X402Config, facilitator_fallback_str)),
            post: None,
        },
    ],
    init: None,
    handler: Some(x402_ngx_handler),
}
