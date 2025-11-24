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
pub use handler::{x402_handler_impl, x402_metrics_handler_impl, x402_ngx_handler_impl};
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
#[no_mangle]
pub unsafe extern "C" fn x402_metrics_handler(r: *mut ngx::ffi::ngx_http_request_t) -> ngx::ffi::ngx_int_t {
    use std::mem;
    let req_ptr: *mut ngx::http::Request = mem::transmute(r);
    let req_mut = &mut *req_ptr;
    
    match x402_metrics_handler_impl(req_mut) {
        ngx::core::Status::NGX_OK => ngx::ffi::NGX_OK as ngx::ffi::ngx_int_t,
        ngx::core::Status::NGX_ERROR => ngx::ffi::NGX_ERROR as ngx::ffi::ngx_int_t,
        ngx::core::Status::NGX_DECLINED => ngx::ffi::NGX_DECLINED as ngx::ffi::ngx_int_t,
    }
}

/// Phase handler for CONTENT phase
///
/// This handler is registered as a phase handler in the postconfiguration hook.
/// It serves as a fallback mechanism if the content handler (`clcf->handler`) is not
/// set or was overwritten during configuration merging.
///
/// The handler checks:
/// 1. If `content_handler` is already set (decline to let nginx call it)
/// 2. If the module is enabled for the current location
/// 3. If enabled, calls the main handler directly
///
/// # Safety
///
/// This function is marked `unsafe` because it performs raw pointer operations
/// to convert the nginx request pointer to a Rust Request object.
#[no_mangle]
pub unsafe extern "C" fn x402_phase_handler(r: *mut ngx::ffi::ngx_http_request_t) -> ngx::ffi::ngx_int_t {
    unsafe {
        use std::mem;
        let req_ptr: *mut ngx::http::Request = mem::transmute(r);
        let req_mut = &mut *req_ptr;
        
        use crate::ngx_module::logging::log_debug;
        use crate::ngx_module::module::get_module_config;
        
        let r_raw = req_mut.as_ref();
        
        // Phase handler should only be called if content_handler is NOT set
        // If content_handler is already set, decline and let nginx call the content handler
        if r_raw.content_handler.is_some() {
            log_debug(Some(req_mut), "[x402] Phase handler: content_handler already set, declining");
            return ngx::ffi::NGX_DECLINED as ngx::ffi::ngx_int_t;
        }
        
        // Check if module is enabled for this location
        let conf = match get_module_config(req_mut) {
            Ok(c) => c,
            Err(_) => {
                // Module not configured for this location, decline
                return ngx::ffi::NGX_DECLINED as ngx::ffi::ngx_int_t;
            }
        };
        
        // Check if module is enabled
        if conf.enabled == 0 {
            return ngx::ffi::NGX_DECLINED as ngx::ffi::ngx_int_t;
        }
        
        // Module is enabled but content_handler is not set (fallback case)
        // Call the handler directly as a fallback
        x402_ngx_handler(r)
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
pub unsafe extern "C" fn x402_ngx_handler(r: *mut ngx::ffi::ngx_http_request_t) -> ngx::ffi::ngx_int_t {
    unsafe {
        use std::mem;
        let req_ptr: *mut ngx::http::Request = mem::transmute(r);
        let req_mut = &mut *req_ptr;
        
        match x402_ngx_handler_impl(req_mut) {
            ngx::core::Status::NGX_OK => ngx::ffi::NGX_OK as ngx::ffi::ngx_int_t,
            ngx::core::Status::NGX_ERROR => ngx::ffi::NGX_ERROR as ngx::ffi::ngx_int_t,
            ngx::core::Status::NGX_DECLINED => ngx::ffi::NGX_DECLINED as ngx::ffi::ngx_int_t,
            _ => ngx::ffi::NGX_ERROR as ngx::ffi::ngx_int_t,
        }
    }
}
