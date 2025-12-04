//! Response generation and sending

use crate::ngx_module::config::ParsedX402Config;
use crate::ngx_module::error::{ConfigError, Result};
use crate::ngx_module::request::is_browser_request;
use ngx::core::Status;
use ngx::http::{HTTPStatus, Request};
use rust_x402::template::generate_paywall_html;
use rust_x402::types::{PaymentRequirements, PaymentRequirementsResponse};
use serde_json;

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
pub fn send_402_response(
    r: &mut Request,
    requirements: &[PaymentRequirements],
    config: &ParsedX402Config,
    error_msg: Option<&str>,
) -> Result<()> {
    // Set status code 402 (Payment Required)
    r.set_status(HTTPStatus::from_u16(402).map_err(|_| ConfigError::from("Invalid status code"))?);

    let is_browser = is_browser_request(r);

    // Use error_msg if provided, otherwise use config description, otherwise use empty string
    let error_message = error_msg.or(config.description.as_deref()).unwrap_or("");

    if is_browser {
        // Send HTML paywall
        let html = generate_paywall_html(error_message, requirements, None);

        // Set Content-Type header
        r.add_header_out("Content-Type", "text/html; charset=utf-8")
            .ok_or_else(|| ConfigError::from("Failed to set Content-Type header"))?;

        // Send body using buffer and chain
        send_response_body(r, html.as_bytes())?;
    } else {
        // Send JSON response
        let response = PaymentRequirementsResponse::new(error_message, requirements.to_vec());
        let json = serde_json::to_string(&response)
            .map_err(|_| ConfigError::from("Failed to serialize response"))?;

        // Set Content-Type header
        r.add_header_out("Content-Type", "application/json; charset=utf-8")
            .ok_or_else(|| ConfigError::from("Failed to set Content-Type header"))?;

        // Send body using buffer and chain
        send_response_body(r, json.as_bytes())?;
    }

    Ok(())
}

/// Send response body using ngx buffer and chain
pub fn send_response_body(r: &mut Request, body: &[u8]) -> Result<()> {
    use ngx::ffi::{ngx_alloc_chain_link, ngx_create_temp_buf};

    let pool = r.pool();
    let body_len = body.len();

    // Ensure body is not empty - nginx requires non-zero buffer size
    if body_len == 0 {
        return Err(ConfigError::from("Cannot send empty response body"));
    }

    // Create temporary buffer
    let buf = unsafe { ngx_create_temp_buf(pool.as_ptr(), body_len) };
    if buf.is_null() {
        return Err(ConfigError::from("Failed to allocate buffer"));
    }

    // Copy body data to buffer with bounds checking
    unsafe {
        // Validate buffer pointer and fields
        let buf_ptr = &mut *buf;

        // Verify pos is not null
        if buf_ptr.pos.is_null() {
            return Err(ConfigError::from("Buffer pos pointer is null"));
        }

        // Verify buffer capacity: end - pos should be >= body_len
        // ngx_create_temp_buf allocates exactly body_len bytes, so end - pos == body_len
        // But we check anyway to be safe
        let pos_ptr = buf_ptr.pos as usize;
        let end_ptr = buf_ptr.end as usize;

        if end_ptr < pos_ptr {
            return Err(ConfigError::from("Buffer end is before buffer pos"));
        }

        let buf_capacity = end_ptr - pos_ptr;
        if buf_capacity < body_len {
            return Err(ConfigError::from(format!(
                "Buffer capacity ({}) is less than body length ({})",
                buf_capacity, body_len
            )));
        }

        // Use checked_add to prevent integer overflow
        let new_last_ptr = pos_ptr
            .checked_add(body_len)
            .ok_or_else(|| ConfigError::from("Buffer pointer addition overflow"))?;

        // Verify new_last doesn't exceed buffer end
        if new_last_ptr > end_ptr {
            return Err(ConfigError::from("Buffer pointer would exceed buffer end"));
        }

        // Safe to create slice: we've validated pos is not null and body_len is within bounds
        let buf_slice = core::slice::from_raw_parts_mut(buf_ptr.pos, body_len);
        buf_slice.copy_from_slice(body);

        // Safe to set last: we've validated the addition doesn't overflow
        // last is a *mut u8, so we can directly assign the pointer
        buf_ptr.last = new_last_ptr as *mut u8;
        buf_ptr.set_last_buf(1);
        buf_ptr.set_last_in_chain(1);
    }

    // Allocate chain link
    let chain = unsafe { ngx_alloc_chain_link(pool.as_ptr()) };
    if chain.is_null() {
        return Err(ConfigError::from("Failed to allocate chain link"));
    }

    unsafe {
        (*chain).buf = buf;
        (*chain).next = core::ptr::null_mut();
    }

    // Set content length
    r.set_content_length_n(body_len);

    // CRITICAL: In Nginx content handler, send_header() may fail if:
    // 1. Request headers_out.status is not set (we set it in send_402_response)
    // 2. Request is already finalized
    // 3. Header was already sent
    // 4. Request is not in correct state
    //
    // In Nginx, r->header_sent is a flag indicating if header was sent
    // But ngx-rust may wrap this differently, so we'll try to send header anyway
    // If it fails, we'll handle the error

    // Send header using ngx-rust's method
    // This should handle request state correctly
    let status = r.send_header();
    if status == Status::NGX_OK {
    } else {
        // send_header failed - check if we can use alternative approach
        // In Nginx, we can use ngx_http_send_special_response for error responses
        // But for now, let's try to understand why it fails
        // The issue might be that we're calling send_header from a phase handler context
        // instead of a content handler context
        let error_msg =
            format!("Failed to send header: status={status:?}. Request state may be incorrect.");
        return Err(ConfigError::from(error_msg));
    }

    // Send body using output filter
    let chain_mut = unsafe { &mut *chain };
    let status = r.output_filter(chain_mut);
    if status == Status::NGX_OK {
    } else {
        let error_msg = format!("Failed to send body: status={status:?}");
        return Err(ConfigError::from(error_msg));
    }

    Ok(())
}
