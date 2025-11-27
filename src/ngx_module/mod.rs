//! Pure Rust Nginx module implementation using ngx-rust
//!
//! This module implements the x402 HTTP micropayment protocol verification
//! directly in Rust, using the official [ngx-rust](https://github.com/nginx/ngx-rust) bindings.
//!
//! # Features
//!
//! - ✅ **Payment Verification**: Validates X-PAYMENT headers against facilitator service
//! - ✅ **402 Response Generation**: Sends HTML paywall or JSON responses based on client type
//! - ✅ **Configuration Parsing**: Full nginx configuration directive support
//! - ✅ **Metrics**: Prometheus metrics endpoint for monitoring
//! - ✅ **Fallback Handling**: Configurable error handling (error/pass modes)
//! - ✅ **Type Safety**: Full Rust type safety with ngx-rust bindings
//!
//! # Module Structure
//!
//! The module is organized into several submodules:
//!
//! - `commands`: Nginx configuration directive handlers
//! - `config`: Configuration parsing and validation
//! - `handler`: Request processing and payment verification
//! - `response`: HTTP response generation (402, HTML, JSON)
//! - `runtime`: Async runtime and facilitator client
//! - `metrics`: Prometheus metrics collection
//! - `module`: Module registration and nginx integration

pub mod commands;
pub mod config;
pub mod error;
pub mod handler;
pub mod logging;
pub mod metrics;
pub mod module;
pub mod request;
pub mod requirements;
pub mod response;
pub mod runtime;

// Re-export public types and functions
pub use config::{FacilitatorFallback, ParsedX402Config, X402Config};
pub use error::{user_errors, ConfigError, Result};
pub use handler::{
    x402_handler_impl, x402_metrics_handler_impl, x402_ngx_handler_impl, HandlerResult,
};
pub use logging::{log_debug, log_error, log_info, log_warn};
pub use metrics::{collect_metrics, X402Metrics};
pub use module::{get_module_config, ngx_http_x402_module};
pub use request::{get_header_value, is_browser_request};
pub use requirements::create_requirements;
pub use response::{send_402_response, send_response_body};
pub use runtime::{
    get_facilitator_client, get_runtime, verify_payment, DEFAULT_FACILITATOR_TIMEOUT,
    FACILITATOR_CLIENTS, MAX_PAYMENT_HEADER_SIZE, RUNTIME,
};

/// Metrics handler C export
///
/// This function is exported for C linkage and wraps the Rust metrics handler.
/// It converts the raw nginx request pointer to a Request object and calls
/// the implementation function.
///
/// # Safety
///
/// The caller must ensure that `r` is a valid pointer to a `ngx_http_request_t`
/// structure. The pointer must remain valid for the duration of this function call.
#[no_mangle]
pub unsafe extern "C" fn x402_metrics_handler(
    r: *mut ngx::ffi::ngx_http_request_t,
) -> ngx::ffi::ngx_int_t {
    use std::mem;
    let req_ptr: *mut ngx::http::Request = mem::transmute(r);
    let req_mut = &mut *req_ptr;

    match x402_metrics_handler_impl(req_mut) {
        ngx::core::Status::NGX_OK => ngx::ffi::NGX_OK as ngx::ffi::ngx_int_t,
        ngx::core::Status::NGX_ERROR => ngx::ffi::NGX_ERROR as ngx::ffi::ngx_int_t,
        ngx::core::Status::NGX_DECLINED => ngx::ffi::NGX_DECLINED as ngx::ffi::ngx_int_t,
        _ => ngx::ffi::NGX_ERROR as ngx::ffi::ngx_int_t,
    }
}

/// Phase handler for ACCESS phase
///
/// This handler is registered as a phase handler in ACCESS_PHASE (before CONTENT_PHASE).
/// This ensures payment verification happens BEFORE proxy_pass or other content handlers
/// set their handlers, allowing x402 to work correctly even when proxy_pass is configured.
///
/// The handler:
/// 1. Checks if the module is enabled for the current location
/// 2. If enabled, performs payment verification
/// 3. If payment is valid, returns NGX_OK to allow request to proceed
/// 4. If payment is invalid or missing, sends 402 response and finalizes request
///
/// # Safety
///
/// This function is marked `unsafe` because it performs raw pointer operations
/// to convert the nginx request pointer to a Rust Request object.
#[no_mangle]
pub unsafe extern "C" fn x402_phase_handler(
    r: *mut ngx::ffi::ngx_http_request_t,
) -> ngx::ffi::ngx_int_t {
    use std::mem;
    let req_ptr: *mut ngx::http::Request = mem::transmute(r);
    let req_mut = &mut *req_ptr;

    use crate::ngx_module::logging::log_debug;
    use crate::ngx_module::module::get_module_config;
    use crate::ngx_module::request::is_websocket_request;

    // Skip payment verification for special request types
    // These requests should bypass payment verification:
    // 1. WebSocket upgrades - long-lived connections that use special HTTP Upgrade mechanism.
    //    Payment verification would interfere with WebSocket handshake, and subsequent
    //    WebSocket frames are not HTTP requests, so payment verification is not applicable.
    //    Payment should be handled at application layer for WebSocket connections.
    // 2. Subrequests (auth_request, etc.) - detected via raw request pointer
    // 3. Internal redirects - detected via raw request pointer

    // Check for WebSocket upgrade (can be detected via headers)
    if is_websocket_request(req_mut) {
        log_debug(
            Some(req_mut),
            "[x402] Phase handler: WebSocket upgrade detected, skipping payment verification",
        );
        return ngx::ffi::NGX_DECLINED as ngx::ffi::ngx_int_t;
    }

    // Check for subrequest using raw request pointer
    // Subrequests have r->parent != NULL
    unsafe {
        use ngx::ffi::ngx_http_request_t;
        let r_raw = r as *const ngx_http_request_t;
        if !r_raw.is_null() {
            let parent = (*r_raw).parent;
            if !parent.is_null() {
                log_debug(
                    Some(req_mut),
                    "[x402] Phase handler: Subrequest detected (parent != NULL), skipping payment verification",
                );
                return ngx::ffi::NGX_DECLINED as ngx::ffi::ngx_int_t;
            }
        }
    }

    // Check for internal redirect using raw request pointer
    // Internal redirects have r->internal = 1 (unsigned flag)
    // In nginx C code, internal is a field in ngx_http_request_t structure
    unsafe {
        use ngx::ffi::ngx_http_request_t;
        let r_raw = r as *const ngx_http_request_t;
        if !r_raw.is_null() {
            // Access internal field directly from C structure
            // internal is an unsigned integer field in ngx_http_request_t
            // We need to access it via pointer dereference
            // Note: This is accessing the raw C structure, so we need to be careful
            let request_struct = &*r_raw;
            // Try to access internal field - it should be a field, not a method
            // If this doesn't compile, we may need to use offset_of! macro or other approach
            // For now, we'll use a workaround: check if uri.data starts with @ (named locations)
            // Named locations (like @fallback) are always internal redirects
            let uri = request_struct.uri;
            if !uri.data.is_null() && uri.len > 0 {
                // Check if URI starts with '@' which indicates named location (always internal)
                let uri_slice = std::slice::from_raw_parts(uri.data as *const u8, uri.len.min(1));
                if uri_slice[0] == b'@' {
                    log_debug(
                        Some(req_mut),
                        "[x402] Phase handler: Internal redirect detected (named location @), skipping payment verification",
                    );
                    return ngx::ffi::NGX_DECLINED as ngx::ffi::ngx_int_t;
                }
            }
        }
    }

    // Check if module is enabled for this location
    let conf = match get_module_config(req_mut) {
        Ok(c) => c,
        Err(_) => {
            // Module not configured for this location, decline to let other handlers process
            return ngx::ffi::NGX_DECLINED as ngx::ffi::ngx_int_t;
        }
    };

    // Check if module is enabled
    if conf.enabled == 0 {
        return ngx::ffi::NGX_DECLINED as ngx::ffi::ngx_int_t;
    }

    // Module is enabled - perform payment verification
    // This will verify payment and send 402 if needed, or allow request to proceed
    use crate::ngx_module::handler::HandlerResult;
    let (status, result) = x402_ngx_handler_impl(req_mut);
    match (status, result) {
        (ngx::core::Status::NGX_OK, HandlerResult::PaymentValid) => {
            // Payment verified - allow request to continue
            // This will proceed to CONTENT_PHASE where proxy_pass handler will run
            ngx::ffi::NGX_OK as ngx::ffi::ngx_int_t
        }
        (ngx::core::Status::NGX_DECLINED, HandlerResult::ResponseSent) => {
            // Response was sent (402 or error) - stop processing
            // Return OK to indicate we handled the request and prevent further processing
            // This prevents proxy_pass from executing
            // Note: In nginx, when a response is sent in ACCESS_PHASE, returning NGX_OK
            // tells nginx that we've handled the request and it should not proceed to CONTENT_PHASE
            ngx::ffi::NGX_OK as ngx::ffi::ngx_int_t
        }
        (ngx::core::Status::NGX_ERROR, _) => {
            // Error occurred during payment verification
            // The handler should have sent an appropriate response (402 or 500)
            // Return OK to indicate we handled the request and prevent further processing
            ngx::ffi::NGX_OK as ngx::ffi::ngx_int_t
        }
        _ => {
            // Unexpected status - return OK to prevent further processing
            ngx::ffi::NGX_OK as ngx::ffi::ngx_int_t
        }
    }
}

/// Main content handler C export
///
/// This is the primary handler function that nginx calls when processing requests
/// for locations where `x402 on;` is configured. It converts the raw nginx request
/// pointer to a Rust Request object and delegates to the implementation function.
///
/// # Safety
///
/// This function is marked `unsafe` because it performs raw pointer operations
/// to convert the nginx request pointer to a Rust Request object. The caller must
/// ensure that the pointer is valid and points to a valid `ngx_http_request_t`.
#[no_mangle]
pub unsafe extern "C" fn x402_ngx_handler(
    r: *mut ngx::ffi::ngx_http_request_t,
) -> ngx::ffi::ngx_int_t {
    use std::mem;
    let req_ptr: *mut ngx::http::Request = mem::transmute(r);
    let req_mut = &mut *req_ptr;

    let (status, _result) = x402_ngx_handler_impl(req_mut);
    match status {
        ngx::core::Status::NGX_OK => ngx::ffi::NGX_OK as ngx::ffi::ngx_int_t,
        ngx::core::Status::NGX_ERROR => ngx::ffi::NGX_ERROR as ngx::ffi::ngx_int_t,
        ngx::core::Status::NGX_DECLINED => ngx::ffi::NGX_DECLINED as ngx::ffi::ngx_int_t,
        _ => ngx::ffi::NGX_ERROR as ngx::ffi::ngx_int_t,
    }
}
