#![doc = include_str!("../README.md")]

pub mod config;
pub mod ngx_module;

// Re-export validation functions for testing
pub use config::validation;

// Re-export ngx_module types and functions
pub use config::validation::validate_payment_header;
pub use ngx_module::{ParsedX402Config, Result, X402Config};

// Re-export commonly used types
pub use config::NginxX402Config;
