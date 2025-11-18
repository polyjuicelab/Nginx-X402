//! Runtime and facilitator client management

use crate::ngx_module::error::{ConfigError, Result};
use rust_x402::facilitator::FacilitatorClient;
use rust_x402::types::FacilitatorConfig;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;
use tokio::time::timeout;

/// Global tokio runtime for async operations
pub static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();

/// Global facilitator client pool
///
/// Stores facilitator clients keyed by URL to enable reuse across requests.
/// Each URL gets its own client instance with connection pooling.
pub static FACILITATOR_CLIENTS: OnceLock<Mutex<HashMap<String, FacilitatorClient>>> =
    OnceLock::new();

/// Default timeout for facilitator requests (10 seconds)
pub const DEFAULT_FACILITATOR_TIMEOUT: Duration = Duration::from_secs(10);

/// Maximum size for X-PAYMENT header (64KB to prevent DoS)
pub const MAX_PAYMENT_HEADER_SIZE: usize = 64 * 1024;

/// Get or initialize the global tokio runtime
///
/// # Returns
/// - `Ok(&'static Runtime)` if runtime is available
/// - `Err` if runtime initialization failed
///
/// # Errors
/// - Returns error if runtime cannot be created (e.g., system resource exhaustion)
pub fn get_runtime() -> Result<&'static tokio::runtime::Runtime> {
    RUNTIME.get_or_init(|| {
        tokio::runtime::Runtime::new()
            .unwrap_or_else(|e| panic!("Failed to create tokio runtime: {}", e))
    });
    RUNTIME
        .get()
        .ok_or_else(|| ConfigError::from("Runtime not initialized"))
}

/// Get or create a facilitator client for the given URL
///
/// Uses a global pool to reuse clients across requests, improving performance
/// by avoiding repeated client creation and connection setup.
///
/// # Arguments
/// - `url`: Facilitator service URL
///
/// # Returns
/// - `Ok(&'static FacilitatorClient)` if client is available
/// - `Err` if client cannot be created or retrieved
pub fn get_facilitator_client(url: &str) -> Result<&'static FacilitatorClient> {
    let clients = FACILITATOR_CLIENTS.get_or_init(|| Mutex::new(HashMap::new()));

    // Check if client already exists
    {
        let guard = clients
            .lock()
            .map_err(|_| ConfigError::from("Lock poisoned"))?;
        if let Some(client) = guard.get(url) {
            // Return a reference to the existing client
            // SAFETY: The client is stored in a static OnceLock, so it lives for 'static
            // We need to use unsafe to convert the reference, but the client is guaranteed
            // to live as long as the program runs.
            return unsafe { Ok(&*(client as *const FacilitatorClient)) };
        }
    }

    // Create new client
    let config = FacilitatorConfig::new(url);
    let client = FacilitatorClient::new(config)
        .map_err(|e| ConfigError::from(format!("Failed to create facilitator client: {}", e)))?;

    // Store in pool
    {
        let mut guard = clients
            .lock()
            .map_err(|_| ConfigError::from("Lock poisoned"))?;
        guard.insert(url.to_string(), client);
    }

    // Retrieve the stored client
    let guard = clients
        .lock()
        .map_err(|_| ConfigError::from("Lock poisoned"))?;
    guard
        .get(url)
        .map(|client| unsafe { &*(client as *const FacilitatorClient) })
        .ok_or_else(|| ConfigError::from("Failed to retrieve facilitator client"))
}

/// Verify payment with facilitator service
///
/// # Arguments
/// - `payment_b64`: Base64-encoded payment payload
/// - `requirements`: Payment requirements to verify against
/// - `facilitator_url`: Facilitator service URL
/// - `timeout`: Optional timeout (uses default if None)
///
/// # Returns
/// - `Ok(true)` if payment is valid
/// - `Ok(false)` if payment is invalid
/// - `Err` if verification fails (network error, timeout, etc.)
pub async fn verify_payment(
    payment_b64: &str,
    requirements: &rust_x402::types::PaymentRequirements,
    facilitator_url: &str,
    timeout_duration: Option<Duration>,
) -> Result<bool> {
    use crate::ngx_module::error::user_errors;
    use crate::ngx_module::logging::{log_error, log_warn};
    use rust_x402::types::PaymentPayload;

    // Validate inputs
    if payment_b64.is_empty() {
        return Err(ConfigError::from(user_errors::INVALID_PAYMENT));
    }
    if facilitator_url.is_empty() {
        return Err(ConfigError::from(user_errors::CONFIGURATION_ERROR));
    }

    // Parse payment payload - use generic error for users
    let payment_payload = PaymentPayload::from_base64(payment_b64).map_err(|e| {
        // Log internal error details
        log_error(None, &format!("Failed to parse payment payload: {}", e));
        // User gets generic error
        ConfigError::from(user_errors::INVALID_PAYMENT)
    })?;

    // Get facilitator client
    let client = get_facilitator_client(facilitator_url)?;

    // Use configured timeout or default
    let timeout_duration = timeout_duration.unwrap_or(DEFAULT_FACILITATOR_TIMEOUT);

    // Verify with timeout
    let verify_future = client.verify(&payment_payload, requirements);
    match timeout(timeout_duration, verify_future).await {
        Ok(Ok(response)) => Ok(response.is_valid),
        Ok(Err(e)) => {
            // Verification failure - log internal details, user gets generic error
            log_error(None, &format!("Payment verification failed: {}", e));
            Err(ConfigError::from(user_errors::PAYMENT_VERIFICATION_FAILED))
        }
        Err(_) => {
            // Timeout - log and return user-facing error
            log_warn(
                None,
                &format!("Payment verification timeout after {:?}", timeout_duration),
            );
            Err(ConfigError::from(user_errors::TIMEOUT))
        }
    }
}
