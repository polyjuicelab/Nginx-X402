//! Request handler implementation

use crate::config::validate_payment_header;
use crate::ngx_module::config::{FacilitatorFallback, ParsedX402Config};
use crate::ngx_module::error::{user_errors, ConfigError, Result};
use crate::ngx_module::logging::{log_debug, log_error, log_info, log_warn};
use crate::ngx_module::metrics::X402Metrics;
use crate::ngx_module::module::get_module_config;
use crate::ngx_module::request::get_header_value;
use crate::ngx_module::requirements::create_requirements;
use crate::ngx_module::response::{send_402_response, send_response_body};
use crate::ngx_module::runtime::{get_runtime, verify_payment};
use ngx::core::Status;
use ngx::http::{HTTPStatus, Request};
use std::time::Instant;

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
/// Core payment verification handler implementation
///
/// This function contains the main business logic for payment verification.
/// It is called by `x402_ngx_handler_impl` after configuration parsing.
///
/// # Errors
/// - Returns error if payment requirements cannot be created
/// - Returns error if facilitator URL is not configured
/// - Returns error if payment verification fails
/// - Returns error if 402 response cannot be sent
pub fn x402_handler_impl(r: &mut Request, config: &ParsedX402Config) -> Result<()> {
    // Record request metric
    let metrics = X402Metrics::get();
    metrics.record_request();

    if !config.enabled {
        return Ok(()); // Module disabled, pass through
    }

    let resource = config
        .resource
        .as_deref()
        .or_else(|| r.path().to_str().ok())
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
    // Create slice reference for send_402_response (supports multiple requirements)
    let requirements_slice = std::slice::from_ref(&requirements);

    // Record payment amount metric (convert from smallest units to decimal units)
    if let Ok(amount_decimal) = requirements.amount_in_decimal_units(6) {
        // Convert Decimal to f64 for metrics
        // Use to_f64_retain() to preserve precision, or fallback to to_f64()
        if let Some(amount_f64) = amount_decimal.to_f64() {
            metrics.record_payment_amount(amount_f64);
        }
    }

    // Check for X-PAYMENT header
    let payment_header = get_header_value(r, "X-PAYMENT");

    if let Some(payment_b64) = payment_header {
        log_debug(
            Some(r),
            "X-PAYMENT header found, validating and verifying payment",
        );

        // Record verification attempt
        metrics.record_verification_attempt();

        // Validate payment header format and size
        validate_payment_header(&payment_b64).map_err(|e| {
            log_warn(Some(r), &format!("Invalid payment header format: {}", e));
            ConfigError::from(e)
        })?;

        // Verify payment
        let facilitator_url = config.facilitator_url.as_deref().ok_or_else(|| {
            log_error(Some(r), "Facilitator URL not configured");
            ConfigError::from("Facilitator URL not configured")
        })?;

        // Block on async verification
        // Use configured timeout or default
        let timeout = config.timeout;
        let runtime = get_runtime()?;
        let verification_start = Instant::now();
        let verification_result = runtime.block_on(async {
            verify_payment(&payment_b64, &requirements, facilitator_url, timeout).await
        });
        let verification_duration = verification_start.elapsed().as_secs_f64();

        // Record verification duration
        metrics.record_verification_duration(verification_duration);

        // Handle verification result with fallback logic
        let is_valid = match verification_result {
            Ok(valid) => valid,
            Err(e) => {
                // Facilitator verification failed (network error, timeout, etc.)
                log_error(Some(r), &format!("Facilitator verification error: {}", e));
                metrics.record_facilitator_error();
                match config.facilitator_fallback {
                    FacilitatorFallback::Error => {
                        // Return 500 error
                        r.set_status(
                            HTTPStatus::from_u16(500)
                                .map_err(|_| ConfigError::from("Invalid status code"))?,
                        );
                        r.add_header_out("Content-Type", "text/plain; charset=utf-8")
                            .ok_or_else(|| {
                                ConfigError::from("Failed to set Content-Type header")
                            })?;
                        send_response_body(r, b"Internal server error")?;
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
            metrics.record_verification_success();
            Ok(())
        } else {
            // Payment invalid - send user-facing error message
            log_warn(Some(r), "Payment verification failed, sending 402 response");
            metrics.record_verification_failed();
            metrics.record_402_response();
            send_402_response(
                r,
                requirements_slice,
                config,
                Some(user_errors::PAYMENT_VERIFICATION_FAILED),
            )?;
            Ok(())
        }
    } else {
        // No payment header, send 402
        log_debug(Some(r), "No X-PAYMENT header found, sending 402 response");
        metrics.record_402_response();
        send_402_response(r, requirements_slice, config, None)?;
        Ok(())
    }
}

/// Request handler wrapper for ngx-rust
///
/// This function wraps the core payment verification logic and adapts it
/// to ngx-rust's request handler interface.
///
/// # Configuration Access
///
/// This implementation attempts to get module configuration from the request.
/// The exact API depends on ngx-rust 0.5's actual implementation.
///
/// This function is called by the `http_request_handler!` macro-generated `x402_ngx_handler`.
pub fn x402_ngx_handler_impl(req: &mut Request) -> Status {
    // Get module configuration from request
    let conf = match get_module_config(req) {
        Ok(c) => c,
        Err(e) => {
            log_error(Some(req), &format!("Failed to get module config: {}", e));
            return Status::NGX_ERROR;
        }
    };

    let parsed_config = match conf.parse() {
        Ok(c) => c,
        Err(e) => {
            log_error(Some(req), &format!("Failed to parse config: {}", e));
            return Status::NGX_ERROR;
        }
    };

    // Call the core handler
    match x402_handler_impl(req, &parsed_config) {
        Ok(()) => Status::NGX_OK,
        Err(e) => {
            log_error(Some(req), &format!("Handler error: {}", e));
            Status::NGX_ERROR
        }
    }
}

/// Metrics handler for exposing Prometheus metrics
///
/// This handler exposes Prometheus metrics via a `/metrics` endpoint.
/// It should be registered as a separate handler for a dedicated location.
///
/// # Usage
///
/// In Nginx configuration:
/// ```nginx
/// location /metrics {
///     x402_metrics on;
/// }
/// ```
pub fn x402_metrics_handler_impl(req: &mut Request) -> Status {
    use crate::ngx_module::metrics::collect_metrics;

    // Collect metrics in Prometheus text format
    let metrics_text = collect_metrics();

    // Set response headers
    if req
        .add_header_out("Content-Type", "text/plain; version=0.0.4; charset=utf-8")
        .is_none()
    {
        log_error(Some(req), "Failed to set Content-Type header for metrics");
        return Status::NGX_ERROR;
    }

    // Set status to 200 OK
    // HTTP 200 is always valid, but handle error case gracefully
    if let Ok(status) = HTTPStatus::from_u16(200) {
        req.set_status(status);
    } else {
        log_error(Some(req), "Failed to set status code 200 for metrics");
        return Status::NGX_ERROR;
    }

    // Send metrics response
    match send_response_body(req, metrics_text.as_bytes()) {
        Ok(()) => Status::NGX_OK,
        Err(e) => {
            log_error(
                Some(req),
                &format!("Failed to send metrics response: {}", e),
            );
            Status::NGX_ERROR
        }
    }
}
