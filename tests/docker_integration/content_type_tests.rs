//! Content type and response format tests
//!
//! This module tests how the x402 module handles different content types and response formats.
//!
//! # Test Categories
//!
//! - Content-Type: application/json requests - should return JSON response
//! - Browser requests without Content-Type - should return HTML response
//! - Response format detection based on request headers
//!
//! # Background
//!
//! The x402 module should return different response formats based on the request:
//! - JSON responses for API clients (Content-Type: application/json or Accept: application/json)
//! - HTML responses for browsers (browser User-Agent without JSON Content-Type)
//!
//! This ensures that:
//! - API clients receive machine-readable JSON responses
//! - Browsers receive human-readable HTML payment pages
//! - The correct format is detected based on request headers

#[cfg(feature = "integration-test")]
mod tests {
    use super::common::*;

    #[test]
    #[ignore = "requires Docker"]
    fn test_content_type_json_returns_json_response() {
        // Test Case: Content-Type: application/json should return JSON response, not HTML
        //
        // This test verifies the fix for issue where API requests with browser User-Agent
        // were incorrectly returning HTML instead of JSON.
        //
        // Expected behavior:
        // - Request with Content-Type: application/json should return JSON response
        // - Response should start with '{' or '['
        // - Response should contain JSON structure (e.g., "accepts" or "error" field)
        // - Response should NOT contain HTML tags

        if !ensure_container_running() {
            eprintln!("Failed to start container. Skipping test.");
            return;
        }

        // Make request with Content-Type: application/json and browser User-Agent
        // This simulates a browser making an API request (e.g., fetch() with JSON)
        let response_body = http_request_with_headers(
            "/api/protected",
            &[
                ("Content-Type", "application/json"),
                (
                    "User-Agent",
                    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.0.0 Safari/537.36",
                ),
            ],
        )
        .expect("Failed to make HTTP request");

        // Verify response is JSON, not HTML
        assert!(
            response_body.trim_start().starts_with('{')
                || response_body.trim_start().starts_with('['),
            "Response should be JSON, but got: {}",
            response_body.chars().take(200).collect::<String>()
        );

        // Verify response contains JSON structure (should have "accepts" array for payment requirements)
        assert!(
            response_body.contains("\"accepts\"") || response_body.contains("\"error\""),
            "Response should contain JSON structure with 'accepts' or 'error' field"
        );

        // Verify response does NOT contain HTML tags
        assert!(
            !response_body.contains("<!DOCTYPE") && !response_body.contains("<html"),
            "Response should not contain HTML, but got HTML content"
        );

        println!("✓ Content-Type: application/json correctly returns JSON response");
    }

    #[test]
    #[ignore = "requires Docker"]
    fn test_content_type_json_without_user_agent() {
        // Test Case: Content-Type: application/json without User-Agent should return JSON
        //
        // This test verifies that Content-Type header alone is sufficient to trigger JSON response,
        // even without a User-Agent header.
        //
        // Expected behavior:
        // - Request with only Content-Type: application/json should return JSON response
        // - Response should start with '{' or '['
        // - This ensures API clients without User-Agent headers still get JSON responses

        if !ensure_container_running() {
            eprintln!("Failed to start container. Skipping test.");
            return;
        }

        // Make request with only Content-Type: application/json (no User-Agent)
        let response_body =
            http_request_with_headers("/api/protected", &[("Content-Type", "application/json")])
                .expect("Failed to make HTTP request");

        // Verify response is JSON
        assert!(
            response_body.trim_start().starts_with('{')
                || response_body.trim_start().starts_with('['),
            "Response should be JSON, but got: {}",
            response_body.chars().take(200).collect::<String>()
        );

        println!("✓ Content-Type: application/json (no User-Agent) correctly returns JSON");
    }

    #[test]
    #[ignore = "requires Docker"]
    fn test_browser_request_without_content_type_returns_html() {
        // Test Case: Browser request without Content-Type should return HTML
        //
        // This ensures we didn't break the existing browser behavior.
        // Browser requests (with browser User-Agent) without Content-Type header should
        // receive HTML payment pages for better user experience.
        //
        // Expected behavior:
        // - Request with browser User-Agent but no Content-Type should return HTML
        // - Response should contain HTML tags (<!DOCTYPE or <html>)
        // - This ensures browsers get human-readable payment pages

        if !ensure_container_running() {
            eprintln!("Failed to start container. Skipping test.");
            return;
        }

        // Make request with browser User-Agent but no Content-Type
        let response_body = http_request_with_headers(
            "/api/protected",
            &[(
                "User-Agent",
                "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.0.0 Safari/537.36",
            )],
        )
        .expect("Failed to make HTTP request");

        // Verify response is HTML
        assert!(
            response_body.contains("<!DOCTYPE") || response_body.contains("<html"),
            "Browser request without Content-Type should return HTML, but got: {}",
            response_body.chars().take(200).collect::<String>()
        );

        println!("✓ Browser request (no Content-Type) correctly returns HTML");
    }
}

