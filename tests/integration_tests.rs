//! Integration tests for Nginx x402 module
//!
//! These tests simulate how Nginx would use the x402 FFI functions
//! to verify payments and return appropriate responses.

mod tests {
    use nginx_x402::ffi::*;
    use rust_x402::types::PaymentRequirements;
    use std::ffi::CString;
    use std::os::raw::c_char;

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

    #[test]
    fn test_nginx_request_without_payment() {
        let simulator = NginxRequestSimulator::new()
            .with_uri("/api/protected")
            .with_user_agent("Mozilla/5.0")
            .with_accept("text/html");

        let (status, response) = simulator.process();

        // Should generate 402 response
        assert_eq!(status, 0);
        assert!(response.contains("Payment Required") || response.contains("paymentRequirements"));
    }

    #[test]
    fn test_nginx_request_browser_detection() {
        // Browser request
        let simulator = NginxRequestSimulator::new()
            .with_uri("/api/protected")
            .with_user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
            .with_accept("text/html,application/xhtml+xml");

        let (status, response) = simulator.generate_402_response();

        assert_eq!(status, 0);
        assert!(response.contains("<!DOCTYPE html>"));
        assert!(response.contains("Payment Required"));
    }

    #[test]
    fn test_nginx_request_api_client() {
        // API client request
        let simulator = NginxRequestSimulator::new()
            .with_uri("/api/protected")
            .with_user_agent("curl/7.68.0")
            .with_accept("application/json");

        let (status, response) = simulator.generate_402_response();

        assert_eq!(status, 0);
        // Check for JSON response indicators
        assert!(
            response.contains("paymentRequirements")
                || response.contains("accepts")
                || response.contains("base-sepolia"),
            "Response should contain payment requirements: {}",
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

        assert_eq!(status, 0);
        assert!(result_len > 0);

        let result_str = unsafe {
            std::ffi::CStr::from_ptr(result.as_ptr() as *const c_char)
                .to_str()
                .unwrap()
        };

        let requirements: PaymentRequirements = serde_json::from_str(result_str).unwrap();
        assert_eq!(requirements.scheme, "exact");
        assert_eq!(requirements.network, "base-sepolia");
        assert_eq!(requirements.resource, "/api/protected");
    }
}
