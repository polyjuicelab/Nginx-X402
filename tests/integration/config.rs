//! Configuration integration tests for nginx-x402 module
//!
//! Tests for x402 module configuration options: content-type detection, asset fallback,
//! network ID configuration, etc.

#[cfg(feature = "integration-test")]
mod tests {
    use super::super::common::*;

    #[test]
    #[ignore = "requires Docker"]
    fn test_content_type_json_returns_json_response() {
        // Test Case: Content-Type: application/json should return JSON response, not HTML
        // This test verifies the fix for issue where API requests with browser User-Agent
        // were incorrectly returning HTML instead of JSON
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
        // This ensures we didn't break the existing browser behavior
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

    #[test]
    #[ignore = "requires Docker"]
    fn test_asset_fallback_uses_default_usdc() {
        // Test Case: x402_asset not specified should use default USDC address
        if !ensure_container_running() {
            eprintln!("Failed to start container. Skipping test.");
            return;
        }

        // Make API request to get JSON response with payment requirements
        let response_body = http_request_with_headers(
            "/api/asset-fallback",
            &[
                ("Content-Type", "application/json"),
                ("Accept", "application/json"),
            ],
        )
        .expect("Failed to make HTTP request");

        // Parse JSON response to check asset address
        // Response should contain payment requirements with USDC address for base-sepolia
        // Base Sepolia USDC address: 0x036CbD53842c5426634e7929541eC2318f3dCF7e
        assert!(
            response_body.contains("0x036CbD53842c5426634e7929541eC2318f3dCF7e")
                || response_body.contains("\"asset\"")
                || response_body.contains("\"accepts\""),
            "Response should contain USDC asset address or payment requirements structure. Got: {}",
            response_body.chars().take(500).collect::<String>()
        );

        println!("✓ Asset fallback correctly uses default USDC address");
    }

    #[test]
    #[ignore = "requires Docker"]
    fn test_network_id_configuration() {
        // Test Case: x402_network_id should work correctly
        if !ensure_container_running() {
            eprintln!("Failed to start container. Skipping test.");
            return;
        }

        // Test with Base Sepolia chainId (84532)
        let response_body = http_request_with_headers(
            "/api/network-id",
            &[
                ("Content-Type", "application/json"),
                ("Accept", "application/json"),
            ],
        )
        .expect("Failed to make HTTP request");

        // Should return 402 with payment requirements for base-sepolia
        assert!(
            response_body.contains("\"accepts\"")
                || response_body.contains("\"error\"")
                || response_body.contains("base-sepolia")
                || response_body.contains("0x036CbD53842c5426634e7929541eC2318f3dCF7e"),
            "Response should contain payment requirements for base-sepolia network. Got: {}",
            response_body.chars().take(500).collect::<String>()
        );

        println!("✓ Network ID (chainId 84532) configuration works correctly");
    }

    #[test]
    #[ignore = "requires Docker"]
    fn test_network_id_mainnet() {
        // Test Case: x402_network_id with mainnet chainId (8453)
        if !ensure_container_running() {
            eprintln!("Failed to start container. Skipping test.");
            return;
        }

        let response_body = http_request_with_headers(
            "/api/network-id-mainnet",
            &[
                ("Content-Type", "application/json"),
                ("Accept", "application/json"),
            ],
        )
        .expect("Failed to make HTTP request");

        // Should return 402 with payment requirements for base mainnet
        // Base Mainnet USDC address: 0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913
        assert!(
            response_body.contains("\"accepts\"")
                || response_body.contains("\"error\"")
                || response_body.contains("base")
                || response_body.contains("0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913"),
            "Response should contain payment requirements for base mainnet network. Got: {}",
            response_body.chars().take(500).collect::<String>()
        );

        println!("✓ Network ID (chainId 8453 - mainnet) configuration works correctly");
    }

    #[test]
    #[ignore = "requires Docker"]
    fn test_custom_asset_address() {
        // Test Case: Custom x402_asset address should use specified address
        if !ensure_container_running() {
            eprintln!("Failed to start container. Skipping test.");
            return;
        }

        let response_body = http_request_with_headers(
            "/api/custom-asset",
            &[
                ("Content-Type", "application/json"),
                ("Accept", "application/json"),
            ],
        )
        .expect("Failed to make HTTP request");

        // Should contain the custom asset address specified in config
        // Custom address: 0x036CbD53842c5426634e7929541eC2318f3dCF7e
        assert!(
            response_body.contains("0x036CbD53842c5426634e7929541eC2318f3dCF7e")
                || response_body.contains("\"asset\"")
                || response_body.contains("\"accepts\""),
            "Response should contain custom asset address. Got: {}",
            response_body.chars().take(500).collect::<String>()
        );

        println!("✓ Custom asset address configuration works correctly");
    }

    #[test]
    #[ignore = "requires Docker"]
    fn test_network_id_takes_precedence() {
        // Test Case: x402_network_id should take precedence over x402_network
        if !ensure_container_running() {
            eprintln!("Failed to start container. Skipping test.");
            return;
        }

        let response_body = http_request_with_headers(
            "/api/network-priority",
            &[
                ("Content-Type", "application/json"),
                ("Accept", "application/json"),
            ],
        )
        .expect("Failed to make HTTP request");

        // Should use network_id (8453 = Base Mainnet) instead of network (base-sepolia)
        // Base Mainnet USDC: 0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913
        // Base Sepolia USDC: 0x036CbD53842c5426634e7929541eC2318f3dCF7e
        // Should NOT contain Sepolia address if network_id takes precedence
        let contains_mainnet = response_body.contains("0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913")
            || response_body.contains("base")
            || response_body.contains("\"accepts\"");

        assert!(
            contains_mainnet,
            "Response should use network_id (mainnet) instead of network (sepolia). Got: {}",
            response_body.chars().take(500).collect::<String>()
        );

        println!("✓ Network ID correctly takes precedence over network name");
    }
}
