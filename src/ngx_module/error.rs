//! Error types and user-facing error messages for the Nginx module

use core::fmt;

/// Configuration parsing error
#[derive(Debug)]
pub struct ConfigError(String);

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for ConfigError {
    fn from(s: &str) -> Self {
        ConfigError(s.to_string())
    }
}

impl From<String> for ConfigError {
    fn from(s: String) -> Self {
        ConfigError(s)
    }
}

impl From<rust_x402::X402Error> for ConfigError {
    fn from(e: rust_x402::X402Error) -> Self {
        ConfigError(format!("{e}"))
    }
}

/// Result type alias for module operations
pub type Result<T> = core::result::Result<T, ConfigError>;

/// User-facing error messages (safe to expose to clients)
pub mod user_errors {
    pub const PAYMENT_VERIFICATION_FAILED: &str = "Payment verification failed";
    pub const INVALID_PAYMENT: &str = "Invalid payment";
    pub const CONFIGURATION_ERROR: &str = "Configuration error";
    pub const TIMEOUT: &str = "Request timeout";
}
