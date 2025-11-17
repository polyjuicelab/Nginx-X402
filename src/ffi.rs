//! FFI bindings for Nginx x402 module
//!
//! This module provides C-compatible function bindings that can be called from
//! Nginx C modules. All functions are designed to be thread-safe and memory-safe.
//!
//! # Return Codes
//!
//! All functions return an integer status code:
//! - `0` = Success
//! - `1` = Invalid input
//! - `2` = Payment verification failed
//! - `3` = Facilitator error
//! - `4` = Memory allocation error
//! - `5` = Internal error
//!
//! # Memory Management
//!
//! Functions that return strings allocate memory that must be freed using
//! `x402_free_string`. The caller is responsible for freeing all allocated memory.

use crate::config::NginxX402Config;
use rust_x402::template::generate_paywall_html;
use rust_x402::types::{PaymentPayload, PaymentRequirements, PaymentRequirementsResponse};
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};
use std::ptr;
use std::sync::OnceLock;

/// Global tokio runtime for async operations
static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();

/// Get or create the global tokio runtime
fn get_runtime() -> &'static tokio::runtime::Runtime {
    RUNTIME.get_or_init(|| tokio::runtime::Runtime::new().expect("Failed to create tokio runtime"))
}

/// Free a string allocated by x402 functions
///
/// # Safety
///
/// This function must only be called with pointers returned by x402 functions.
/// Calling with other pointers or multiple times on the same pointer is undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn x402_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        let _ = CString::from_raw(ptr);
    }
}

/// Verify a payment payload
///
/// # Arguments
/// - `payment_b64`: Base64-encoded payment payload from X-PAYMENT header
/// - `requirements_json`: JSON string of payment requirements
/// - `facilitator_url`: URL of the facilitator service
/// - `result`: Output buffer for result (JSON)
/// - `result_len`: Input: buffer size, Output: actual length
///
/// # Returns
/// - `0` on success (payment is valid)
/// - `1` on invalid input
/// - `2` on payment verification failure
/// - `3` on facilitator error
/// - `5` on internal error
#[no_mangle]
pub unsafe extern "C" fn x402_verify_payment(
    payment_b64: *const c_char,
    requirements_json: *const c_char,
    facilitator_url: *const c_char,
    result: *mut c_char,
    result_len: *mut usize,
) -> c_int {
    if payment_b64.is_null()
        || requirements_json.is_null()
        || result.is_null()
        || result_len.is_null()
    {
        return 1;
    }

    let payment_str = match CStr::from_ptr(payment_b64).to_str() {
        Ok(s) => s,
        Err(_) => return 1,
    };

    let requirements_str = match CStr::from_ptr(requirements_json).to_str() {
        Ok(s) => s,
        Err(_) => return 1,
    };

    let facilitator_url_str = if facilitator_url.is_null() {
        "https://x402.org/facilitator"
    } else {
        match CStr::from_ptr(facilitator_url).to_str() {
            Ok(s) => s,
            Err(_) => return 1,
        }
    };

    // Decode payment payload
    let payment_payload = match PaymentPayload::from_base64(payment_str) {
        Ok(p) => p,
        Err(_) => return 1,
    };

    // Parse requirements
    let requirements: PaymentRequirements = match serde_json::from_str(requirements_str) {
        Ok(r) => r,
        Err(_) => return 1,
    };

    // Verify payment using facilitator
    let facilitator_config = rust_x402::types::FacilitatorConfig::new(facilitator_url_str);
    let facilitator = match rust_x402::facilitator::FacilitatorClient::new(facilitator_config) {
        Ok(f) => f,
        Err(_) => return 3,
    };

    let verify_result =
        get_runtime().block_on(async { facilitator.verify(&payment_payload, &requirements).await });

    let verify_response = match verify_result {
        Ok(v) => v,
        Err(_) => return 3,
    };

    // Write result to buffer
    let result_json = match serde_json::to_string(&verify_response) {
        Ok(j) => j,
        Err(_) => return 5,
    };

    let result_bytes = result_json.as_bytes();
    let required_len = result_bytes.len() + 1; // +1 for null terminator

    if *result_len < required_len {
        *result_len = required_len;
        return 4; // Buffer too small
    }

    ptr::copy_nonoverlapping(result_bytes.as_ptr(), result as *mut u8, result_bytes.len());
    *result.add(result_bytes.len()) = 0; // Null terminator
    *result_len = result_bytes.len();

    if verify_response.is_valid {
        0
    } else {
        2
    }
}

/// Create payment requirements JSON
///
/// # Arguments
/// - `amount`: Payment amount as decimal string (e.g., "0.0001")
/// - `pay_to`: Recipient wallet address
/// - `network`: Network identifier (e.g., "base-sepolia")
/// - `resource`: Resource URL
/// - `description`: Payment description
/// - `testnet`: Whether to use testnet (1 = true, 0 = false)
/// - `result`: Output buffer for JSON result
/// - `result_len`: Input: buffer size, Output: actual length
///
/// # Returns
/// - `0` on success
/// - `1` on invalid input
/// - `4` on buffer too small
/// - `5` on internal error
#[no_mangle]
pub unsafe extern "C" fn x402_create_requirements(
    amount: *const c_char,
    pay_to: *const c_char,
    network: *const c_char,
    resource: *const c_char,
    description: *const c_char,
    testnet: c_int,
    result: *mut c_char,
    result_len: *mut usize,
) -> c_int {
    if amount.is_null() || pay_to.is_null() || result.is_null() || result_len.is_null() {
        return 1;
    }

    let config =
        match unsafe { NginxX402Config::from_c_strings(amount, pay_to, ptr::null(), testnet != 0) }
        {
            Ok(c) => c,
            Err(_) => return 1,
        };

    let network_str = if network.is_null() {
        if testnet != 0 {
            rust_x402::types::networks::BASE_SEPOLIA
        } else {
            rust_x402::types::networks::BASE_MAINNET
        }
    } else {
        match CStr::from_ptr(network).to_str() {
            Ok(s) => s,
            Err(_) => return 1,
        }
    };

    let resource_str = if resource.is_null() {
        "/"
    } else {
        match CStr::from_ptr(resource).to_str() {
            Ok(s) => s,
            Err(_) => return 1,
        }
    };

    let requirements = match config.create_payment_requirements(resource_str) {
        Ok(r) => r,
        Err(_) => return 5,
    };

    // Override network if provided
    let mut requirements = requirements;
    if !network.is_null() {
        requirements.network = network_str.to_string();
    }

    // Override description if provided
    if !description.is_null() {
        if let Ok(desc) = CStr::from_ptr(description).to_str() {
            requirements.description = desc.to_string();
        }
    }

    let result_json = match serde_json::to_string(&requirements) {
        Ok(j) => j,
        Err(_) => return 5,
    };

    let result_bytes = result_json.as_bytes();
    let required_len = result_bytes.len() + 1;

    if *result_len < required_len {
        *result_len = required_len;
        return 4;
    }

    ptr::copy_nonoverlapping(result_bytes.as_ptr(), result as *mut u8, result_bytes.len());
    *result.add(result_bytes.len()) = 0;
    *result_len = result_bytes.len();

    0
}

/// Generate paywall HTML
///
/// # Arguments
/// - `requirements_json`: JSON string of payment requirements
/// - `error_msg`: Error message to display (can be null)
/// - `result`: Output buffer for HTML
/// - `result_len`: Input: buffer size, Output: actual length
///
/// # Returns
/// - `0` on success
/// - `1` on invalid input
/// - `4` on buffer too small
/// - `5` on internal error
#[no_mangle]
pub unsafe extern "C" fn x402_generate_paywall_html(
    requirements_json: *const c_char,
    error_msg: *const c_char,
    result: *mut c_char,
    result_len: *mut usize,
) -> c_int {
    if requirements_json.is_null() || result.is_null() || result_len.is_null() {
        return 1;
    }

    let requirements_str = match CStr::from_ptr(requirements_json).to_str() {
        Ok(s) => s,
        Err(_) => return 1,
    };

    let requirements: Vec<PaymentRequirements> = match serde_json::from_str(requirements_str) {
        Ok(r) => r,
        Err(_) => return 1,
    };

    let error = if error_msg.is_null() {
        "Payment required"
    } else {
        match CStr::from_ptr(error_msg).to_str() {
            Ok(s) => s,
            Err(_) => "Payment required",
        }
    };

    let html = generate_paywall_html(error, &requirements, None);

    let html_bytes = html.as_bytes();
    let required_len = html_bytes.len() + 1;

    if *result_len < required_len {
        *result_len = required_len;
        return 4;
    }

    ptr::copy_nonoverlapping(html_bytes.as_ptr(), result as *mut u8, html_bytes.len());
    *result.add(html_bytes.len()) = 0;
    *result_len = html_bytes.len();

    0
}

/// Generate JSON 402 response
///
/// # Arguments
/// - `requirements_json`: JSON string of payment requirements
/// - `error_msg`: Error message (can be null)
/// - `result`: Output buffer for JSON
/// - `result_len`: Input: buffer size, Output: actual length
///
/// # Returns
/// - `0` on success
/// - `1` on invalid input
/// - `4` on buffer too small
/// - `5` on internal error
#[no_mangle]
pub unsafe extern "C" fn x402_generate_json_response(
    requirements_json: *const c_char,
    error_msg: *const c_char,
    result: *mut c_char,
    result_len: *mut usize,
) -> c_int {
    if requirements_json.is_null() || result.is_null() || result_len.is_null() {
        return 1;
    }

    let requirements_str = match CStr::from_ptr(requirements_json).to_str() {
        Ok(s) => s,
        Err(_) => return 1,
    };

    let requirements: Vec<PaymentRequirements> = match serde_json::from_str(requirements_str) {
        Ok(r) => r,
        Err(_) => return 1,
    };

    let error = if error_msg.is_null() {
        "X-PAYMENT header is required"
    } else {
        match CStr::from_ptr(error_msg).to_str() {
            Ok(s) => s,
            Err(_) => "X-PAYMENT header is required",
        }
    };

    let response = PaymentRequirementsResponse::new(error, requirements);
    let result_json = match serde_json::to_string(&response) {
        Ok(j) => j,
        Err(_) => return 5,
    };

    let result_bytes = result_json.as_bytes();
    let required_len = result_bytes.len() + 1;

    if *result_len < required_len {
        *result_len = required_len;
        return 4;
    }

    ptr::copy_nonoverlapping(result_bytes.as_ptr(), result as *mut u8, result_bytes.len());
    *result.add(result_bytes.len()) = 0;
    *result_len = result_bytes.len();

    0
}

/// Check if request is from a browser
///
/// # Arguments
/// - `user_agent`: User-Agent header value (can be null)
/// - `accept`: Accept header value (can be null)
///
/// # Returns
/// - `1` if browser request
/// - `0` if API request
#[no_mangle]
pub unsafe extern "C" fn x402_is_browser_request(
    user_agent: *const c_char,
    accept: *const c_char,
) -> c_int {
    let user_agent_str = if user_agent.is_null() {
        ""
    } else {
        CStr::from_ptr(user_agent).to_str().unwrap_or("")
    };

    let accept_str = if accept.is_null() {
        ""
    } else {
        CStr::from_ptr(accept).to_str().unwrap_or("")
    };

    if rust_x402::template::is_browser_request(user_agent_str, accept_str) {
        1
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn test_free_string() {
        let s = CString::new("test").unwrap();
        let ptr = s.into_raw();
        unsafe {
            x402_free_string(ptr);
        }
    }

    #[test]
    fn test_is_browser_request() {
        let user_agent = CString::new("Mozilla/5.0").unwrap();
        let accept = CString::new("text/html").unwrap();

        let result = unsafe { x402_is_browser_request(user_agent.as_ptr(), accept.as_ptr()) };
        assert_eq!(result, 1);

        let accept_json = CString::new("application/json").unwrap();
        let result = unsafe { x402_is_browser_request(user_agent.as_ptr(), accept_json.as_ptr()) };
        assert_eq!(result, 0);
    }

    #[test]
    fn test_create_requirements() {
        let amount = CString::new("0.0001").unwrap();
        let pay_to = CString::new("0x209693Bc6afc0C5328bA36FaF03C514EF312287C").unwrap();
        let network = CString::new("base-sepolia").unwrap();
        let resource = CString::new("/test").unwrap();
        let description = CString::new("Test payment").unwrap();

        let mut result = vec![0u8; 4096];
        let mut result_len = result.len();

        let status = unsafe {
            x402_create_requirements(
                amount.as_ptr(),
                pay_to.as_ptr(),
                network.as_ptr(),
                resource.as_ptr(),
                description.as_ptr(),
                1, // testnet
                result.as_mut_ptr() as *mut c_char,
                &mut result_len,
            )
        };

        assert_eq!(status, 0);
        assert!(result_len > 0);

        let result_str = unsafe {
            CStr::from_ptr(result.as_ptr() as *const c_char)
                .to_str()
                .unwrap()
        };
        assert!(result_str.contains("base-sepolia"));
    }
}

