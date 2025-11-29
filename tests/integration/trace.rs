//! TRACE request integration tests for nginx-x402 module
//!
//! Tests for TRACE request handling and payment verification skipping.

#[cfg(feature = "integration-test")]
mod tests {
    use super::super::common::*;

    #[test]
    #[ignore = "requires Docker"]
    fn test_trace_request_skips_payment() {
        // Test Case: TRACE request should skip payment verification
        // TRACE requests are used for diagnostic and debugging purposes
        if !ensure_container_running() {
            eprintln!("Failed to start container. Skipping test.");
            return;
        }

        // Test TRACE request without payment header
        let status = http_request_with_method("/api/protected", "TRACE", &[])
            .expect("Failed to make TRACE request");

        println!("TRACE request status (no payment): {status}");

        // TRACE request should succeed or return appropriate status without payment verification
        // It should NOT return 402, which would indicate payment verification was attempted
        // Since /api/protected has no proxy_pass, x402 module should send 405 response for TRACE
        // (TRACE is often disabled for security, so 405 Method Not Allowed is expected)
        assert!(
            status == "200" || status == "404" || status == "405" || status == "204",
            "TRACE request should skip payment verification and return appropriate status, got {status}. \
             Expected 405 from x402 module for TRACE requests without proxy_pass."
        );

        // Verify that TRACE request does not require payment
        assert_ne!(
            status, "402",
            "TRACE request should not require payment (got 402). \
             Payment verification should be skipped for TRACE requests."
        );

        println!("✓ TRACE request correctly skipped payment verification");
    }
}
