//! Tests for resource path validation
//!
//! These tests verify that resource path validation works correctly,
//! covering the fix for:
//! - fix-10: Resource path security (preventing path traversal attacks)

mod tests {
    use nginx_x402::validation::validate_resource_path;

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
            assert!(
                result.is_ok(),
                "Valid path '{}' should pass validation",
                path
            );
            let sanitized = result.unwrap();
            // Paths should start with /
            assert!(
                sanitized.starts_with('/'),
                "Sanitized path should start with /, got: {}",
                sanitized
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
                "Dangerous path '{}' should fail validation",
                path
            );
            let error = result.unwrap_err().to_string();
            assert!(
                error.contains("Configuration error") || error.contains("error"),
                "Error should indicate configuration error, got: {}",
                error
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
                "Path with null byte '{}' should fail validation",
                path
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
                "Empty path '{}' should fail validation",
                path
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
            assert!(result.is_ok(), "Path '{}' should be valid", input);
            let sanitized = result.unwrap();
            assert_eq!(
                sanitized, expected,
                "Path '{}' should be normalized to '{}', got '{}'",
                input, expected, sanitized
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
                "Path with control character '{}' should fail validation",
                path
            );
        }
    }
}
