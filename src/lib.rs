#![doc = include_str!("../README.md")]

pub mod config;
pub mod ngx_module;

// Re-export validation functions for testing
pub use config::{
    parse_accept_priority, validate_amount, validate_ethereum_address, validate_network,
    validate_payment_header, validate_resource_path, validate_url,
};

// Re-export ngx_module types and functions
pub use ngx_module::{ParsedX402Config, Result, X402Config};

// Re-export commonly used types
pub use config::NginxX402Config;
