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
/// 0. **Content-Type header** (highest priority): If `application/json`, definitely API
/// 1. **Accept header priority**: Parse Accept header with q-values
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

    // Priority 0: Check Content-Type header first (strongest indicator)
    // If Content-Type is application/json, this is definitely an API request
    if let Some(ref content_type_header) = content_type {
        let ct_lower = content_type_header.to_lowercase();
        if ct_lower.starts_with("application/json") {
            return false; // API request
        }
    }

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
    let has_browser_ua = user_agent.as_ref().is_some_and(|ua| {
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
    // Note: application/json is already handled above as API indicator
    let is_browser_content_type = content_type.as_ref().is_some_and(|ct| {
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

/// Get HTTP method from request
///
/// Returns the HTTP method as a string slice (e.g., "GET", "POST", "OPTIONS").
///
/// Uses the `method` field (integer ID) for reliable detection, falling back to
/// `method_name` (string) if the method ID is not recognized.
///
/// Uses ngx-rust's safe `Request` API instead of raw pointers.
///
/// # Arguments
/// - `r`: Nginx request object
///
/// # Returns
/// - `Some(&str)` with the HTTP method name (uppercase) if available
/// - `None` if method cannot be determined
#[must_use]
pub fn get_http_method(r: &Request) -> Option<&'static str> {
    // Use Request's as_ref() to access the underlying structure
    // Safe: Request is a zero-cost wrapper, as_ref() returns a valid reference
    let request_struct = r.as_ref();

    // Use method ID (integer) for reliable detection
    // Nginx method IDs:
    // NGX_HTTP_GET = 0x00000002
    // NGX_HTTP_HEAD = 0x00000004
    // NGX_HTTP_POST = 0x00000008
    // NGX_HTTP_PUT = 0x00000010
    // NGX_HTTP_DELETE = 0x00000020
    // NGX_HTTP_OPTIONS = 0x00000200
    // NGX_HTTP_PATCH = 0x00004000
    // NGX_HTTP_TRACE = 0x00008000
    // NGX_HTTP_CONNECT = 0x00010000
    let method_id = request_struct.method;

    match method_id {
        0x00000002 => Some("GET"),
        0x00000004 => Some("HEAD"),
        0x00000008 => Some("POST"),
        0x00000010 => Some("PUT"),
        0x00000020 => Some("DELETE"),
        0x00000200 => Some("OPTIONS"),
        0x00004000 => Some("PATCH"),
        0x00008000 => Some("TRACE"),
        0x00010000 => Some("CONNECT"),
        _ => {
            // Fallback to method_name if method ID is not recognized
            // This handles custom HTTP methods or edge cases
            let method_name = request_struct.method_name;
            if method_name.data.is_null() || method_name.len == 0 {
                return None;
            }

            // Safe: We've validated method_name.data is not null and len > 0
            let method_slice =
                unsafe { std::slice::from_raw_parts(method_name.data, method_name.len) };
            match method_slice {
                b"GET" | b"get" => Some("GET"),
                b"POST" | b"post" => Some("POST"),
                b"PUT" | b"put" => Some("PUT"),
                b"DELETE" | b"delete" => Some("DELETE"),
                b"PATCH" | b"patch" => Some("PATCH"),
                b"HEAD" | b"head" => Some("HEAD"),
                b"OPTIONS" | b"options" => Some("OPTIONS"),
                b"TRACE" | b"trace" => Some("TRACE"),
                b"CONNECT" | b"connect" => Some("CONNECT"),
                _ => None,
            }
        }
    }
}

/// Get HTTP method ID from request
///
/// Returns the HTTP method as an integer ID (nginx's internal representation).
///
/// Uses ngx-rust's safe `Request` API instead of raw pointers.
///
/// # Arguments
/// - `r`: Nginx request object
///
/// # Returns
/// - Method ID as usize (e.g., 0x00000002 for GET)
#[must_use]
pub fn get_http_method_id(r: &Request) -> usize {
    let request_struct = r.as_ref();
    request_struct.method
}

/// Build full URL from request
///
/// Constructs a complete URL from the request's scheme, host, and URI.
/// This is useful for x402 resource requirements that need a full URL instead of a relative path.
///
/// # Arguments
/// - `r`: Nginx request object
///
/// # Returns
/// - `Some(String)` with the full URL if all components are available
/// - `None` if any required component (scheme, host, or URI) is missing
#[must_use]
pub fn build_full_url(r: &Request) -> Option<String> {
    // Get scheme (http or https)
    // Check X-Forwarded-Proto header first (for reverse proxy scenarios)
    // Default to http if header is not present (can be overridden by proxy_set_header)
    let scheme = get_header_value(r, "X-Forwarded-Proto")
        .and_then(|proto| {
            let proto_lower = proto.to_lowercase();
            if proto_lower == "https" {
                Some("https")
            } else if proto_lower == "http" {
                Some("http")
            } else {
                None
            }
        })
        .unwrap_or("http");

    // Get host from Host header
    let host = get_header_value(r, "Host")?;

    // Get URI path (ensure it starts with /)
    let uri = r.path().to_str().ok()?;
    let uri_normalized = if uri.starts_with('/') {
        uri.to_string()
    } else {
        // If URI doesn't start with /, add it
        format!("/{}", uri)
    };

    // Build full URL: scheme://host/uri
    // Note: URI already starts with /, so no need to add another /
    Some(format!("{}://{}{}", scheme, host, uri_normalized))
}

/// Infer MIME type from request headers
///
/// Attempts to determine the MIME type from the request's Accept or Content-Type headers.
/// This is useful for x402 payment requirements that need a mimeType field.
///
/// Priority:
/// 1. Content-Type header (if present)
/// 2. Accept header (highest priority media type)
/// 3. Default to "application/json" if neither is available
///
/// # Arguments
/// - `r`: Nginx request object
///
/// # Returns
/// - MIME type string (e.g., "application/json", "text/html")
#[must_use]
pub fn infer_mime_type(r: &Request) -> String {
    // First, try Content-Type header
    if let Some(content_type) = get_header_value(r, "Content-Type") {
        // Extract MIME type from Content-Type (remove parameters like charset)
        let mime_type = content_type
            .split(';')
            .next()
            .unwrap_or("application/json")
            .trim()
            .to_string();
        if !mime_type.is_empty() {
            return mime_type;
        }
    }

    // Second, try Accept header
    if let Some(accept) = get_header_value(r, "Accept") {
        // Parse Accept header to find highest priority media type
        // Look for common MIME types in order of preference
        let accept_lower = accept.to_lowercase();

        // Check for specific types first
        if accept_lower.contains("application/json") {
            return "application/json".to_string();
        }
        if accept_lower.contains("text/html") {
            return "text/html".to_string();
        }
        if accept_lower.contains("application/xml") || accept_lower.contains("text/xml") {
            return "application/xml".to_string();
        }
        if accept_lower.contains("text/plain") {
            return "text/plain".to_string();
        }

        // Try to extract first media type from Accept header
        // Format: "type/subtype;q=value, type/subtype"
        if let Some(first_type) = accept.split(',').next() {
            let mime_type = first_type
                .split(';')
                .next()
                .unwrap_or("application/json")
                .trim()
                .to_string();
            if !mime_type.is_empty() && mime_type != "*/*" {
                return mime_type;
            }
        }
    }

    // Default fallback
    "application/json".to_string()
}

/// Check if HTTP method should skip payment verification
///
/// Some HTTP methods are used for protocol-level operations and should
/// not require payment verification:
/// - **OPTIONS**: CORS preflight requests sent by browsers before cross-origin requests
/// - **HEAD**: Used to check resource existence without retrieving body
/// - **TRACE**: Used for diagnostic and debugging purposes
///
/// These methods are typically used for infrastructure/checking purposes rather than
/// actual resource access, so payment verification should be skipped.
///
/// Uses ngx-rust's safe `Request` API instead of raw pointers.
///
/// # Arguments
/// - `r`: Nginx request object
///
/// # Returns
/// - `true` if the HTTP method should skip payment verification
/// - `false` otherwise
#[must_use]
pub fn should_skip_payment_for_method(r: &Request) -> bool {
    matches!(
        get_http_method(r),
        Some("OPTIONS") | Some("HEAD") | Some("TRACE")
    )
}

#[cfg(test)]
mod tests {
    // Note: Unit tests for `is_browser_request` require nginx Request objects
    // which are difficult to mock. The behavior is verified by integration tests:
    // - test_content_type_json_returns_json_response
    // - test_content_type_json_without_user_agent
    // - test_browser_request_without_content_type_returns_html
    //
    // Expected behavior:
    // 1. Content-Type: application/json -> API request (false)
    // 2. Content-Type: application/json + browser User-Agent -> API request (false)
    // 3. Browser User-Agent without Content-Type -> Browser request (true)
}
