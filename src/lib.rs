//! Nginx module for x402 HTTP micropayment protocol
//!
//! This module provides FFI bindings and utilities for integrating x402 payment
//! verification into Nginx servers. It exposes C-compatible functions that can
//! be called from Nginx C modules.
//!
//! # Architecture
//!
//! The Nginx integration uses a hybrid approach:
//! - **FFI Module** (`ffi.rs`) - C-compatible functions for payment operations
//! - **Nginx C Module** - C module that calls FFI functions (to be implemented)
//!
//! # Usage
//!
//! ## From Nginx C Module
//!
//! ```c
//! #include "x402_ffi.h"
//!
//! // Verify payment
//! char result[4096];
//! size_t result_len = sizeof(result);
//! int status = x402_verify_payment(
//!     payment_b64, requirements_json, facilitator_url,
//!     result, &result_len
//! );
//!
//! if (status == 0) {
//!     // Payment is valid, proceed with request
//! } else if (status == 2) {
//!     // Payment verification failed, return 402
//! }
//! ```

pub mod config;
pub mod ffi;

// Re-export commonly used types
pub use config::NginxX402Config;
pub use ffi::*;

