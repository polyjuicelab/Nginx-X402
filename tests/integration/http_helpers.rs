//! HTTP request helper functions for integration tests
//!
//! This module contains helper functions for making HTTP requests in integration tests.

use super::common::NGINX_PORT;
use std::process::Command;

/// Make HTTP request and return status code
pub fn http_request(path: &str) -> Option<String> {
    Command::new("curl")
        .args([
            "-s",
            "-o",
            "/dev/null",
            "-w",
            "%{http_code}",
            &format!("http://localhost:{NGINX_PORT}{path}"),
        ])
        .output()
        .ok()
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Get HTTP response body
pub fn http_get(path: &str) -> Option<String> {
    Command::new("curl")
        .args(["-s", &format!("http://localhost:{NGINX_PORT}{path}")])
        .output()
        .ok()
        .map(|output| String::from_utf8_lossy(&output.stdout).to_string())
}

/// Make HTTP request with custom headers and return response body
pub fn http_request_with_headers(path: &str, headers: &[(&str, &str)]) -> Option<String> {
    let url = format!("http://localhost:{NGINX_PORT}{path}");
    let header_strings: Vec<String> = headers
        .iter()
        .map(|(name, value)| format!("{}: {}", name, value))
        .collect();
    let mut args = vec!["-s", &url];
    for header in &header_strings {
        args.push("-H");
        args.push(header);
    }
    Command::new("curl")
        .args(args)
        .output()
        .ok()
        .map(|output| String::from_utf8_lossy(&output.stdout).to_string())
}

/// Make HTTP request with custom method and headers, return status code
pub fn http_request_with_method(
    path: &str,
    method: &str,
    headers: &[(&str, &str)],
) -> Option<String> {
    let url = format!("http://localhost:{NGINX_PORT}{path}");
    let header_strings: Vec<String> = headers
        .iter()
        .map(|(name, value)| format!("{}: {}", name, value))
        .collect();

    let mut args = vec![
        "-s",
        "-o",
        "/dev/null",
        "-w",
        "%{http_code}",
        "-X",
        method,
        &url,
    ];

    for header in &header_strings {
        args.push("-H");
        args.push(header);
    }

    Command::new("curl")
        .args(args)
        .output()
        .ok()
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Make HTTP request with custom method and headers, return status code and headers
pub fn http_request_with_method_and_headers(
    path: &str,
    method: &str,
    headers: &[(&str, &str)],
) -> Option<(String, String)> {
    let url = format!("http://localhost:{NGINX_PORT}{path}");
    let header_strings: Vec<String> = headers
        .iter()
        .map(|(name, value)| format!("{}: {}", name, value))
        .collect();

    let mut args = vec!["-s", "-i", "-X", method, &url];

    for header in &header_strings {
        args.push("-H");
        args.push(header);
    }

    Command::new("curl").args(args).output().ok().map(|output| {
        let response = String::from_utf8_lossy(&output.stdout).to_string();
        // Extract status code from response headers
        let status = response
            .lines()
            .next()
            .and_then(|line| line.split_whitespace().nth(1).map(|s| s.to_string()))
            .unwrap_or_else(|| "000".to_string());
        (status, response)
    })
}

/// Make HTTP request with custom method, headers, and body, return full response
pub fn http_request_with_headers_and_body(
    path: &str,
    method: &str,
    headers: &[(&str, &str)],
    body: &str,
) -> Option<(String, String)> {
    let url = format!("http://localhost:{NGINX_PORT}{path}");
    let header_strings: Vec<String> = headers
        .iter()
        .map(|(name, value)| format!("{}: {}", name, value))
        .collect();

    let mut args = vec!["-s", "-i", "-X", method, &url];

    for header in &header_strings {
        args.push("-H");
        args.push(header);
    }

    if !body.is_empty() {
        args.push("--data-binary");
        args.push(body);
    }

    Command::new("curl").args(args).output().ok().map(|output| {
        let response = String::from_utf8_lossy(&output.stdout).to_string();
        // Extract status code from response headers
        let status = response
            .lines()
            .next()
            .and_then(|line| line.split_whitespace().nth(1).map(|s| s.to_string()))
            .unwrap_or_else(|| "000".to_string());
        (status, response)
    })
}

/// Extract specific headers from curl output
pub fn get_response_headers(response: &str, header_names: &[&str]) -> Vec<(String, String)> {
    let mut headers = Vec::new();

    for line in response.lines() {
        for header_name in header_names {
            if line
                .to_lowercase()
                .starts_with(&format!("{}:", header_name.to_lowercase()))
            {
                if let Some(colon_pos) = line.find(':') {
                    let name = line[..colon_pos].trim().to_string();
                    let value = line[colon_pos + 1..].trim().to_string();
                    headers.push((name, value));
                }
            }
        }
    }

    headers
}
