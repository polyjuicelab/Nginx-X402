//! Common utilities and helper functions for Docker integration tests
//!
//! This module provides shared functionality used across all Docker integration test modules.
//! It includes Docker container management, HTTP request helpers, and nginx readiness checks.
//!
//! # Usage
//!
//! All test modules should use these shared utilities to ensure consistent behavior
//! and avoid code duplication.

use std::process::Command;
use std::thread;
use std::time::Duration;

/// Docker image name for tests
pub const DOCKER_IMAGE: &str = "nginx-x402-test";

/// Docker container name for tests
pub const CONTAINER_NAME: &str = "nginx-x402-test-container";

/// Port on which nginx listens in the test container
pub const NGINX_PORT: u16 = 8080;

/// Build the Docker test image
///
/// # Returns
///
/// Returns `true` if the image was built successfully, `false` otherwise.
pub fn build_docker_image() -> bool {
    println!("Building Docker test image...");
    let output = Command::new("docker")
        .args([
            "build",
            "-t",
            DOCKER_IMAGE,
            "-f",
            "tests/Dockerfile.test",
            ".",
        ])
        .output();

    match output {
        Ok(output) if output.status.success() => {
            println!("Docker image built successfully");
            true
        }
        Ok(output) => {
            eprintln!(
                "Docker build failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            false
        }
        Err(e) => {
            eprintln!("Failed to run docker build: {e}");
            false
        }
    }
}

/// Start the Docker container
///
/// # Returns
///
/// Returns `true` if the container was started successfully and nginx is ready, `false` otherwise.
pub fn start_container() -> bool {
    println!("Starting Docker container...");
    let output = Command::new("docker")
        .args([
            "run",
            "-d",
            "--name",
            CONTAINER_NAME,
            "-p",
            &format!("{NGINX_PORT}:80"),
            DOCKER_IMAGE,
        ])
        .output();

    match output {
        Ok(output) if output.status.success() => {
            println!("Container started successfully");
            // Wait for nginx to be ready (up to 30 seconds)
            if wait_for_nginx(Duration::from_secs(30)) {
                println!("Nginx is ready");
                true
            } else {
                eprintln!("Nginx did not become ready within 30 seconds");
                false
            }
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("already in use") {
                // Container already exists, try to start it
                Command::new("docker")
                    .args(["start", CONTAINER_NAME])
                    .output()
                    .ok();
                // Wait for nginx to be ready (up to 30 seconds)
                if wait_for_nginx(Duration::from_secs(30)) {
                    println!("Nginx is ready");
                    true
                } else {
                    eprintln!("Nginx did not become ready within 30 seconds");
                    false
                }
            } else {
                eprintln!("Failed to start container: {stderr}");
                false
            }
        }
        Err(e) => {
            eprintln!("Failed to run docker: {e}");
            false
        }
    }
}

/// Stop and remove the Docker container
///
/// This function cleans up the test container. It's safe to call even if the container
/// doesn't exist or is already stopped.
pub fn cleanup_container() {
    let _ = Command::new("docker")
        .args(["stop", CONTAINER_NAME])
        .output();
    let _ = Command::new("docker").args(["rm", CONTAINER_NAME]).output();
}

/// Check if nginx is responding to health checks
///
/// # Returns
///
/// Returns `true` if nginx responds with HTTP 200 to the health endpoint, `false` otherwise.
pub fn nginx_is_ready() -> bool {
    Command::new("curl")
        .args([
            "-s",
            "-o",
            "/dev/null",
            "-w",
            "%{http_code}",
            &format!("http://localhost:{NGINX_PORT}/health"),
        ])
        .output()
        .map(|output| String::from_utf8_lossy(&output.stdout).trim() == "200")
        .unwrap_or(false)
}

/// Wait for nginx to be ready
///
/// This function polls the nginx health endpoint until it responds successfully
/// or the timeout is reached. It requires 3 consecutive successful health checks
/// to ensure nginx is fully ready.
///
/// # Arguments
///
/// * `max_wait` - Maximum duration to wait for nginx to become ready
///
/// # Returns
///
/// Returns `true` if nginx became ready within the timeout, `false` otherwise.
pub fn wait_for_nginx(max_wait: Duration) -> bool {
    let start = std::time::Instant::now();
    let mut consecutive_successes = 0;
    const REQUIRED_SUCCESSES: usize = 3; // Require 3 consecutive successful health checks

    while start.elapsed() < max_wait {
        if nginx_is_ready() {
            consecutive_successes += 1;
            if consecutive_successes >= REQUIRED_SUCCESSES {
                return true;
            }
        } else {
            consecutive_successes = 0; // Reset on failure
        }
        thread::sleep(Duration::from_millis(500));
    }
    false
}

/// Make HTTP request and return status code
///
/// # Arguments
///
/// * `path` - The path to request (e.g., "/api/protected")
///
/// # Returns
///
/// Returns `Some(status_code)` if the request succeeded, `None` otherwise.
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
///
/// # Arguments
///
/// * `path` - The path to request (e.g., "/api/protected")
///
/// # Returns
///
/// Returns `Some(body)` if the request succeeded, `None` otherwise.
pub fn http_get(path: &str) -> Option<String> {
    Command::new("curl")
        .args(["-s", &format!("http://localhost:{NGINX_PORT}{path}")])
        .output()
        .ok()
        .map(|output| String::from_utf8_lossy(&output.stdout).to_string())
}

/// Make HTTP request with custom headers and return response body
///
/// # Arguments
///
/// * `path` - The path to request
/// * `headers` - Array of (header_name, header_value) tuples
///
/// # Returns
///
/// Returns `Some(body)` if the request succeeded, `None` otherwise.
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
///
/// # Arguments
///
/// * `path` - The path to request
/// * `method` - HTTP method (e.g., "GET", "POST", "OPTIONS")
/// * `headers` - Array of (header_name, header_value) tuples
///
/// # Returns
///
/// Returns `Some(status_code)` if the request succeeded, `None` otherwise.
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

/// Ensure container is running, start it if needed
///
/// This function checks if the test container is running and ready. If not,
/// it attempts to start an existing container or build and start a new one.
///
/// # Returns
///
/// Returns `true` if the container is running and nginx is ready, `false` otherwise.
pub fn ensure_container_running() -> bool {
    // Check if container is already running
    if nginx_is_ready() {
        return true;
    }

    // Check if container exists but is stopped
    let check_output = Command::new("docker")
        .args([
            "ps",
            "-a",
            "--filter",
            &format!("name={CONTAINER_NAME}"),
            "--format",
            "{{.Status}}",
        ])
        .output();

    if let Ok(output) = check_output {
        let status = String::from_utf8_lossy(&output.stdout);
        if status.contains("Up") {
            // Container is running, wait for nginx (up to 30 seconds)
            return wait_for_nginx(Duration::from_secs(30));
        } else if !status.is_empty() {
            // Container exists but is stopped, start it
            let _ = Command::new("docker")
                .args(["start", CONTAINER_NAME])
                .output();
            // Wait for nginx to be ready (up to 30 seconds)
            return wait_for_nginx(Duration::from_secs(30));
        }
    }

    // Container doesn't exist, build and start it
    cleanup_container();
    build_docker_image() && start_container() && wait_for_nginx(Duration::from_secs(10))
}

