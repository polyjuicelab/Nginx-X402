//! Pure Rust Nginx module implementation using ngx-rust
//!
//! This module implements the x402 payment verification directly in Rust,
//! using the official [ngx-rust](https://github.com/nginx/ngx-rust) bindings.
//!
//! # Status
//!
//! **Core Logic**: ✅ Complete
//! - Payment verification logic (`x402_handler_impl`)
//! - Configuration parsing (`X402Config::parse`)
//! - Payment requirements creation
//! - 402 response generation (HTML/JSON)
//! - Facilitator fallback handling (error/pass modes)
//! - Comprehensive test coverage (10 test suites)
//!
//! **Module Registration**: ⚠️ Framework Ready, Needs API Verification
//! - Module structure defined
//! - Configuration structure ready
//! - Handler wrapper function ready
//! - Core logic fully testable (see `tests/` directory)
//! - **TODO**: Wire up with actual ngx-rust 0.5 API for full integration
//!
//! # Usage
//!
//! To use this module, build with the `vendored` feature (recommended for CI/CD):
//!
//! ```bash
//! # With vendored Nginx source (auto-download, enabled by default)
//! cargo build --release
//!
//! # Or with Nginx source (for production matching)
//! export NGINX_SOURCE_DIR=/path/to/nginx
//! cargo build --release
//! ```
//!
//! # Next Steps
//!
//! To complete the ngx-rust implementation:
//!
//! 1. Review [ngx-rust examples](https://github.com/nginx/ngx-rust/tree/main/examples)
//! 2. Verify the actual API in [ngx-rust 0.5 docs](https://docs.rs/ngx/0.5.0)
//! 3. Update `x402_ngx_handler` to properly get module configuration
//! 4. Implement module registration using the correct macro/API
//! 5. Test with a real Nginx build

pub mod config;
pub mod error;
pub mod handler;
pub mod logging;
pub mod metrics;
pub mod module;
pub mod request;
pub mod requirements;
pub mod response;
pub mod runtime;

// Re-export public types and functions
pub use config::{FacilitatorFallback, ParsedX402Config, X402Config};
pub use error::{user_errors, ConfigError, Result};
pub use handler::{x402_handler_impl, x402_metrics_handler_impl, x402_ngx_handler_impl};
pub use logging::{log_debug, log_error, log_info, log_warn};
pub use metrics::{collect_metrics, X402Metrics};
pub use module::{get_module_config, ngx_http_x402_module};
pub use request::{get_header_value, is_browser_request};
pub use requirements::create_requirements;
pub use response::{send_402_response, send_response_body};
pub use runtime::{
    get_facilitator_client, get_runtime, verify_payment, DEFAULT_FACILITATOR_TIMEOUT,
    FACILITATOR_CLIENTS, MAX_PAYMENT_HEADER_SIZE, RUNTIME,
};

// Export the handlers using ngx-rust's macro
ngx::http_request_handler!(x402_ngx_handler, x402_ngx_handler_impl);
ngx::http_request_handler!(x402_metrics_handler, x402_metrics_handler_impl);
