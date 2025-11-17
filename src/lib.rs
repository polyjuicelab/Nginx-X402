//! Nginx module for x402 HTTP micropayment protocol
//!
//! This module provides a pure Rust implementation using [ngx-rust](https://github.com/nginx/ngx-rust)
//! to implement the module entirely in Rust. No C wrapper is needed!
//!
//! # Architecture
//!
//! Uses the official `ngx-rust` crate to implement the module directly in Rust.
//! The module can be compiled as a dynamic library and loaded directly by Nginx.

pub mod config;

#[cfg(feature = "ngx-rust")]
pub mod ngx_module;

#[cfg(feature = "ngx-rust")]
pub use ngx_module::{ParsedX402Config, X402Config};

// Re-export commonly used types
pub use config::NginxX402Config;
