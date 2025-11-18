//! Response generation and sending

use crate::ngx_module::config::ParsedX402Config;
use crate::ngx_module::error::{ConfigError, Result};
use crate::ngx_module::request::is_browser_request;
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

    // Send header
    let status = r.send_header();
    if !status.is_ok() {
        return Err(ConfigError::from("Failed to send header"));
    }

    // Send body
    let chain_mut = unsafe { &mut *chain };
    let status = r.output_filter(chain_mut);
    if !status.is_ok() {
        return Err(ConfigError::from("Failed to send body"));
    }

    Ok(())
}
