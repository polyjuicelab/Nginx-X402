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
//!
//! **Module Registration**: ⚠️ Framework Ready, Needs API Verification
//! - Module structure defined
//! - Configuration structure ready
//! - Handler wrapper function ready
//! - Core logic fully testable (see `tests/ngx_module_tests.rs` - 7 tests passing)
//! - **TODO**: Wire up with actual ngx-rust 0.5 API for full integration
//!
//! # Usage
//!
//! To use this module, enable the `ngx-rust` feature and build:
//!
//! ```bash
//! cargo build --release --features ngx-rust
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

use ngx::{
    http::{handler::http_request_handler, request::Request, status::Status},
    string::String as NgxString,
    Error, Result,
};
use rust_decimal::Decimal;
use rust_x402::{
    template::generate_paywall_html,
    types::{
        FacilitatorClient, FacilitatorConfig, PaymentPayload, PaymentRequirements,
        PaymentRequirementsResponse,
    },
};
use serde_json;
use std::str::FromStr;
use std::sync::OnceLock;

/// Module configuration (raw strings from Nginx config)
#[derive(Clone, Default)]
pub struct X402Config {
    pub enabled: ngx::flag_t,
    pub amount_str: ngx::string::String,
    pub pay_to_str: ngx::string::String,
    pub facilitator_url_str: ngx::string::String,
    pub testnet: ngx::flag_t,
    pub description_str: ngx::string::String,
    pub network_str: ngx::string::String,
    pub resource_str: ngx::string::String,
}

/// Parsed configuration
pub struct ParsedX402Config {
    pub enabled: bool,
    pub amount: Option<Decimal>,
    pub pay_to: Option<String>,
    pub facilitator_url: Option<String>,
    pub testnet: bool,
    pub description: Option<String>,
    pub network: Option<String>,
    pub resource: Option<String>,
}

impl X402Config {
    /// Parse raw config strings into typed values
    fn parse(&self) -> Result<ParsedX402Config> {
        let amount = if self.amount_str.is_empty() {
            None
        } else {
            self.amount_str
                .to_str()
                .and_then(|s| Decimal::from_str(s).ok())
                .ok()
        };

        let pay_to = if self.pay_to_str.is_empty() {
            None
        } else {
            self.pay_to_str.to_str().map(|s| s.to_string())
        };

        let facilitator_url = if self.facilitator_url_str.is_empty() {
            None
        } else {
            self.facilitator_url_str.to_str().map(|s| s.to_string())
        };

        let description = if self.description_str.is_empty() {
            None
        } else {
            self.description_str.to_str().map(|s| s.to_string())
        };

        let network = if self.network_str.is_empty() {
            None
        } else {
            self.network_str.to_str().map(|s| s.to_string())
        };

        let resource = if self.resource_str.is_empty() {
            None
        } else {
            self.resource_str.to_str().map(|s| s.to_string())
        };

        Ok(ParsedX402Config {
            enabled: self.enabled != 0,
            amount,
            pay_to,
            facilitator_url,
            testnet: self.testnet != 0,
            description,
            network,
            resource,
        })
    }
}

/// Global tokio runtime for async operations
static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();

fn get_runtime() -> &'static tokio::runtime::Runtime {
    RUNTIME.get_or_init(|| tokio::runtime::Runtime::new().expect("Failed to create tokio runtime"))
}

/// Get header value from request
fn get_header_value(r: &Request, name: &str) -> Option<String> {
    let headers = r.headers_in()?;
    let header_name = NgxString::from_str(name).ok()?;

    headers
        .get(&header_name)
        .and_then(|h| h.value().to_str().ok())
        .map(|s| s.to_string())
}

/// Check if request is from a browser
fn is_browser_request(r: &Request) -> bool {
    let user_agent = get_header_value(r, "User-Agent");
    let accept = get_header_value(r, "Accept");

    let has_browser_ua = user_agent
        .as_ref()
        .map(|ua| {
            ua.contains("Mozilla")
                || ua.contains("Chrome")
                || ua.contains("Safari")
                || ua.contains("Firefox")
                || ua.contains("Edge")
        })
        .unwrap_or(false);

    let accepts_html = accept
        .as_ref()
        .map(|a| a.contains("text/html") || a.contains("*/*"))
        .unwrap_or(false);

    has_browser_ua || accepts_html
}

/// Verify payment
async fn verify_payment(
    payment_b64: &str,
    requirements: &PaymentRequirements,
    facilitator_url: &str,
) -> Result<bool> {
    let payment_payload = PaymentPayload::from_base64(payment_b64)
        .map_err(|_| Error::from("Invalid payment payload"))?;

    let facilitator_config = FacilitatorConfig::new(facilitator_url);
    let facilitator = FacilitatorClient::new(facilitator_config)
        .map_err(|_| Error::from("Failed to create facilitator client"))?;

    let verify_result = facilitator
        .verify(&payment_payload, requirements)
        .await
        .map_err(|_| Error::from("Payment verification failed"))?;

    Ok(verify_result.valid)
}

/// Create payment requirements from config
fn create_requirements(config: &ParsedX402Config, resource: &str) -> Result<PaymentRequirements> {
    use rust_x402::types::networks;

    let amount = config
        .amount
        .ok_or_else(|| Error::from("Amount not configured"))?;
    let pay_to = config
        .pay_to
        .as_ref()
        .ok_or_else(|| Error::from("Pay-to address not configured"))?;

    // Determine network - use configured network or derive from testnet flag
    let network = if let Some(ref net) = config.network {
        net.as_str()
    } else if config.testnet {
        networks::BASE_SEPOLIA
    } else {
        networks::BASE_MAINNET
    };

    // Get USDC address for the network
    let usdc_address = networks::get_usdc_address(network)
        .ok_or_else(|| Error::from(format!("Network not supported: {}", network)))?;

    let resource = if let Some(ref resource_url) = config.resource {
        resource_url.clone()
    } else {
        resource.to_string()
    };

    // Convert amount to max_amount_required (in smallest unit, e.g., wei for USDC)
    let max_amount_required = (amount * rust_decimal::Decimal::from(1_000_000u64))
        .normalize()
        .to_string();

    let mut requirements = PaymentRequirements::new(
        rust_x402::types::schemes::EXACT,
        network,
        max_amount_required,
        usdc_address,
        pay_to.to_lowercase(),
        resource,
        config.description.as_deref().unwrap_or(""),
    );

    // Set network-specific USDC info
    let network_enum = if config.testnet {
        rust_x402::types::Network::Testnet
    } else {
        rust_x402::types::Network::Mainnet
    };
    requirements.set_usdc_info(network_enum)?;

    Ok(requirements)
}

/// Send 402 Payment Required response
fn send_402_response(
    r: &mut Request,
    requirements: &[PaymentRequirements],
    config: &ParsedX402Config,
    error_msg: Option<&str>,
) -> Result<()> {
    r.set_status(402)?;

    let is_browser = is_browser_request(r);

    if is_browser {
        // Send HTML paywall
        // Use error_msg if provided, otherwise use config description, otherwise use empty string
        let error_message = error_msg
            .or_else(|| config.description.as_deref())
            .unwrap_or("");
        let html = generate_paywall_html(error_message, requirements, None);

        r.headers_out_mut()
            .set_content_type("text/html; charset=utf-8")?;
        r.send_body(html.as_bytes())?;
    } else {
        // Send JSON response
        // Use error_msg if provided, otherwise use config description, otherwise use empty string
        let error_message = error_msg
            .or_else(|| config.description.as_deref())
            .unwrap_or("");
        let response = PaymentRequirementsResponse::new(error_message, requirements.to_vec());
        let json = serde_json::to_string(&response)
            .map_err(|_| Error::from("Failed to serialize response"))?;

        r.headers_out_mut()
            .set_content_type("application/json; charset=utf-8")?;
        r.send_body(json.as_bytes())?;
    }

    Ok(())
}

/// Request handler - core payment verification logic
///
/// This function contains the main payment verification logic for the ngx-rust module.
fn x402_handler_impl(r: &mut Request, config: &ParsedX402Config) -> Result<()> {
    if !config.enabled {
        return Ok(()); // Module disabled, pass through
    }

    let resource = config
        .resource
        .as_deref()
        .or_else(|| r.uri().to_str().ok())
        .unwrap_or("/");

    // Create payment requirements
    let requirements = create_requirements(config, resource)?;
    let requirements_vec = vec![requirements.clone()];

    // Check for X-PAYMENT header
    let payment_header = get_header_value(r, "X-PAYMENT");

    if let Some(payment_b64) = payment_header {
        // Verify payment
        let facilitator_url = config
            .facilitator_url
            .as_deref()
            .ok_or_else(|| Error::from("Facilitator URL not configured"))?;

        // Block on async verification
        let is_valid = get_runtime().block_on(async {
            verify_payment(&payment_b64, &requirements, facilitator_url).await
        })?;

        if is_valid {
            // Payment valid, allow request to proceed
            return Ok(());
        } else {
            // Payment invalid
            send_402_response(
                r,
                &requirements_vec,
                config,
                Some("Payment verification failed"),
            )?;
            return Ok(());
        }
    } else {
        // No payment header, send 402
        send_402_response(r, &requirements_vec, config, None)?;
        return Ok(());
    }
}

// ============================================================================
// ngx-rust Module Implementation
// ============================================================================

// ============================================================================
// NOTE: ngx-rust Module Registration
// ============================================================================
//
// The module registration below is a framework based on ngx-rust documentation.
// The exact API for ngx-rust 0.5 needs to be verified by:
//
// 1. Reviewing official examples: https://github.com/nginx/ngx-rust/tree/main/examples
// 2. Checking the actual API: https://docs.rs/ngx/0.5.0
// 3. Testing with a real Nginx build
//
// The core payment verification logic (x402_handler_impl) is complete and
// can be used once the module registration is properly wired up.
//
// The core logic is fully testable without requiring Nginx source code.
// See `tests/ngx_module_tests.rs` for test examples.
//
// ============================================================================

/// Request handler wrapper for ngx-rust
///
/// This function wraps the core payment verification logic and adapts it
/// to ngx-rust's request handler interface.
///
/// # Configuration Access
///
/// This implementation attempts to get module configuration from the request.
/// The exact API depends on ngx-rust 0.5's actual implementation.
#[cfg(feature = "ngx-rust")]
fn x402_ngx_handler(req: &mut Request) -> Result<()> {
    // Get module configuration from request
    // The actual API may be one of:
    // - req.get_loc_conf::<X402Module, X402Config>()
    // - req.loc_conf::<X402Config>()
    // - Or similar method

    let conf = get_module_config(req)?;
    let parsed_config = conf.parse()?;

    // Call the core handler
    x402_handler_impl(req, &parsed_config)
}

/// Get module configuration from request
///
/// This function retrieves the module's location configuration from the request.
/// The implementation uses ngx-rust's API to access the configuration.
///
/// # Implementation Notes
///
/// The ngx-rust `module!` macro generates a module structure that includes
/// a context index. The configuration can be accessed using:
/// - `req.get_loc_conf::<X402Module, X402Config>()` (if available)
/// - Or via the module's context index
///
/// The exact API depends on ngx-rust 0.5's implementation. This function
/// attempts to use the safest available method.
#[cfg(feature = "ngx-rust")]
fn get_module_config(req: &Request) -> Result<X402Config> {
    // The ngx-rust module! macro should provide a way to access configuration
    // The exact method depends on ngx-rust 0.5's API

    // Try to use ngx-rust's provided method (if available):
    // This is the preferred approach as it's type-safe
    if let Ok(conf) = req.get_loc_conf::<X402Module, X402Config>() {
        return Ok(conf.clone());
    }

    // Fallback: Use unsafe access via module context index
    // The module! macro should provide X402Module::ctx_index()
    unsafe {
        let r = req.as_ptr();
        if r.is_null() {
            return Err(Error::from("Invalid request pointer"));
        }

        // Get the module's context index (provided by ngx-rust module! macro)
        let ctx_index = X402Module::ctx_index();

        // Access loc_conf array at the module's context index
        let loc_conf = (*r).loc_conf as *mut *mut std::ffi::c_void;
        if loc_conf.is_null() {
            return Err(Error::from("Invalid loc_conf pointer"));
        }

        let conf_ptr = (*loc_conf.add(ctx_index)) as *mut X402Config;
        if conf_ptr.is_null() {
            return Err(Error::from("Invalid configuration pointer"));
        }

        Ok((*conf_ptr).clone())
    }
}

// ============================================================================
// Module Registration
// ============================================================================
//
// The module registration below provides a framework that can be adapted
// based on ngx-rust 0.5's actual API. The exact syntax may vary.
//
// Based on ngx-rust documentation and examples, there are typically two
// approaches:
//
// 1. Using the `http_request_handler!` macro for simple handlers
// 2. Using the `module!` macro for full module registration with commands
//
// Since we need configuration directives, we'll use approach 2.
//
// ============================================================================

// Module registration using ngx-rust's module! macro
//
// This macro generates the necessary boilerplate for Nginx module registration,
// including module structure, command definitions, and configuration management.
//
// The exact syntax may vary in ngx-rust 0.5, so this implementation provides
// a complete framework that can be adjusted once the API is verified.
//
// For testing purposes, the core logic (x402_handler_impl, create_requirements, etc.)
// is fully testable without requiring Nginx source code or the full module registration.

#[cfg(feature = "ngx-rust")]
ngx::http::module! {
    name: X402Module,
    commands: [
        // x402 on|off - Enable/disable x402 payment verification
        ngx::conf::Command {
            name: ngx::string!("x402"),
            ty: ngx::conf::CommandType::Flag,
            set: ngx::conf::set_flag_slot,
            conf: ngx::conf::CommandConf::LocConf(ngx::conf::offset_of!(X402Config, enabled)),
            post: None,
        },
        // x402_amount <amount> - Payment amount (e.g., "0.0001")
        ngx::conf::Command {
            name: ngx::string!("x402_amount"),
            ty: ngx::conf::CommandType::Take1,
            set: ngx::conf::set_str_slot,
            conf: ngx::conf::CommandConf::LocConf(ngx::conf::offset_of!(X402Config, amount_str)),
            post: None,
        },
        // x402_pay_to <address> - Recipient wallet address
        ngx::conf::Command {
            name: ngx::string!("x402_pay_to"),
            ty: ngx::conf::CommandType::Take1,
            set: ngx::conf::set_str_slot,
            conf: ngx::conf::CommandConf::LocConf(ngx::conf::offset_of!(X402Config, pay_to_str)),
            post: None,
        },
        // x402_facilitator_url <url> - Facilitator service URL
        ngx::conf::Command {
            name: ngx::string!("x402_facilitator_url"),
            ty: ngx::conf::CommandType::Take1,
            set: ngx::conf::set_str_slot,
            conf: ngx::conf::CommandConf::LocConf(ngx::conf::offset_of!(X402Config, facilitator_url_str)),
            post: None,
        },
        // x402_testnet on|off - Use testnet
        ngx::conf::Command {
            name: ngx::string!("x402_testnet"),
            ty: ngx::conf::CommandType::Flag,
            set: ngx::conf::set_flag_slot,
            conf: ngx::conf::CommandConf::LocConf(ngx::conf::offset_of!(X402Config, testnet)),
            post: None,
        },
        // x402_description <text> - Payment description
        ngx::conf::Command {
            name: ngx::string!("x402_description"),
            ty: ngx::conf::CommandType::Take1,
            set: ngx::conf::set_str_slot,
            conf: ngx::conf::CommandConf::LocConf(ngx::conf::offset_of!(X402Config, description_str)),
            post: None,
        },
        // x402_network <network> - Network identifier (e.g., "base-sepolia")
        ngx::conf::Command {
            name: ngx::string!("x402_network"),
            ty: ngx::conf::CommandType::Take1,
            set: ngx::conf::set_str_slot,
            conf: ngx::conf::CommandConf::LocConf(ngx::conf::offset_of!(X402Config, network_str)),
            post: None,
        },
        // x402_resource <path> - Resource path override
        ngx::conf::Command {
            name: ngx::string!("x402_resource"),
            ty: ngx::conf::CommandType::Take1,
            set: ngx::conf::set_str_slot,
            conf: ngx::conf::CommandConf::LocConf(ngx::conf::offset_of!(X402Config, resource_str)),
            post: None,
        },
    ],
    init: None,
    handler: Some(x402_ngx_handler),
}
