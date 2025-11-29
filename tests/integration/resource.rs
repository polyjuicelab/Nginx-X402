//! Resource URL validation integration tests for nginx-x402 module
//!
//! Tests for resource field URL validation in payment requirements.

#[cfg(feature = "integration-test")]
mod tests {
    use super::super::common::*;

    #[test]
    #[ignore = "requires Docker"]
    fn test_resource_url_is_valid_full_url() {
        // Test Case: Verify that resource field in payment requirements is a valid full URL
        // The resource field should be a complete URL (http:// or https://) not a relative path
        if !ensure_container_running() {
            eprintln!("Failed to start container. Skipping test.");
            return;
        }

        // Make API request to get JSON response with payment requirements
        let response_body = http_request_with_headers(
            "/api/protected",
            &[
                ("Content-Type", "application/json"),
                ("Accept", "application/json"),
            ],
        )
        .expect("Failed to make HTTP request");

        // Parse JSON response to check resource field
        // Response should be JSON with payment requirements
        assert!(
            response_body.trim_start().starts_with('{'),
            "Response should be JSON, but got: {}",
            response_body.chars().take(200).collect::<String>()
        );

        // Check if response contains "accepts" array (payment requirements)
        if response_body.contains("\"accepts\"") {
            // Extract resource field value from JSON response
            // Look for "resource":"..." pattern using simple string matching
            let mut resource_value: Option<String> = None;

            // Look for resource field with various patterns
            let patterns = vec!["\"resource\":\"", "\"resource\" : \"", "\"resource\": \""];
            for pattern in patterns {
                if let Some(idx) = response_body.find(pattern) {
                    let start = idx + pattern.len();
                    if let Some(end) = response_body[start..].find('"') {
                        let value = &response_body[start..start + end];
                        resource_value = Some(value.to_string());
                        break;
                    }
                }
            }

            // Validate resource URL
            if let Some(resource) = resource_value {
                // Check if resource is a valid full URL (starts with http:// or https://)
                let is_valid_url =
                    resource.starts_with("http://") || resource.starts_with("https://");

                // Check for double prefix bug (/http:// or /https://)
                let has_double_prefix =
                    resource.starts_with("/http://") || resource.starts_with("/https://");

                assert!(
                    is_valid_url && !has_double_prefix,
                    "Resource field should be a valid full URL (http:// or https://) without double prefix. \
                     Got: '{}'. \
                     Response: {}",
                    resource,
                    response_body.chars().take(500).collect::<String>()
                );

                // Additional validation: ensure URL format is correct
                assert!(
                    resource.len() > 7, // Minimum: "http://a"
                    "Resource URL is too short: '{}'",
                    resource
                );

                println!("✓ Resource field is a valid full URL: {}", resource);
            } else {
                panic!(
                    "Could not extract resource field from response. \
                     Response: {}",
                    response_body.chars().take(500).collect::<String>()
                );
            }
        } else if response_body.contains("\"error\"") {
            // If there's an error, we can't verify resource URL
            // But we should still check that error doesn't mention invalid URL
            assert!(
                !response_body.contains("Invalid url")
                    && !response_body.contains("invalid_string")
                    && !response_body.contains("\"path\":[\"resource\"]"),
                "Response contains URL validation error for resource field. \
                 This suggests resource URL is not valid. Response: {}",
                response_body.chars().take(500).collect::<String>()
            );
            println!("✓ No URL validation errors in error response");
        } else {
            // Unexpected response format
            panic!(
                "Unexpected response format. Expected JSON with 'accepts' or 'error' field. \
                 Got: {}",
                response_body.chars().take(500).collect::<String>()
            );
        }
    }
}
