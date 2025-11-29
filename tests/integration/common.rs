//! Common utilities for Docker-based integration tests
//!
//! This module contains shared helper functions for managing Docker containers
//! and making HTTP requests in integration tests.

use std::process::Command;
use std::thread;
use std::time::Duration;

// Re-export HTTP helper functions for convenience
pub use super::http_helpers::*;

pub const DOCKER_IMAGE: &str = "nginx-x402-test";
pub const CONTAINER_NAME: &str = "nginx-x402-test-container";
pub const NGINX_PORT: u16 = 8080;

/// Build the Docker test image
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
pub fn cleanup_container() {
    let _ = Command::new("docker")
        .args(["stop", CONTAINER_NAME])
        .output();
    let _ = Command::new("docker").args(["rm", CONTAINER_NAME]).output();
}

/// Check if nginx is responding
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

/// Ensure container is running, start it if needed
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
