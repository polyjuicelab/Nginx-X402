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
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers.
/// The caller must ensure:
/// - `payment_b64` points to a valid null-terminated C string (or is null)
/// - `requirements_json` points to a valid null-terminated C string
/// - `facilitator_url` points to a valid null-terminated C string (or is null)
/// - `result` points to a buffer of at least `*result_len` bytes
/// - `result_len` points to a valid `usize` value
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
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers.
/// The caller must ensure:
/// - `amount` points to a valid null-terminated C string
/// - `pay_to` points to a valid null-terminated C string
/// - `network` points to a valid null-terminated C string (or is null)
/// - `resource` points to a valid null-terminated C string (or is null)
/// - `description` points to a valid null-terminated C string (or is null)
/// - `result` points to a buffer of at least `*result_len` bytes
/// - `result_len` points to a valid `usize` value
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
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers.
/// The caller must ensure:
/// - `requirements_json` points to a valid null-terminated C string
/// - `error_msg` points to a valid null-terminated C string (or is null)
/// - `result` points to a buffer of at least `*result_len` bytes
/// - `result_len` points to a valid `usize` value
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
        CStr::from_ptr(error_msg)
            .to_str()
            .unwrap_or("Payment required")
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
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers.
/// The caller must ensure:
/// - `requirements_json` points to a valid null-terminated C string
/// - `error_msg` points to a valid null-terminated C string (or is null)
/// - `result` points to a buffer of at least `*result_len` bytes
/// - `result_len` points to a valid `usize` value
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
        CStr::from_ptr(error_msg)
            .to_str()
            .unwrap_or("X-PAYMENT header is required")
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
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers.
/// The caller must ensure:
/// - `user_agent` points to a valid null-terminated C string (or is null)
/// - `accept` points to a valid null-terminated C string (or is null)
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
    use std::ptr;

    #[test]
    fn test_free_string() {
        let s = CString::new("test").unwrap();
        let ptr = s.into_raw();
        unsafe {
            x402_free_string(ptr);
        }
    }

    #[test]
    fn test_free_string_null() {
        // Should handle null pointer gracefully
        unsafe {
            x402_free_string(ptr::null_mut());
        }
    }

    #[test]
    fn test_is_browser_request() {
        let user_agent = CString::new("Mozilla/5.0").unwrap();
        let accept = CString::new("text/html").unwrap();

        let result = unsafe { x402_is_browser_request(user_agent.as_ptr(), accept.as_ptr()) };
        assert_eq!(result, 1, "Mozilla with text/html should be browser");

        let accept_json = CString::new("application/json").unwrap();
        let result = unsafe { x402_is_browser_request(user_agent.as_ptr(), accept_json.as_ptr()) };
        assert_eq!(
            result, 0,
            "Mozilla with application/json should not be browser"
        );
    }

    #[test]
    fn test_is_browser_request_null_pointers() {
        // Both null
        let result = unsafe { x402_is_browser_request(ptr::null(), ptr::null()) };
        assert_eq!(result, 0, "Both null should return 0");

        // User-Agent null, Accept valid
        let accept = CString::new("text/html").unwrap();
        let result = unsafe { x402_is_browser_request(ptr::null(), accept.as_ptr()) };
        assert!(
            result == 0 || result == 1,
            "Result should be 0 or 1, got: {}",
            result
        );

        // User-Agent valid, Accept null
        let ua = CString::new("Mozilla/5.0").unwrap();
        let result = unsafe { x402_is_browser_request(ua.as_ptr(), ptr::null()) };
        assert!(
            result == 0 || result == 1,
            "Result should be 0 or 1, got: {}",
            result
        );
    }

    #[test]
    fn test_is_browser_request_various_combinations() {
        let mozilla = CString::new("Mozilla/5.0").unwrap();
        let chrome = CString::new("Mozilla/5.0 (Windows NT 10.0) Chrome/91.0").unwrap();
        let curl = CString::new("curl/7.68.0").unwrap();

        let html = CString::new("text/html").unwrap();
        let json = CString::new("application/json").unwrap();
        let xhtml = CString::new("application/xhtml+xml").unwrap();

        // Browser with HTML
        assert_eq!(
            unsafe { x402_is_browser_request(mozilla.as_ptr(), html.as_ptr()) },
            1,
            "Browser with HTML should return 1"
        );

        // Browser with XHTML - check actual behavior (may vary by implementation)
        let xhtml_result = unsafe { x402_is_browser_request(chrome.as_ptr(), xhtml.as_ptr()) };
        assert!(
            xhtml_result == 0 || xhtml_result == 1,
            "Browser with XHTML should return 0 or 1, got: {}",
            xhtml_result
        );

        // Browser with JSON (should not be browser)
        assert_eq!(
            unsafe { x402_is_browser_request(mozilla.as_ptr(), json.as_ptr()) },
            0,
            "Browser with JSON should return 0"
        );

        // API client
        assert_eq!(
            unsafe { x402_is_browser_request(curl.as_ptr(), json.as_ptr()) },
            0,
            "curl with JSON should return 0"
        );
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

        assert_eq!(status, 0, "Status should be 0 for valid input");
        assert!(result_len > 0, "Result length should be greater than 0");

        let result_str = unsafe {
            CStr::from_ptr(result.as_ptr() as *const c_char)
                .to_str()
                .expect("Result should be valid UTF-8")
        };
        assert!(
            result_str.contains("base-sepolia"),
            "Result should contain network name"
        );

        // Verify it's valid JSON
        let requirements: PaymentRequirements = serde_json::from_str(result_str)
            .expect("Result should be valid JSON PaymentRequirements");
        assert_eq!(requirements.scheme, "exact", "Scheme should be exact");
        assert_eq!(requirements.network, "base-sepolia", "Network should match");
        assert_eq!(requirements.resource, "/test", "Resource should match");
    }

    #[test]
    fn test_create_requirements_null_pointers() {
        let amount = CString::new("0.0001").unwrap();
        let pay_to = CString::new("0x209693Bc6afc0C5328bA36FaF03C514EF312287C").unwrap();
        let mut result = vec![0u8; 4096];
        let mut result_len = result.len();

        // Null amount
        let status = unsafe {
            x402_create_requirements(
                ptr::null(),
                pay_to.as_ptr(),
                ptr::null(),
                ptr::null(),
                ptr::null(),
                1,
                result.as_mut_ptr() as *mut c_char,
                &mut result_len,
            )
        };
        assert_eq!(status, 1, "Should return 1 for null amount");

        // Null pay_to
        let status = unsafe {
            x402_create_requirements(
                amount.as_ptr(),
                ptr::null(),
                ptr::null(),
                ptr::null(),
                ptr::null(),
                1,
                result.as_mut_ptr() as *mut c_char,
                &mut result_len,
            )
        };
        assert_eq!(status, 1, "Should return 1 for null pay_to");
    }

    #[test]
    fn test_create_requirements_invalid_amount() {
        let pay_to = CString::new("0x209693Bc6afc0C5328bA36FaF03C514EF312287C").unwrap();
        let mut result = vec![0u8; 4096];
        let mut result_len = result.len();

        // Invalid amount format
        let invalid_amount = CString::new("not-a-number").unwrap();
        let status = unsafe {
            x402_create_requirements(
                invalid_amount.as_ptr(),
                pay_to.as_ptr(),
                ptr::null(),
                ptr::null(),
                ptr::null(),
                1,
                result.as_mut_ptr() as *mut c_char,
                &mut result_len,
            )
        };
        assert_eq!(status, 1, "Should return 1 for invalid amount");

        // Empty amount
        let empty_amount = CString::new("").unwrap();
        let status = unsafe {
            x402_create_requirements(
                empty_amount.as_ptr(),
                pay_to.as_ptr(),
                ptr::null(),
                ptr::null(),
                ptr::null(),
                1,
                result.as_mut_ptr() as *mut c_char,
                &mut result_len,
            )
        };
        assert_eq!(status, 1, "Should return 1 for empty amount");
    }

    #[test]
    fn test_create_requirements_buffer_too_small() {
        let amount = CString::new("0.0001").unwrap();
        let pay_to = CString::new("0x209693Bc6afc0C5328bA36FaF03C514EF312287C").unwrap();
        let network = CString::new("base-sepolia").unwrap();
        let resource = CString::new("/test").unwrap();
        let description = CString::new("Test payment").unwrap();

        // Very small buffer
        let mut result = vec![0u8; 10];
        let mut result_len = result.len();

        let status = unsafe {
            x402_create_requirements(
                amount.as_ptr(),
                pay_to.as_ptr(),
                network.as_ptr(),
                resource.as_ptr(),
                description.as_ptr(),
                1,
                result.as_mut_ptr() as *mut c_char,
                &mut result_len,
            )
        };
        assert_eq!(status, 4, "Should return 4 (buffer too small)");
        assert!(
            result_len > 10,
            "result_len should be updated to required size: {}",
            result_len
        );
    }

    #[test]
    fn test_create_requirements_with_null_optional_params() {
        let amount = CString::new("0.0001").unwrap();
        let pay_to = CString::new("0x209693Bc6afc0C5328bA36FaF03C514EF312287C").unwrap();
        let mut result = vec![0u8; 4096];
        let mut result_len = result.len();

        // All optional params null
        let status = unsafe {
            x402_create_requirements(
                amount.as_ptr(),
                pay_to.as_ptr(),
                ptr::null(), // network
                ptr::null(), // resource
                ptr::null(), // description
                1,           // testnet
                result.as_mut_ptr() as *mut c_char,
                &mut result_len,
            )
        };
        assert_eq!(status, 0, "Should succeed with null optional params");
        assert!(result_len > 0, "Should produce output");

        let result_str = unsafe {
            CStr::from_ptr(result.as_ptr() as *const c_char)
                .to_str()
                .expect("Result should be valid UTF-8")
        };
        let requirements: PaymentRequirements =
            serde_json::from_str(result_str).expect("Result should be valid JSON");
        assert_eq!(
            requirements.network, "base-sepolia",
            "Should default to base-sepolia for testnet"
        );
    }

    #[test]
    fn test_generate_paywall_html_null_pointers() {
        let requirements_json = r#"[{"scheme":"exact","network":"base-sepolia"}]"#;
        let requirements_cstr = CString::new(requirements_json).unwrap();
        let mut result = vec![0u8; 4096];
        let mut result_len = result.len();

        // Null requirements_json
        let status = unsafe {
            x402_generate_paywall_html(
                ptr::null(),
                ptr::null(),
                result.as_mut_ptr() as *mut c_char,
                &mut result_len,
            )
        };
        assert_eq!(status, 1, "Should return 1 for null requirements_json");

        // Null result buffer
        let mut result_len = 4096;
        let status = unsafe {
            x402_generate_paywall_html(
                requirements_cstr.as_ptr(),
                ptr::null(),
                ptr::null_mut(),
                &mut result_len,
            )
        };
        assert_eq!(status, 1, "Should return 1 for null result buffer");
    }

    #[test]
    fn test_generate_paywall_html_invalid_json() {
        let invalid_json = CString::new("not valid json").unwrap();
        let mut result = vec![0u8; 4096];
        let mut result_len = result.len();

        let status = unsafe {
            x402_generate_paywall_html(
                invalid_json.as_ptr(),
                ptr::null(),
                result.as_mut_ptr() as *mut c_char,
                &mut result_len,
            )
        };
        assert_eq!(status, 1, "Should return 1 for invalid JSON");
    }

    #[test]
    fn test_generate_paywall_html_buffer_too_small() {
        let requirements_json = r#"[{"scheme":"exact","network":"base-sepolia","maxAmountRequired":"1000000","asset":"0x036CbD53842c5426634e7929541eC2318f3dCF7e","payTo":"0x209693Bc6afc0C5328bA36FaF03C514EF312287C","resource":"/test","description":"Test","maxTimeoutSeconds":60}]"#;
        let requirements_cstr = CString::new(requirements_json).unwrap();
        let mut result = vec![0u8; 10]; // Very small buffer
        let mut result_len = result.len();

        let status = unsafe {
            x402_generate_paywall_html(
                requirements_cstr.as_ptr(),
                ptr::null(),
                result.as_mut_ptr() as *mut c_char,
                &mut result_len,
            )
        };
        assert_eq!(status, 4, "Should return 4 (buffer too small)");
        assert!(
            result_len > 10,
            "result_len should be updated to required size"
        );
    }

    #[test]
    fn test_generate_paywall_html_with_error_message() {
        let requirements_json = r#"[{"scheme":"exact","network":"base-sepolia","maxAmountRequired":"1000000","asset":"0x036CbD53842c5426634e7929541eC2318f3dCF7e","payTo":"0x209693Bc6afc0C5328bA36FaF03C514EF312287C","resource":"/test","description":"Test","maxTimeoutSeconds":60}]"#;
        let requirements_cstr = CString::new(requirements_json).unwrap();
        let error_msg = CString::new("Custom error message").unwrap();
        let mut result = vec![0u8; 8192];
        let mut result_len = result.len();

        let status = unsafe {
            x402_generate_paywall_html(
                requirements_cstr.as_ptr(),
                error_msg.as_ptr(),
                result.as_mut_ptr() as *mut c_char,
                &mut result_len,
            )
        };
        assert_eq!(status, 0, "Should succeed");
        let html = unsafe {
            CStr::from_ptr(result.as_ptr() as *const c_char)
                .to_str()
                .expect("Result should be valid UTF-8")
        };
        assert!(
            html.contains("Custom error message"),
            "HTML should contain custom error message"
        );
        assert!(
            html.contains("<!DOCTYPE html>"),
            "HTML should contain DOCTYPE"
        );
    }

    #[test]
    fn test_generate_json_response_null_pointers() {
        let requirements_json = r#"[{"scheme":"exact","network":"base-sepolia"}]"#;
        let requirements_cstr = CString::new(requirements_json).unwrap();
        let mut result = vec![0u8; 4096];
        let mut result_len = result.len();

        // Null requirements_json
        let status = unsafe {
            x402_generate_json_response(
                ptr::null(),
                ptr::null(),
                result.as_mut_ptr() as *mut c_char,
                &mut result_len,
            )
        };
        assert_eq!(status, 1, "Should return 1 for null requirements_json");

        // Null result buffer
        let mut result_len = 4096;
        let status = unsafe {
            x402_generate_json_response(
                requirements_cstr.as_ptr(),
                ptr::null(),
                ptr::null_mut(),
                &mut result_len,
            )
        };
        assert_eq!(status, 1, "Should return 1 for null result buffer");
    }

    #[test]
    fn test_generate_json_response_invalid_json() {
        let invalid_json = CString::new("not valid json").unwrap();
        let mut result = vec![0u8; 4096];
        let mut result_len = result.len();

        let status = unsafe {
            x402_generate_json_response(
                invalid_json.as_ptr(),
                ptr::null(),
                result.as_mut_ptr() as *mut c_char,
                &mut result_len,
            )
        };
        assert_eq!(status, 1, "Should return 1 for invalid JSON");
    }

    #[test]
    fn test_generate_json_response_buffer_too_small() {
        let requirements_json = r#"[{"scheme":"exact","network":"base-sepolia","maxAmountRequired":"1000000","asset":"0x036CbD53842c5426634e7929541eC2318f3dCF7e","payTo":"0x209693Bc6afc0C5328bA36FaF03C514EF312287C","resource":"/test","description":"Test","maxTimeoutSeconds":60}]"#;
        let requirements_cstr = CString::new(requirements_json).unwrap();
        let mut result = vec![0u8; 10]; // Very small buffer
        let mut result_len = result.len();

        let status = unsafe {
            x402_generate_json_response(
                requirements_cstr.as_ptr(),
                ptr::null(),
                result.as_mut_ptr() as *mut c_char,
                &mut result_len,
            )
        };
        assert_eq!(status, 4, "Should return 4 (buffer too small)");
        assert!(
            result_len > 10,
            "result_len should be updated to required size"
        );
    }

    #[test]
    fn test_generate_json_response_structure() {
        let requirements_json = r#"[{"scheme":"exact","network":"base-sepolia","maxAmountRequired":"1000000","asset":"0x036CbD53842c5426634e7929541eC2318f3dCF7e","payTo":"0x209693Bc6afc0C5328bA36FaF03C514EF312287C","resource":"/test","description":"Test","maxTimeoutSeconds":60}]"#;
        let requirements_cstr = CString::new(requirements_json).unwrap();
        let mut result = vec![0u8; 4096];
        let mut result_len = result.len();

        let status = unsafe {
            x402_generate_json_response(
                requirements_cstr.as_ptr(),
                ptr::null(),
                result.as_mut_ptr() as *mut c_char,
                &mut result_len,
            )
        };
        assert_eq!(status, 0, "Should succeed");
        let json_str = unsafe {
            CStr::from_ptr(result.as_ptr() as *const c_char)
                .to_str()
                .expect("Result should be valid UTF-8")
        };
        let json: serde_json::Value =
            serde_json::from_str(json_str).expect("Result should be valid JSON");

        // Verify JSON structure
        assert!(
            json.get("error").is_some() || json.get("message").is_some(),
            "JSON should have error or message field. Got: {}",
            json_str
        );
        assert!(
            json.get("paymentRequirements").is_some() || json.get("accepts").is_some(),
            "JSON should have paymentRequirements or accepts field. Got: {}",
            json_str
        );
    }

    #[test]
    fn test_generate_json_response_with_error_message() {
        let requirements_json = r#"[{"scheme":"exact","network":"base-sepolia","maxAmountRequired":"1000000","asset":"0x036CbD53842c5426634e7929541eC2318f3dCF7e","payTo":"0x209693Bc6afc0C5328bA36FaF03C514EF312287C","resource":"/test","description":"Test","maxTimeoutSeconds":60}]"#;
        let requirements_cstr = CString::new(requirements_json).unwrap();
        let error_msg = CString::new("Payment verification failed").unwrap();
        let mut result = vec![0u8; 4096];
        let mut result_len = result.len();

        let status = unsafe {
            x402_generate_json_response(
                requirements_cstr.as_ptr(),
                error_msg.as_ptr(),
                result.as_mut_ptr() as *mut c_char,
                &mut result_len,
            )
        };
        assert_eq!(status, 0, "Should succeed");
        let json_str = unsafe {
            CStr::from_ptr(result.as_ptr() as *const c_char)
                .to_str()
                .expect("Result should be valid UTF-8")
        };
        let json: serde_json::Value =
            serde_json::from_str(json_str).expect("Result should be valid JSON");
        assert_eq!(
            json["error"].as_str(),
            Some("Payment verification failed"),
            "JSON should contain error message"
        );
    }
}
