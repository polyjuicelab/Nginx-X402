//! Request handling utilities

use ngx::http::Request;

/// Get header value from request
///
/// # Arguments
/// - `r`: Nginx request object
/// - `name`: Header name (case-insensitive)
///
/// # Returns
/// - `Some(String)` if header exists and can be read
/// - `None` if header doesn't exist or cannot be read
#[must_use] 
pub fn get_header_value(r: &Request, name: &str) -> Option<String> {
    if name.trim().is_empty() {
        return None;
    }

    // Iterate through headers_in to find the header
    for (key, value) in r.headers_in_iterator() {
        if let Ok(key_str) = key.to_str() {
            if key_str.eq_ignore_ascii_case(name) {
                return value.to_str().ok().map(std::string::ToString::to_string);
            }
        }
    }
    None
}

/// Check if request is from a browser
///
/// Uses a strict, priority-based detection algorithm:
/// 1. **Accept header priority** (highest priority): Parse Accept header with q-values
///    - If `text/html` has q > 0.5, likely browser
///    - If `application/json` has q > 0.5 and no `text/html`, likely API
///    - If `*/*` is present with high q-value, check other indicators
/// 2. **User-Agent header**: Check for browser identifiers
///    - Must contain known browser strings (case-insensitive)
///    - Exclude common API clients (curl, wget, python-requests, etc.)
/// 3. **Content-Type header**: Check for browser-specific content types
///    - `multipart/form-data` (browser form submissions)
///    - `application/x-www-form-urlencoded` (browser forms)
/// 4. **Upgrade header**: Check for protocol upgrades (WebSocket, etc.)
///
/// # Arguments
/// - `r`: Nginx request object
///
/// # Returns
/// - `true` if request appears to be from a browser
/// - `false` if request appears to be from an API client
#[must_use] 
pub fn is_browser_request(r: &Request) -> bool {
    let user_agent = get_header_value(r, "User-Agent");
    let accept = get_header_value(r, "Accept");
    let content_type = get_header_value(r, "Content-Type");
    let upgrade = get_header_value(r, "Upgrade");

    // Priority 1: Check Accept header with q-value parsing
    if let Some(ref accept_header) = accept {
        let html_priority = crate::config::parse_accept_priority(accept_header, "text/html");
        let json_priority = crate::config::parse_accept_priority(accept_header, "application/json");
        let wildcard_priority = crate::config::parse_accept_priority(accept_header, "*/*");

        // If HTML has high priority (>0.5), likely browser
        if html_priority > 0.5 {
            return true;
        }

        // If JSON has high priority (>0.5) and HTML is low or absent, likely API
        if json_priority > 0.5 && html_priority < 0.3 {
            return false;
        }

        // If wildcard with high priority, check other indicators
        if wildcard_priority > 0.8 {
            // Continue to other checks
        } else if wildcard_priority > 0.5 {
            // Medium priority wildcard, prefer other indicators
        }
    }

    // Priority 2: Check User-Agent for browser identifiers
    // Use stricter matching: must contain browser identifier AND not be an API client
    let has_browser_ua = user_agent
        .as_ref()
        .is_some_and(|ua| {
            let ua_lower = ua.to_lowercase();

            // Check for browser identifiers
            let has_browser = ua_lower.contains("mozilla")
                && (ua_lower.contains("chrome")
                    || ua_lower.contains("safari")
                    || ua_lower.contains("firefox")
                    || ua_lower.contains("edge")
                    || ua_lower.contains("opera")
                    || ua_lower.contains("brave")
                    || ua_lower.contains("webkit"));

            // Exclude common API clients
            let is_api_client = ua_lower.contains("curl")
                || ua_lower.contains("wget")
                || ua_lower.contains("python-requests")
                || ua_lower.contains("go-http-client")
                || ua_lower.contains("java/")
                || ua_lower.contains("okhttp")
                || ua_lower.contains("httpie")
                || ua_lower.contains("postman")
                || ua_lower.contains("insomnia")
                || ua_lower.starts_with("rest-client")
                || ua_lower.starts_with("http");

            has_browser && !is_api_client
        });

    // Priority 3: Check Content-Type for browser-specific types
    let is_browser_content_type = content_type
        .as_ref()
        .is_some_and(|ct| {
            let ct_lower = ct.to_lowercase();
            ct_lower.starts_with("multipart/form-data")
                || ct_lower.starts_with("application/x-www-form-urlencoded")
        });

    // Priority 4: Check Upgrade header (WebSocket, etc.)
    let has_upgrade = upgrade.is_some();

    // Combine indicators with priority weighting
    // Browser UA is strong indicator, but not sufficient alone
    // Content-Type and Upgrade are strong indicators
    // Accept header already handled above
    is_browser_content_type
        || (has_browser_ua
            && (has_upgrade
                || accept.is_none()
                || crate::config::parse_accept_priority(
                    accept.as_deref().unwrap_or(""),
                    "text/html",
                ) > 0.0))
}

/// Check if request is a WebSocket upgrade request
///
/// WebSocket requests use the HTTP Upgrade mechanism and should typically
/// skip payment verification since they are long-lived connections.
///
/// # Arguments
/// - `r`: Nginx request object
///
/// # Returns
/// - `true` if request is a WebSocket upgrade request
/// - `false` otherwise
#[must_use] 
pub fn is_websocket_request(r: &Request) -> bool {
    let upgrade = get_header_value(r, "Upgrade");
    let connection = get_header_value(r, "Connection");

    // Check for WebSocket upgrade headers
    let has_upgrade = upgrade
        .as_ref()
        .is_some_and(|u| u.to_lowercase() == "websocket");

    let has_connection_upgrade = connection
        .as_ref()
        .is_some_and(|c| c.to_lowercase().contains("upgrade"));

    has_upgrade && has_connection_upgrade
}
