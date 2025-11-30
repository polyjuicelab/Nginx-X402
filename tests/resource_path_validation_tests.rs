//! Tests for resource path validation
//!
//! These tests verify that resource path validation works correctly,
//! covering the fix for:
//! - fix-10: Resource path security (preventing path traversal attacks)

mod tests {
    use nginx_x402::validate_resource_path;

    #[test]
    fn test_validate_resource_path_valid() {
        let valid_paths = vec![
            "/",
            "/api",
            "/api/v1",
            "/api/v1/resource",
            "/path/to/resource",
            "/path/to/resource?query=value",
            "/path/to/resource#fragment",
        ];

        for path in valid_paths {
            let result = validate_resource_path(path);
            assert!(result.is_ok(), "Valid path '{path}' should pass validation");
            let sanitized = result.unwrap();
            // Paths should start with /
            assert!(
                sanitized.starts_with('/'),
                "Sanitized path should start with /, got: {sanitized}"
            );
        }
    }

    #[test]
    fn test_validate_resource_path_path_traversal() {
        let dangerous_paths = vec![
            "../",
            "..",
            "../etc/passwd",
            "/../etc/passwd",
            "/api/../etc/passwd",
            "..\\",
            "..\\windows\\system32",
            "/api/..\\windows",
        ];

        for path in dangerous_paths {
            let result = validate_resource_path(path);
            assert!(
                result.is_err(),
                "Dangerous path '{path}' should fail validation"
            );
            let error = result.unwrap_err().to_string();
            assert!(
                error.contains("Configuration error") || error.contains("error"),
                "Error should indicate configuration error, got: {error}"
            );
        }
    }

    #[test]
    fn test_validate_resource_path_null_bytes() {
        let paths_with_null = vec!["/api\0/resource", "/api\0x00/resource", "\0/api"];

        for path in paths_with_null {
            let result = validate_resource_path(path);
            assert!(
                result.is_err(),
                "Path with null byte '{path}' should fail validation"
            );
        }
    }

    #[test]
    fn test_validate_resource_path_empty() {
        let empty_paths = vec!["", "   ", "\t", "\n", "\r\n"];

        for path in empty_paths {
            let result = validate_resource_path(path);
            assert!(
                result.is_err(),
                "Empty path '{path}' should fail validation"
            );
        }
    }

    #[test]
    fn test_validate_resource_path_too_long() {
        // Create a path longer than MAX_PATH_LENGTH (2048)
        let long_path = "/".to_string() + &"a".repeat(2100);
        let result = validate_resource_path(&long_path);
        assert!(
            result.is_err(),
            "Path longer than 2048 characters should fail validation"
        );
    }

    #[test]
    fn test_validate_resource_path_normalizes() {
        // Test that paths are normalized (leading/trailing whitespace removed)
        let paths = vec![
            (" /api ", "/api"),
            ("/api ", "/api"),
            (" /api", "/api"),
            ("api", "/api"), // Should add leading /
        ];

        for (input, expected) in paths {
            let result = validate_resource_path(input);
            assert!(result.is_ok(), "Path '{input}' should be valid");
            let sanitized = result.unwrap();
            assert_eq!(
                sanitized, expected,
                "Path '{input}' should be normalized to '{expected}', got '{sanitized}'"
            );
        }
    }

    #[test]
    fn test_validate_resource_path_control_characters() {
        // Test that control characters (except newline, tab, carriage return) are rejected
        let paths_with_control = vec![
            "/api\x01/resource", // SOH
            "/api\x02/resource", // STX
            "/api\x1F/resource", // Unit separator
        ];

        for path in paths_with_control {
            let result = validate_resource_path(path);
            assert!(
                result.is_err(),
                "Path with control character '{path}' should fail validation"
            );
        }
    }

    #[test]
    fn test_validate_resource_path_full_urls() {
        // Test that full URLs (http:// or https://) are preserved as-is without adding leading /
        let full_urls = vec![
            (
                "http://example.com/api/resource",
                "http://example.com/api/resource",
            ),
            (
                "https://example.com/api/resource",
                "https://example.com/api/resource",
            ),
            (
                "http://test.example.com/api/profiles/username/test.user",
                "http://test.example.com/api/profiles/username/test.user",
            ),
            (
                "https://api.example.com/v1/users?page=1",
                "https://api.example.com/v1/users?page=1",
            ),
            (" http://example.com/api ", "http://example.com/api"), // Should trim whitespace
            ("https://example.com/api ", "https://example.com/api"), // Should trim whitespace
        ];

        for (input, expected) in full_urls {
            let result = validate_resource_path(input);
            assert!(result.is_ok(), "Full URL '{input}' should be valid");
            let sanitized = result.unwrap();
            assert_eq!(
                sanitized, expected,
                "Full URL '{input}' should be preserved as '{expected}', got '{sanitized}'"
            );
            // Ensure no leading / was added before http:// or https://
            assert!(
                !sanitized.starts_with("/http"),
                "Full URL should not have leading / before http://, got '{sanitized}'"
            );
        }
    }

    #[test]
    fn test_validate_resource_path_relative_paths_still_get_slash() {
        // Test that relative paths (not full URLs) still get leading / added
        let relative_paths = vec![
            ("api", "/api"),
            ("api/resource", "/api/resource"),
            ("api/resource?query=value", "/api/resource?query=value"),
        ];

        for (input, expected) in relative_paths {
            let result = validate_resource_path(input);
            assert!(result.is_ok(), "Relative path '{input}' should be valid");
            let sanitized = result.unwrap();
            assert_eq!(
                sanitized, expected,
                "Relative path '{input}' should be normalized to '{expected}', got '{sanitized}'"
            );
            assert!(
                sanitized.starts_with('/'),
                "Relative path should start with /, got '{sanitized}'"
            );
        }
    }
}
