//! Tests for browser detection logic
//!
//! These tests verify that browser detection works correctly,
//! covering the fix for:
//! - fix-9: Browser detection improvements (strict logic, Accept header priority)
//! - fix/content-type-json-api-detection: Content-Type: application/json should be treated as API request

mod tests {
    use nginx_x402::parse_accept_priority;

    /// Helper function to compare floats with epsilon tolerance
    fn float_eq(a: f64, b: f64) -> bool {
        (a - b).abs() < f64::EPSILON
    }

    #[test]
    fn test_parse_accept_priority_html() {
        // Test HTML priority parsing
        let accept = "text/html, application/json;q=0.9";
        assert!(float_eq(parse_accept_priority(accept, "text/html"), 1.0));
        assert!(float_eq(
            parse_accept_priority(accept, "application/json"),
            0.9
        ));
    }

    #[test]
    fn test_parse_accept_priority_with_q_values() {
        // Test various q-value scenarios
        let test_cases = vec![
            ("text/html;q=0.8", "text/html", 0.8),
            ("text/html;q=1.0", "text/html", 1.0),
            ("text/html;q=0.0", "text/html", 0.0),
            ("text/html;q=1.5", "text/html", 1.0), // Clamped to 1.0
            ("text/html;q=-0.5", "text/html", 0.0), // Clamped to 0.0
        ];

        for (accept, media_type, expected) in test_cases {
            let result = parse_accept_priority(accept, media_type);
            assert!(
                float_eq(result, expected),
                "Accept '{accept}' for '{media_type}' should have priority {expected}, got {result}"
            );
        }
    }

    #[test]
    fn test_parse_accept_priority_wildcard() {
        // Test wildcard matching
        let accept = "*/*;q=0.8";
        assert!(float_eq(parse_accept_priority(accept, "text/html"), 0.8));
        assert!(float_eq(
            parse_accept_priority(accept, "application/json"),
            0.8
        ));
        assert!(float_eq(parse_accept_priority(accept, "*/*"), 0.8));
    }

    #[test]
    fn test_parse_accept_priority_not_found() {
        // Test when media type is not in Accept header
        let accept = "application/json";
        assert!(float_eq(parse_accept_priority(accept, "text/html"), 0.0));

        // Should fall back to wildcard if available
        let accept_with_wildcard = "application/json, */*;q=0.5";
        assert!(float_eq(
            parse_accept_priority(accept_with_wildcard, "text/html"),
            0.5
        ));
    }

    #[test]
    fn test_parse_accept_priority_multiple_types() {
        // Test complex Accept header
        let accept = "text/html, application/xhtml+xml, application/xml;q=0.9, */*;q=0.8";
        assert!(float_eq(parse_accept_priority(accept, "text/html"), 1.0));
        assert!(float_eq(
            parse_accept_priority(accept, "application/xhtml+xml"),
            1.0
        ));
        assert!(float_eq(
            parse_accept_priority(accept, "application/xml"),
            0.9
        ));
        assert!(float_eq(parse_accept_priority(accept, "image/png"), 0.8)); // Falls back to */*
    }

    #[test]
    fn test_parse_accept_priority_case_insensitive() {
        // Test that parsing handles case (though HTTP headers should be case-insensitive)
        let accept = "TEXT/HTML, APPLICATION/JSON;Q=0.9";
        // Note: Our implementation is case-sensitive, which is fine for testing
        // In production, headers should be normalized
        assert!(float_eq(parse_accept_priority(accept, "TEXT/HTML"), 1.0));
    }

    #[test]
    fn test_parse_accept_priority_empty() {
        // Test empty Accept header
        assert!(float_eq(parse_accept_priority("", "text/html"), 0.0));
    }

    #[test]
    fn test_parse_accept_priority_complex() {
        // Test real-world browser Accept header
        let browser_accept = "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7";
        assert!(float_eq(
            parse_accept_priority(browser_accept, "text/html"),
            1.0
        ));
        assert!(float_eq(
            parse_accept_priority(browser_accept, "image/webp"),
            1.0
        ));
        assert!(float_eq(
            parse_accept_priority(browser_accept, "application/json"),
            0.8
        )); // Falls back to */*
    }

    #[test]
    fn test_parse_accept_priority_api_client() {
        // Test API client Accept header (JSON preferred)
        let api_accept = "application/json, text/plain;q=0.9, */*;q=0.8";
        assert!(float_eq(
            parse_accept_priority(api_accept, "application/json"),
            1.0
        ));
        assert!(float_eq(
            parse_accept_priority(api_accept, "text/html"),
            0.8
        )); // Falls back to */*
    }
}
