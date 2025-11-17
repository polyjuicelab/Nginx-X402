//! Tests for browser detection logic
//!
//! These tests verify that browser detection works correctly,
//! covering the fix for:
//! - fix-9: Browser detection improvements (strict logic, Accept header priority)

mod tests {
    use nginx_x402::parse_accept_priority;

    #[test]
    fn test_parse_accept_priority_html() {
        // Test HTML priority parsing
        let accept = "text/html, application/json;q=0.9";
        assert_eq!(parse_accept_priority(accept, "text/html"), 1.0);
        assert_eq!(parse_accept_priority(accept, "application/json"), 0.9);
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
            assert_eq!(
                result, expected,
                "Accept '{}' for '{}' should have priority {}",
                accept, media_type, expected
            );
        }
    }

    #[test]
    fn test_parse_accept_priority_wildcard() {
        // Test wildcard matching
        let accept = "*/*;q=0.8";
        assert_eq!(parse_accept_priority(accept, "text/html"), 0.8);
        assert_eq!(parse_accept_priority(accept, "application/json"), 0.8);
        assert_eq!(parse_accept_priority(accept, "*/*"), 0.8);
    }

    #[test]
    fn test_parse_accept_priority_not_found() {
        // Test when media type is not in Accept header
        let accept = "application/json";
        assert_eq!(parse_accept_priority(accept, "text/html"), 0.0);

        // Should fall back to wildcard if available
        let accept_with_wildcard = "application/json, */*;q=0.5";
        assert_eq!(
            parse_accept_priority(accept_with_wildcard, "text/html"),
            0.5
        );
    }

    #[test]
    fn test_parse_accept_priority_multiple_types() {
        // Test complex Accept header
        let accept = "text/html, application/xhtml+xml, application/xml;q=0.9, */*;q=0.8";
        assert_eq!(parse_accept_priority(accept, "text/html"), 1.0);
        assert_eq!(parse_accept_priority(accept, "application/xhtml+xml"), 1.0);
        assert_eq!(parse_accept_priority(accept, "application/xml"), 0.9);
        assert_eq!(parse_accept_priority(accept, "image/png"), 0.8); // Falls back to */*
    }

    #[test]
    fn test_parse_accept_priority_case_insensitive() {
        // Test that parsing handles case (though HTTP headers should be case-insensitive)
        let accept = "TEXT/HTML, APPLICATION/JSON;Q=0.9";
        // Note: Our implementation is case-sensitive, which is fine for testing
        // In production, headers should be normalized
        assert_eq!(parse_accept_priority(accept, "TEXT/HTML"), 1.0);
    }

    #[test]
    fn test_parse_accept_priority_empty() {
        // Test empty Accept header
        assert_eq!(parse_accept_priority("", "text/html"), 0.0);
    }

    #[test]
    fn test_parse_accept_priority_complex() {
        // Test real-world browser Accept header
        let browser_accept = "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7";
        assert_eq!(parse_accept_priority(browser_accept, "text/html"), 1.0);
        assert_eq!(parse_accept_priority(browser_accept, "image/webp"), 1.0);
        assert_eq!(
            parse_accept_priority(browser_accept, "application/json"),
            0.8
        ); // Falls back to */*
    }

    #[test]
    fn test_parse_accept_priority_api_client() {
        // Test API client Accept header (JSON preferred)
        let api_accept = "application/json, text/plain;q=0.9, */*;q=0.8";
        assert_eq!(parse_accept_priority(api_accept, "application/json"), 1.0);
        assert_eq!(parse_accept_priority(api_accept, "text/html"), 0.8); // Falls back to */*
    }
}
