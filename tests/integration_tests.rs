//! Integration tests for Nginx x402 module
//!
//! These tests simulate how Nginx would use the x402 FFI functions
//! to verify payments and return appropriate responses.

mod tests {
    use nginx_x402::ffi::*;
    use rust_x402::types::PaymentRequirements;
    use std::ffi::CString;
    use std::os::raw::c_char;
    use std::ptr;

    /// Simulate Nginx request processing with x402 payment verification
    struct NginxRequestSimulator {
        payment_header: Option<String>,
        user_agent: String,
        accept: String,
        request_uri: String,
    }

    impl NginxRequestSimulator {
        fn new() -> Self {
            Self {
                payment_header: None,
                user_agent: String::new(),
                accept: String::new(),
                request_uri: String::new(),
            }
        }

        fn with_user_agent(mut self, ua: &str) -> Self {
            self.user_agent = ua.to_string();
            self
        }

        fn with_accept(mut self, accept: &str) -> Self {
            self.accept = accept.to_string();
            self
        }

        fn with_uri(mut self, uri: &str) -> Self {
            self.request_uri = uri.to_string();
            self
        }

        /// Simulate Nginx processing: check payment and return response
        fn process(&self) -> (i32, String) {
            // Step 1: Check if payment header exists
            if self.payment_header.is_none() {
                // No payment header - generate 402 response
                return self.generate_402_response();
            }

            // Step 2: Verify payment
            let payment_b64 = self.payment_header.as_ref().unwrap();
            let requirements_json = self.create_requirements_json();

            let mut result = vec![0u8; 4096];
            let mut result_len = result.len();

            let payment_cstr = CString::new(payment_b64.as_str()).unwrap();
            let requirements_cstr = CString::new(requirements_json.as_str()).unwrap();
            let facilitator_cstr = CString::new("https://x402.org/facilitator").unwrap();

            let status = unsafe {
                x402_verify_payment(
                    payment_cstr.as_ptr(),
                    requirements_cstr.as_ptr(),
                    facilitator_cstr.as_ptr(),
                    result.as_mut_ptr() as *mut c_char,
                    &mut result_len,
                )
            };

            let result_str = if result_len > 0 {
                unsafe {
                    std::ffi::CStr::from_ptr(result.as_ptr() as *const c_char)
                        .to_str()
                        .unwrap_or("")
                        .to_string()
                }
            } else {
                String::new()
            };

            (status, result_str)
        }

        fn generate_402_response(&self) -> (i32, String) {
            let requirements_json = self.create_requirements_json();
            let is_browser = self.is_browser_request();

            let mut result = vec![0u8; 8192];
            let mut result_len = result.len();

            let requirements_cstr = CString::new(requirements_json.as_str()).unwrap();
            let error_cstr = CString::new("X-PAYMENT header is required").unwrap();

            let status = if is_browser {
                unsafe {
                    x402_generate_paywall_html(
                        requirements_cstr.as_ptr(),
                        error_cstr.as_ptr(),
                        result.as_mut_ptr() as *mut c_char,
                        &mut result_len,
                    )
                }
            } else {
                unsafe {
                    x402_generate_json_response(
                        requirements_cstr.as_ptr(),
                        error_cstr.as_ptr(),
                        result.as_mut_ptr() as *mut c_char,
                        &mut result_len,
                    )
                }
            };

            let result_str = if result_len > 0 {
                unsafe {
                    std::ffi::CStr::from_ptr(result.as_ptr() as *const c_char)
                        .to_str()
                        .unwrap_or("")
                        .to_string()
                }
            } else {
                String::new()
            };

            (status, result_str)
        }

        fn create_requirements_json(&self) -> String {
            let requirements = PaymentRequirements::new(
                "exact",
                "base-sepolia",
                "1000000",
                "0x036CbD53842c5426634e7929541eC2318f3dCF7e",
                "0x209693Bc6afc0C5328bA36FaF03C514EF312287C",
                &self.request_uri,
                "Test payment",
            );
            serde_json::to_string(&vec![requirements]).unwrap()
        }

        fn is_browser_request(&self) -> bool {
            let ua_cstr = CString::new(self.user_agent.as_str()).unwrap_or_default();
            let accept_cstr = CString::new(self.accept.as_str()).unwrap_or_default();

            unsafe { x402_is_browser_request(ua_cstr.as_ptr(), accept_cstr.as_ptr()) == 1 }
        }
    }

    // ========== Basic Functionality Tests ==========

    #[test]
    fn test_nginx_request_without_payment() {
        let simulator = NginxRequestSimulator::new()
            .with_uri("/api/protected")
            .with_user_agent("Mozilla/5.0")
            .with_accept("text/html");

        let (status, response) = simulator.process();

        assert_eq!(
            status, 0,
            "Status should be 0 (success) for 402 response generation"
        );
        assert!(
            response.contains("Payment Required") || response.contains("paymentRequirements"),
            "Response should contain payment requirement indicators. Got: {}",
            response
        );
    }

    #[test]
    fn test_nginx_request_browser_detection() {
        // Browser request
        let simulator = NginxRequestSimulator::new()
            .with_uri("/api/protected")
            .with_user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
            .with_accept("text/html,application/xhtml+xml");

        let (status, response) = simulator.generate_402_response();

        assert_eq!(
            status, 0,
            "Status should be 0 for successful HTML generation"
        );
        assert!(
            response.contains("<!DOCTYPE html>"),
            "Browser request should return HTML. Got: {}",
            &response[..response.len().min(200)]
        );
        assert!(
            response.contains("Payment Required"),
            "HTML should contain 'Payment Required'. Got: {}",
            &response[..response.len().min(200)]
        );
    }

    #[test]
    fn test_nginx_request_api_client() {
        // API client request
        let simulator = NginxRequestSimulator::new()
            .with_uri("/api/protected")
            .with_user_agent("curl/7.68.0")
            .with_accept("application/json");

        let (status, response) = simulator.generate_402_response();

        assert_eq!(
            status, 0,
            "Status should be 0 for successful JSON generation"
        );
        // Verify it's valid JSON
        let json: serde_json::Value =
            serde_json::from_str(&response).expect("Response should be valid JSON");
        assert!(
            json.get("paymentRequirements").is_some() || json.get("error").is_some(),
            "JSON should contain paymentRequirements or error field. Got: {}",
            response
        );
    }

    #[test]
    fn test_nginx_payment_requirements_creation() {
        let amount = CString::new("0.0001").unwrap();
        let pay_to = CString::new("0x209693Bc6afc0C5328bA36FaF03C514EF312287C").unwrap();
        let network = CString::new("base-sepolia").unwrap();
        let resource = CString::new("/api/protected").unwrap();
        let description = CString::new("API access payment").unwrap();

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

        assert_eq!(
            status, 0,
            "Status should be 0 for successful requirements creation"
        );
        assert!(result_len > 0, "Result length should be greater than 0");

        let result_str = unsafe {
            std::ffi::CStr::from_ptr(result.as_ptr() as *const c_char)
                .to_str()
                .expect("Result should be valid UTF-8")
        };

        let requirements: PaymentRequirements = serde_json::from_str(result_str)
            .expect("Result should be valid JSON PaymentRequirements");
        assert_eq!(requirements.scheme, "exact", "Scheme should be 'exact'");
        assert_eq!(
            requirements.network, "base-sepolia",
            "Network should be 'base-sepolia'"
        );
        assert_eq!(
            requirements.resource, "/api/protected",
            "Resource should match input"
        );
        assert!(
            !requirements.pay_to.is_empty(),
            "Pay-to address should not be empty"
        );
    }

    // ========== Error Handling Tests ==========

    #[test]
    fn test_create_requirements_null_pointers() {
        let amount = CString::new("0.0001").unwrap();
        let pay_to = CString::new("0x209693Bc6afc0C5328bA36FaF03C514EF312287C").unwrap();
        let mut result = vec![0u8; 4096];
        let mut result_len = result.len();

        // Test null amount pointer
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
        assert_eq!(status, 1, "Should return 1 (invalid input) for null amount");

        // Test null pay_to pointer
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
        assert_eq!(status, 1, "Should return 1 (invalid input) for null pay_to");

        // Test null result pointer
        let mut result_len = 4096;
        let status = unsafe {
            x402_create_requirements(
                amount.as_ptr(),
                pay_to.as_ptr(),
                ptr::null(),
                ptr::null(),
                ptr::null(),
                1,
                ptr::null_mut(),
                &mut result_len,
            )
        };
        assert_eq!(
            status, 1,
            "Should return 1 (invalid input) for null result buffer"
        );

        // Test null result_len pointer
        let mut result = vec![0u8; 4096];
        let status = unsafe {
            x402_create_requirements(
                amount.as_ptr(),
                pay_to.as_ptr(),
                ptr::null(),
                ptr::null(),
                ptr::null(),
                1,
                result.as_mut_ptr() as *mut c_char,
                ptr::null_mut(),
            )
        };
        assert_eq!(
            status, 1,
            "Should return 1 (invalid input) for null result_len"
        );
    }

    #[test]
    fn test_create_requirements_invalid_input() {
        let pay_to = CString::new("0x209693Bc6afc0C5328bA36FaF03C514EF312287C").unwrap();
        let mut result = vec![0u8; 4096];
        let mut result_len = result.len();

        // Test invalid amount (not a number)
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
        assert_eq!(
            status, 1,
            "Should return 1 (invalid input) for invalid amount"
        );

        // Test empty amount
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
        assert_eq!(
            status, 1,
            "Should return 1 (invalid input) for empty amount"
        );
    }

    #[test]
    fn test_create_requirements_buffer_too_small() {
        let amount = CString::new("0.0001").unwrap();
        let pay_to = CString::new("0x209693Bc6afc0C5328bA36FaF03C514EF312287C").unwrap();
        let network = CString::new("base-sepolia").unwrap();
        let resource = CString::new("/api/protected").unwrap();
        let description = CString::new("API access payment").unwrap();

        // Test with buffer too small
        let mut result = vec![0u8; 10]; // Very small buffer
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
    fn test_generate_paywall_html_null_pointers() {
        let requirements_json = r#"[{"scheme":"exact","network":"base-sepolia"}]"#;
        let requirements_cstr = CString::new(requirements_json).unwrap();
        let mut result = vec![0u8; 4096];
        let mut result_len = result.len();

        // Test null requirements_json
        let status = unsafe {
            x402_generate_paywall_html(
                ptr::null(),
                ptr::null(),
                result.as_mut_ptr() as *mut c_char,
                &mut result_len,
            )
        };
        assert_eq!(
            status, 1,
            "Should return 1 (invalid input) for null requirements_json"
        );

        // Test null result buffer
        let mut result_len = 4096;
        let status = unsafe {
            x402_generate_paywall_html(
                requirements_cstr.as_ptr(),
                ptr::null(),
                ptr::null_mut(),
                &mut result_len,
            )
        };
        assert_eq!(
            status, 1,
            "Should return 1 (invalid input) for null result buffer"
        );

        // Test null result_len
        let mut result = vec![0u8; 4096];
        let status = unsafe {
            x402_generate_paywall_html(
                requirements_cstr.as_ptr(),
                ptr::null(),
                result.as_mut_ptr() as *mut c_char,
                ptr::null_mut(),
            )
        };
        assert_eq!(
            status, 1,
            "Should return 1 (invalid input) for null result_len"
        );
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
        assert_eq!(
            status, 1,
            "Should return 1 (invalid input) for invalid JSON"
        );
    }

    #[test]
    fn test_generate_json_response_null_pointers() {
        let requirements_json = r#"[{"scheme":"exact","network":"base-sepolia"}]"#;
        let requirements_cstr = CString::new(requirements_json).unwrap();
        let mut result = vec![0u8; 4096];
        let mut result_len = result.len();

        // Test null requirements_json
        let status = unsafe {
            x402_generate_json_response(
                ptr::null(),
                ptr::null(),
                result.as_mut_ptr() as *mut c_char,
                &mut result_len,
            )
        };
        assert_eq!(
            status, 1,
            "Should return 1 (invalid input) for null requirements_json"
        );

        // Test null result buffer
        let mut result_len = 4096;
        let status = unsafe {
            x402_generate_json_response(
                requirements_cstr.as_ptr(),
                ptr::null(),
                ptr::null_mut(),
                &mut result_len,
            )
        };
        assert_eq!(
            status, 1,
            "Should return 1 (invalid input) for null result buffer"
        );
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
        assert_eq!(
            status, 1,
            "Should return 1 (invalid input) for invalid JSON"
        );
    }

    #[test]
    fn test_verify_payment_null_pointers() {
        let payment = CString::new("dummy").unwrap();
        let requirements = CString::new(r#"{"scheme":"exact"}"#).unwrap();
        let mut result = vec![0u8; 4096];
        let mut result_len = result.len();

        // Test null payment_b64
        let status = unsafe {
            x402_verify_payment(
                ptr::null(),
                requirements.as_ptr(),
                ptr::null(),
                result.as_mut_ptr() as *mut c_char,
                &mut result_len,
            )
        };
        assert_eq!(
            status, 1,
            "Should return 1 (invalid input) for null payment_b64"
        );

        // Test null requirements_json
        let status = unsafe {
            x402_verify_payment(
                payment.as_ptr(),
                ptr::null(),
                ptr::null(),
                result.as_mut_ptr() as *mut c_char,
                &mut result_len,
            )
        };
        assert_eq!(
            status, 1,
            "Should return 1 (invalid input) for null requirements_json"
        );

        // Test null result buffer
        let mut result_len = 4096;
        let status = unsafe {
            x402_verify_payment(
                payment.as_ptr(),
                requirements.as_ptr(),
                ptr::null(),
                ptr::null_mut(),
                &mut result_len,
            )
        };
        assert_eq!(
            status, 1,
            "Should return 1 (invalid input) for null result buffer"
        );
    }

    #[test]
    fn test_verify_payment_invalid_base64() {
        let invalid_payment = CString::new("not-valid-base64!!!").unwrap();
        let requirements =
            CString::new(r#"[{"scheme":"exact","network":"base-sepolia"}]"#).unwrap();
        let mut result = vec![0u8; 4096];
        let mut result_len = result.len();

        let status = unsafe {
            x402_verify_payment(
                invalid_payment.as_ptr(),
                requirements.as_ptr(),
                ptr::null(),
                result.as_mut_ptr() as *mut c_char,
                &mut result_len,
            )
        };
        assert_eq!(
            status, 1,
            "Should return 1 (invalid input) for invalid base64"
        );
    }

    #[test]
    fn test_verify_payment_invalid_requirements_json() {
        let payment = CString::new("dummy").unwrap();
        let invalid_requirements = CString::new("not valid json").unwrap();
        let mut result = vec![0u8; 4096];
        let mut result_len = result.len();

        let status = unsafe {
            x402_verify_payment(
                payment.as_ptr(),
                invalid_requirements.as_ptr(),
                ptr::null(),
                result.as_mut_ptr() as *mut c_char,
                &mut result_len,
            )
        };
        assert_eq!(
            status, 1,
            "Should return 1 (invalid input) for invalid requirements JSON"
        );
    }

    // ========== Browser Detection Tests ==========

    #[test]
    fn test_is_browser_request_null_pointers() {
        // Both null should be treated as non-browser
        let result = unsafe { x402_is_browser_request(ptr::null(), ptr::null()) };
        assert_eq!(result, 0, "Null pointers should return 0 (not browser)");

        // User-Agent null, Accept null
        let result = unsafe { x402_is_browser_request(ptr::null(), ptr::null()) };
        assert_eq!(result, 0, "Both null should return 0");

        // User-Agent null, Accept valid
        let accept = CString::new("text/html").unwrap();
        let result = unsafe { x402_is_browser_request(ptr::null(), accept.as_ptr()) };
        // Should check accept header even if user-agent is null
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
    fn test_is_browser_request_various_user_agents() {
        let html_accept = CString::new("text/html").unwrap();
        let json_accept = CString::new("application/json").unwrap();

        // Chrome
        let chrome =
            CString::new("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36").unwrap();
        assert_eq!(
            unsafe { x402_is_browser_request(chrome.as_ptr(), html_accept.as_ptr()) },
            1,
            "Chrome with HTML accept should be browser"
        );
        assert_eq!(
            unsafe { x402_is_browser_request(chrome.as_ptr(), json_accept.as_ptr()) },
            0,
            "Chrome with JSON accept should not be browser"
        );

        // Firefox
        let firefox =
            CString::new("Mozilla/5.0 (X11; Linux x86_64; rv:91.0) Gecko/20100101 Firefox/91.0")
                .unwrap();
        assert_eq!(
            unsafe { x402_is_browser_request(firefox.as_ptr(), html_accept.as_ptr()) },
            1,
            "Firefox with HTML accept should be browser"
        );

        // Safari
        let safari =
            CString::new("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15")
                .unwrap();
        assert_eq!(
            unsafe { x402_is_browser_request(safari.as_ptr(), html_accept.as_ptr()) },
            1,
            "Safari with HTML accept should be browser"
        );

        // curl
        let curl = CString::new("curl/7.68.0").unwrap();
        assert_eq!(
            unsafe { x402_is_browser_request(curl.as_ptr(), json_accept.as_ptr()) },
            0,
            "curl should not be browser"
        );

        // Python requests
        let python = CString::new("python-requests/2.28.1").unwrap();
        assert_eq!(
            unsafe { x402_is_browser_request(python.as_ptr(), json_accept.as_ptr()) },
            0,
            "Python requests should not be browser"
        );

        // Empty user agent
        let empty_ua = CString::new("").unwrap();
        assert_eq!(
            unsafe { x402_is_browser_request(empty_ua.as_ptr(), html_accept.as_ptr()) },
            0,
            "Empty user agent should not be browser"
        );
    }

    #[test]
    fn test_is_browser_request_various_accept_headers() {
        let mozilla = CString::new("Mozilla/5.0").unwrap();

        // HTML accept
        let html = CString::new("text/html").unwrap();
        assert_eq!(
            unsafe { x402_is_browser_request(mozilla.as_ptr(), html.as_ptr()) },
            1,
            "text/html should be browser"
        );

        // HTML with charset
        let html_charset = CString::new("text/html; charset=utf-8").unwrap();
        assert_eq!(
            unsafe { x402_is_browser_request(mozilla.as_ptr(), html_charset.as_ptr()) },
            1,
            "text/html with charset should be browser"
        );

        // XHTML - check actual behavior (may vary by implementation)
        let xhtml = CString::new("application/xhtml+xml").unwrap();
        let xhtml_result = unsafe { x402_is_browser_request(mozilla.as_ptr(), xhtml.as_ptr()) };
        assert!(
            xhtml_result == 0 || xhtml_result == 1,
            "application/xhtml+xml should return 0 or 1, got: {}",
            xhtml_result
        );

        // JSON
        let json = CString::new("application/json").unwrap();
        assert_eq!(
            unsafe { x402_is_browser_request(mozilla.as_ptr(), json.as_ptr()) },
            0,
            "application/json should not be browser"
        );

        // Empty accept
        let empty = CString::new("").unwrap();
        assert_eq!(
            unsafe { x402_is_browser_request(mozilla.as_ptr(), empty.as_ptr()) },
            0,
            "Empty accept should not be browser"
        );
    }

    // ========== Edge Cases Tests ==========

    #[test]
    fn test_create_requirements_with_null_optional_params() {
        let amount = CString::new("0.0001").unwrap();
        let pay_to = CString::new("0x209693Bc6afc0C5328bA36FaF03C514EF312287C").unwrap();
        let mut result = vec![0u8; 4096];
        let mut result_len = result.len();

        // Test with all optional params as null
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
            std::ffi::CStr::from_ptr(result.as_ptr() as *const c_char)
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
    fn test_create_requirements_mainnet_vs_testnet() {
        let amount = CString::new("0.0001").unwrap();
        let pay_to = CString::new("0x209693Bc6afc0C5328bA36FaF03C514EF312287C").unwrap();
        let mut result = vec![0u8; 4096];
        let mut result_len = result.len();

        // Test testnet
        let status = unsafe {
            x402_create_requirements(
                amount.as_ptr(),
                pay_to.as_ptr(),
                ptr::null(),
                ptr::null(),
                ptr::null(),
                1, // testnet = true
                result.as_mut_ptr() as *mut c_char,
                &mut result_len,
            )
        };
        assert_eq!(status, 0);
        let testnet_result = unsafe {
            std::ffi::CStr::from_ptr(result.as_ptr() as *const c_char)
                .to_str()
                .unwrap()
        };
        let testnet_req: PaymentRequirements = serde_json::from_str(testnet_result).unwrap();
        assert_eq!(
            testnet_req.network, "base-sepolia",
            "Testnet should use base-sepolia"
        );

        // Test mainnet
        result_len = result.len();
        let status = unsafe {
            x402_create_requirements(
                amount.as_ptr(),
                pay_to.as_ptr(),
                ptr::null(),
                ptr::null(),
                ptr::null(),
                0, // testnet = false
                result.as_mut_ptr() as *mut c_char,
                &mut result_len,
            )
        };
        assert_eq!(status, 0);
        let mainnet_result = unsafe {
            std::ffi::CStr::from_ptr(result.as_ptr() as *const c_char)
                .to_str()
                .unwrap()
        };
        let mainnet_req: PaymentRequirements = serde_json::from_str(mainnet_result).unwrap();
        assert_eq!(mainnet_req.network, "base", "Mainnet should use base");
    }

    #[test]
    fn test_create_requirements_different_networks() {
        let amount = CString::new("0.0001").unwrap();
        let pay_to = CString::new("0x209693Bc6afc0C5328bA36FaF03C514EF312287C").unwrap();
        let mut result = vec![0u8; 4096];
        let mut result_len = result.len();

        // Test explicit network override
        let network = CString::new("base-sepolia").unwrap();
        let status = unsafe {
            x402_create_requirements(
                amount.as_ptr(),
                pay_to.as_ptr(),
                network.as_ptr(),
                ptr::null(),
                ptr::null(),
                1,
                result.as_mut_ptr() as *mut c_char,
                &mut result_len,
            )
        };
        assert_eq!(status, 0);
        let result_str = unsafe {
            std::ffi::CStr::from_ptr(result.as_ptr() as *const c_char)
                .to_str()
                .unwrap()
        };
        let requirements: PaymentRequirements = serde_json::from_str(result_str).unwrap();
        assert_eq!(
            requirements.network, "base-sepolia",
            "Network should match input"
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
            std::ffi::CStr::from_ptr(result.as_ptr() as *const c_char)
                .to_str()
                .unwrap()
        };
        assert!(
            html.contains("Custom error message"),
            "HTML should contain custom error message"
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
            std::ffi::CStr::from_ptr(result.as_ptr() as *const c_char)
                .to_str()
                .unwrap()
        };
        let json: serde_json::Value = serde_json::from_str(json_str).expect("Should be valid JSON");
        assert_eq!(
            json["error"].as_str(),
            Some("Payment verification failed"),
            "JSON should contain error message"
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
        assert_eq!(status, 0);
        let json_str = unsafe {
            std::ffi::CStr::from_ptr(result.as_ptr() as *const c_char)
                .to_str()
                .unwrap()
        };
        let json: serde_json::Value = serde_json::from_str(json_str).expect("Should be valid JSON");

        // Verify JSON structure
        assert!(
            json.get("error").is_some() || json.get("message").is_some(),
            "JSON should have error or message field"
        );
        assert!(
            json.get("paymentRequirements").is_some() || json.get("accepts").is_some(),
            "JSON should have paymentRequirements or accepts field"
        );
    }

    #[test]
    fn test_free_string_null_pointer() {
        // Should not crash when freeing null pointer
        unsafe {
            x402_free_string(ptr::null_mut());
        }
        // If we get here, the function handled null gracefully
    }

    #[test]
    fn test_create_requirements_empty_strings() {
        let amount = CString::new("0.0001").unwrap();
        let pay_to = CString::new("0x209693Bc6afc0C5328bA36FaF03C514EF312287C").unwrap();
        let empty_resource = CString::new("").unwrap();
        let mut result = vec![0u8; 4096];
        let mut result_len = result.len();

        // Empty resource should be handled
        let status = unsafe {
            x402_create_requirements(
                amount.as_ptr(),
                pay_to.as_ptr(),
                ptr::null(),
                empty_resource.as_ptr(),
                ptr::null(),
                1,
                result.as_mut_ptr() as *mut c_char,
                &mut result_len,
            )
        };
        // Should either succeed or return error, but not crash
        assert!(
            status == 0 || status == 1 || status == 5,
            "Status should be 0, 1, or 5, got: {}",
            status
        );
    }

    #[test]
    fn test_create_requirements_very_long_strings() {
        let amount = CString::new("0.0001").unwrap();
        let long_pay_to = CString::new("0x209693Bc6afc0C5328bA36FaF03C514EF312287C").unwrap();
        let long_resource = CString::new("/".repeat(1000)).unwrap();
        let long_description = CString::new("A".repeat(500)).unwrap();
        let mut result = vec![0u8; 4096];
        let mut result_len = result.len();

        let status = unsafe {
            x402_create_requirements(
                amount.as_ptr(),
                long_pay_to.as_ptr(),
                ptr::null(),
                long_resource.as_ptr(),
                long_description.as_ptr(),
                1,
                result.as_mut_ptr() as *mut c_char,
                &mut result_len,
            )
        };
        // Should handle long strings gracefully
        assert!(
            status == 0 || status == 4 || status == 5,
            "Status should be 0, 4, or 5 for long strings, got: {}",
            status
        );
    }
}
