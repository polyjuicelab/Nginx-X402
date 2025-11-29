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
pub mod phase_handler;
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
pub use request::{
    get_header_value, get_http_method, is_browser_request, should_skip_payment_for_method,
};
pub use requirements::create_requirements;
pub use response::{send_402_response, send_options_response, send_response_body};
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

/// Clear x402 content handler if it's set
///
/// This helper function clears the content handler if it's set to `x402_ngx_handler`.
/// This prevents duplicate payment verification in CONTENT_PHASE when we've already
/// handled the request in ACCESS_PHASE (e.g., for OPTIONS/HEAD/TRACE requests or
/// when payment is already verified).
///
/// # Safety
///
/// The caller must ensure that `r` is a valid pointer to a `ngx_http_request_t`.
unsafe fn clear_x402_content_handler(
    r: *mut ngx::ffi::ngx_http_request_t,
    req_mut: &mut ngx::http::Request,
    reason: &str,
) {
    use ngx::ffi::ngx_http_request_t;
    let r_raw = r.cast::<ngx_http_request_t>();
    if r_raw.is_null() {
        return;
    }

    extern "C" {
        fn x402_ngx_handler(r: *mut ngx::ffi::ngx_http_request_t) -> ngx::ffi::ngx_int_t;
    }
    let x402_handler_fn: ngx::ffi::ngx_http_handler_pt = Some(x402_ngx_handler);
    let current_handler = (*r_raw).content_handler;

    // Check if content handler is x402_ngx_handler
    let is_x402_handler = if let (Some(current), Some(x402)) = (current_handler, x402_handler_fn) {
        std::ptr::fn_addr_eq(current, x402)
    } else {
        false
    };

    if is_x402_handler {
        // Clear content handler to prevent payment verification in CONTENT_PHASE
        (*r_raw).content_handler = None;
        use crate::ngx_module::logging::log_debug;
        log_debug(
            Some(req_mut),
            &format!(
                "[x402] Phase handler: Cleared x402 content handler {}",
                reason
            ),
        );
    }
}

/// Phase handler for ACCESS phase
///
/// This handler is registered as a phase handler in `ACCESS_PHASE` (before `CONTENT_PHASE`).
/// This ensures payment verification happens BEFORE `proxy_pass` or other content handlers
/// set their handlers, allowing x402 to work correctly even when `proxy_pass` is configured.
///
/// The handler:
/// 1. Checks if the module is enabled for the current location
/// 2. If enabled, performs payment verification
/// 3. If payment is valid, returns `NGX_OK` to allow request to proceed
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
    use crate::ngx_module::request::{
        get_http_method, is_websocket_request, should_skip_payment_for_method,
    };

    // Skip payment verification for special request types
    // These requests should bypass payment verification:
    // 1. Certain HTTP methods (OPTIONS, HEAD, TRACE) - used for protocol-level operations
    //    - OPTIONS: CORS preflight requests sent by browsers before cross-origin requests
    //    - HEAD: Used to check resource existence without retrieving body
    //    - TRACE: Used for diagnostic and debugging purposes
    // 2. WebSocket upgrades - long-lived connections that use special HTTP Upgrade mechanism.
    //    Payment verification would interfere with WebSocket handshake, and subsequent
    //    WebSocket frames are not HTTP requests, so payment verification is not applicable.
    //    Payment should be handled at application layer for WebSocket connections.
    // 3. Subrequests (auth_request, etc.) - detected via raw request pointer
    // 4. Internal redirects - detected via raw request pointer

    // Check if HTTP method should skip payment verification
    // This check happens early to avoid unnecessary processing
    let request_struct = unsafe { &*r };
    let method_id = request_struct.method;
    let detected_method = unsafe { get_http_method(r) };

    // Debug: Log method ID and detected method for troubleshooting
    log_debug(
        Some(req_mut),
        &format!(
            "[x402] Phase handler: method_id=0x{:08x}, detected_method={:?}",
            method_id, detected_method
        ),
    );

    if unsafe { should_skip_payment_for_method(r) } {
        let method = detected_method.unwrap_or("UNKNOWN");
        log_debug(
            Some(req_mut),
            &format!(
                "[x402] Phase handler: {} request detected (method_id=0x{:08x}), skipping payment verification",
                method, method_id
            ),
        );

        // Handle special HTTP methods that should skip payment verification
        use crate::ngx_module::phase_handler::handle_skip_payment_method;
        if let Some(result_code) =
            unsafe { handle_skip_payment_method(r, req_mut, method, clear_x402_content_handler) }
        {
            return result_code;
        }
        // If handle_skip_payment_method returned None, fall through to decline
        return ngx::ffi::NGX_DECLINED as ngx::ffi::ngx_int_t;
    }

    // Check for WebSocket upgrade (can be detected via headers)
    if is_websocket_request(req_mut) {
        log_debug(
            Some(req_mut),
            "[x402] Phase handler: WebSocket upgrade detected, skipping payment verification",
        );
        // Clear content handler if it's x402_ngx_handler to prevent payment verification in CONTENT_PHASE
        unsafe {
            clear_x402_content_handler(
                r,
                req_mut,
                "for WebSocket request to prevent payment verification",
            );
        }
        return ngx::ffi::NGX_DECLINED as ngx::ffi::ngx_int_t;
    }

    // Check for subrequest using raw request pointer
    // Subrequests have r->parent != NULL
    unsafe {
        let r_raw = r.cast_const();
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
        let r_raw = r.cast_const();
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
                let uri_slice = std::slice::from_raw_parts(uri.data.cast_const(), uri.len.min(1));
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
            // If content handler is x402_ngx_handler (no proxy_pass), clear it to prevent
            // duplicate payment verification in CONTENT_PHASE. If content handler is something
            // else (like proxy_pass), keep it so it runs in CONTENT_PHASE.
            unsafe {
                clear_x402_content_handler(
                    r,
                    req_mut,
                    "after payment verification to prevent duplicate verification",
                );
            }
            // This will proceed to CONTENT_PHASE where proxy_pass handler will run (if set)
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
/// for locations where `x402 on;` is configured WITHOUT `proxy_pass`.
///
/// **Note**: When `proxy_pass` is configured, payment verification happens in `ACCESS_PHASE`
/// via `x402_phase_handler`, and this content handler should not be called (it gets cleared
/// in phase handler to prevent duplicate verification).
///
/// This handler converts the raw nginx request pointer to a Rust Request object and
/// delegates to the implementation function.
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
