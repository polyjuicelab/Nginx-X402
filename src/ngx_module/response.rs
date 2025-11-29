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

/// Send OPTIONS response for CORS preflight requests
///
/// Sends a 204 No Content response with appropriate CORS headers for OPTIONS requests.
/// This is used when OPTIONS requests skip payment verification but need a proper response.
///
/// # Arguments
/// - `r`: Nginx request object
///
/// # Returns
/// - `Ok(())` if response is sent successfully
/// - `Err` if response cannot be sent
pub fn send_options_response(r: &mut Request) -> Result<()> {
    // Set status code 204 (No Content) - standard for OPTIONS requests
    r.set_status(HTTPStatus::from_u16(204).map_err(|_| ConfigError::from("Invalid status code"))?);

    // Add CORS headers if Origin header is present
    if let Some(origin) = crate::ngx_module::request::get_header_value(r, "Origin") {
        // Allow the origin that made the request
        r.add_header_out("Access-Control-Allow-Origin", &origin)
            .ok_or_else(|| ConfigError::from("Failed to set Access-Control-Allow-Origin header"))?;

        // Get requested method and headers
        let requested_method =
            crate::ngx_module::request::get_header_value(r, "Access-Control-Request-Method")
                .unwrap_or_else(|| "GET, POST, PUT, DELETE, OPTIONS".to_string());
        let requested_headers =
            crate::ngx_module::request::get_header_value(r, "Access-Control-Request-Headers")
                .unwrap_or_else(|| "content-type, authorization, x-payment".to_string());

        // Set CORS response headers
        r.add_header_out("Access-Control-Allow-Methods", &requested_method)
            .ok_or_else(|| {
                ConfigError::from("Failed to set Access-Control-Allow-Methods header")
            })?;

        r.add_header_out("Access-Control-Allow-Headers", &requested_headers)
            .ok_or_else(|| {
                ConfigError::from("Failed to set Access-Control-Allow-Headers header")
            })?;

        // Allow credentials if needed
        r.add_header_out("Access-Control-Allow-Credentials", "true")
            .ok_or_else(|| {
                ConfigError::from("Failed to set Access-Control-Allow-Credentials header")
            })?;

        // Set max age for preflight cache (24 hours)
        r.add_header_out("Access-Control-Max-Age", "86400")
            .ok_or_else(|| ConfigError::from("Failed to set Access-Control-Max-Age header"))?;
    }

    // Set content length to 0 for 204 No Content
    r.set_content_length_n(0);

    // Send header (204 has no body, so we only send header)
    let status = r.send_header();
    if status != Status::NGX_OK {
        return Err(ConfigError::from(format!(
            "Failed to send OPTIONS response header: status={status:?}"
        )));
    }

    // For 204 No Content, we don't need to send a body
    // The header is sufficient
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

    // Copy body data to buffer
    unsafe {
        let buf_slice = core::slice::from_raw_parts_mut((*buf).pos, body_len);
        buf_slice.copy_from_slice(body);
        (*buf).last = (*buf).pos.add(body_len);
        (*buf).set_last_buf(1);
        (*buf).set_last_in_chain(1);
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
