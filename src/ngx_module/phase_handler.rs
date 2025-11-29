//! Phase handler utilities for x402 module
//!
//! This module contains helper functions for the ACCESS_PHASE handler,
//! including proxy_pass detection and special request handling.

use crate::ngx_module::logging::{log_debug, log_error};
use ngx::ffi::{ngx_http_request_t, ngx_int_t};
use ngx::http::{HTTPStatus, Request};

/// Check if proxy_pass is configured for the current location
///
/// In ACCESS_PHASE, proxy_pass's content_handler may not be set yet,
/// so we check the upstream field instead (proxy_pass sets this).
///
/// # Safety
///
/// The caller must ensure that `r` is a valid pointer to a `ngx_http_request_t`.
#[must_use]
pub unsafe fn has_proxy_pass(r: *mut ngx_http_request_t) -> bool {
    let r_raw = r.cast::<ngx_http_request_t>();
    if r_raw.is_null() {
        return false;
    }

    let request_struct = &*r_raw;

    // Check if upstream is set (proxy_pass sets this)
    // upstream is a raw pointer field in ngx_http_request_t
    // If it's not null, proxy_pass is likely configured
    if !request_struct.upstream.is_null() {
        return true;
    }

    // Also check content_handler as fallback
    // In some cases, proxy_pass may have already set its handler
    let current_handler = request_struct.content_handler;
    if let Some(handler) = current_handler {
        extern "C" {
            fn x402_ngx_handler(r: *mut ngx_http_request_t) -> ngx_int_t;
        }
        let x402_handler_fn: ngx::ffi::ngx_http_handler_pt = Some(x402_ngx_handler);
        if let Some(x402) = x402_handler_fn {
            !std::ptr::fn_addr_eq(handler, x402)
        } else {
            false // Can't determine, assume no proxy_pass
        }
    } else {
        false // No upstream and no handler, no proxy_pass
    }
}

/// Handle OPTIONS request when no proxy_pass is configured
///
/// Sends a 204 No Content response for OPTIONS requests.
///
/// # Returns
///
/// * `Ok(ngx_int_t)` - Successfully sent response (NGX_OK)
/// * `Err(ngx_int_t)` - Failed to send response (NGX_DECLINED)
pub fn handle_options_request(req_mut: &mut Request) -> Result<ngx_int_t, ngx_int_t> {
    use crate::ngx_module::response::send_options_response;

    match send_options_response(req_mut) {
        Ok(_) => {
            log_debug(
                Some(req_mut),
                "[x402] OPTIONS request: sent 204 response (no proxy_pass, CORS headers should be handled by nginx config)",
            );
            Ok(ngx::ffi::NGX_OK as ngx_int_t)
        }
        Err(e) => {
            log_error(
                Some(req_mut),
                &format!("[x402] Failed to send OPTIONS response: {e}"),
            );
            Err(ngx::ffi::NGX_DECLINED as ngx_int_t)
        }
    }
}

/// Handle HEAD or TRACE request when no proxy_pass is configured
///
/// Sends appropriate response:
/// - HEAD: 200 OK with no body (standard behavior)
/// - TRACE: 405 Method Not Allowed (many servers disable TRACE for security)
///
/// # Arguments
///
/// * `req_mut` - Nginx request object
/// * `method` - HTTP method ("HEAD" or "TRACE")
///
/// # Returns
///
/// * `Ok(ngx_int_t)` - Successfully sent response (NGX_OK)
/// * `Err(ngx_int_t)` - Failed to send response (NGX_DECLINED)
pub fn handle_head_or_trace_request(
    req_mut: &mut Request,
    method: &str,
) -> Result<ngx_int_t, ngx_int_t> {
    // Determine status code based on method
    let status_code = if method == "HEAD" {
        200 // HEAD should return 200 OK with no body
    } else {
        405 // TRACE is often disabled, return Method Not Allowed
    };

    // Create HTTP status
    let http_status = match HTTPStatus::from_u16(status_code) {
        Ok(status) => status,
        Err(_) => {
            log_error(
                Some(req_mut),
                &format!(
                    "[x402] Failed to create HTTP status {} for {} request",
                    status_code, method
                ),
            );
            return Err(ngx::ffi::NGX_DECLINED as ngx_int_t);
        }
    };

    // Set status and content length
    req_mut.set_status(http_status);
    req_mut.set_content_length_n(0);

    // Send header
    let send_status = req_mut.send_header();
    if send_status == ngx::core::Status::NGX_OK {
        log_debug(
            Some(req_mut),
            &format!(
                "[x402] {} request: sent {} response (no proxy_pass)",
                method, status_code
            ),
        );
        Ok(ngx::ffi::NGX_OK as ngx_int_t)
    } else {
        log_error(
            Some(req_mut),
            &format!(
                "[x402] Failed to send {} response header: {:?}",
                method, send_status
            ),
        );
        Err(ngx::ffi::NGX_DECLINED as ngx_int_t)
    }
}

/// Handle special HTTP methods (OPTIONS, HEAD, TRACE) that should skip payment
///
/// This function handles requests for methods that should bypass payment verification.
/// If proxy_pass is configured, it forwards the request to the backend.
/// Otherwise, it sends an appropriate response directly.
///
/// # Safety
///
/// The caller must ensure that `r` is a valid pointer to a `ngx_http_request_t`.
///
/// # Arguments
///
/// * `r` - Raw nginx request pointer
/// * `req_mut` - Rust Request object
/// * `method` - HTTP method string
/// * `clear_handler_fn` - Function to clear the x402 content handler
///
/// # Returns
///
/// * `Some(ngx_int_t)` - Response code if request was handled
/// * `None` - Request should be declined (fall through)
pub unsafe fn handle_skip_payment_method(
    r: *mut ngx_http_request_t,
    req_mut: &mut Request,
    method: &str,
    clear_handler_fn: unsafe fn(*mut ngx_http_request_t, &mut Request, &str),
) -> Option<ngx_int_t> {
    // Check if proxy_pass is configured
    if has_proxy_pass(r) {
        // Has proxy_pass - forward request to backend so it can handle CORS headers
        log_debug(
            Some(req_mut),
            &format!(
                "[x402] {} request: forwarding to backend (proxy_pass configured, backend should handle CORS)",
                method
            ),
        );
        // Clear x402 content handler to prevent duplicate verification
        clear_handler_fn(
            r,
            req_mut,
            &format!("for {} request to forward to backend", method),
        );
        return Some(ngx::ffi::NGX_DECLINED as ngx_int_t);
    }

    // No proxy_pass - send response directly to prevent empty responses
    match method {
        "OPTIONS" => {
            match handle_options_request(req_mut) {
                Ok(code) => return Some(code),
                Err(_) => {
                    // Fall through to decline if response sending fails
                }
            }
        }
        "HEAD" | "TRACE" => {
            match handle_head_or_trace_request(req_mut, method) {
                Ok(code) => return Some(code),
                Err(_) => {
                    // Fall through to decline if response sending fails
                }
            }
        }
        _ => {
            // Unknown method, should not happen
            log_error(
                Some(req_mut),
                &format!("[x402] Unknown skip-payment method: {}", method),
            );
        }
    }

    // Clear content handler if it's x402_ngx_handler to prevent payment verification in CONTENT_PHASE
    // When returning NGX_DECLINED, nginx will still proceed to CONTENT_PHASE, so we need to clear
    // the content handler to prevent duplicate payment verification
    clear_handler_fn(
        r,
        req_mut,
        &format!("for {} request to prevent payment verification", method),
    );
    None
}
