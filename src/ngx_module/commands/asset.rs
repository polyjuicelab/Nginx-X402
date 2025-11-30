//! Asset configuration command handlers
//!
//! This module contains handlers for asset-related configuration directives:
//! - `x402_asset`
//! - `x402_asset_decimals`
//! - `x402_resource`

use crate::ngx_module::commands::common::copy_string_to_pool;
use crate::ngx_module::config::X402Config;
use ngx::ffi::{ngx_command_t, ngx_conf_t, ngx_str_t};
use std::ffi::c_char;

/// Parse `x402_resource` directive
pub(crate) unsafe extern "C" fn ngx_http_x402_resource(
    cf: *mut ngx_conf_t,
    _cmd: *mut ngx_command_t,
    conf: *mut core::ffi::c_void,
) -> *mut c_char {
    let conf = conf.cast::<X402Config>();
    if conf.is_null() {
        return ngx::core::NGX_CONF_ERROR.cast::<c_char>();
    }

    let args = (*cf).args;
    if args.is_null() || (*args).nelts < 2 {
        return ngx::core::NGX_CONF_ERROR.cast::<c_char>();
    }

    // elts is a pointer to an array of ngx_str_t, not an array of pointers
    let elts = (*args).elts.cast::<ngx_str_t>();
    let value_str = *elts.add(1);

    match copy_string_to_pool(cf, value_str) {
        Some(allocated_str) => {
            (*conf).resource_str = allocated_str;
        }
        None => {
            return ngx::core::NGX_CONF_ERROR.cast::<c_char>();
        }
    }

    std::ptr::null_mut()
}

/// Parse `x402_asset` directive
///
/// Allows specifying a custom token/contract address instead of using the default USDC address.
/// This enables support for custom tokens or native ETH (using zero address).
///
/// # Example
/// ```nginx
/// x402_asset 0xYourCustomTokenAddress;
/// ```
pub(crate) unsafe extern "C" fn ngx_http_x402_asset(
    cf: *mut ngx_conf_t,
    _cmd: *mut ngx_command_t,
    conf: *mut core::ffi::c_void,
) -> *mut c_char {
    let conf = conf.cast::<X402Config>();
    if conf.is_null() {
        return ngx::core::NGX_CONF_ERROR.cast::<c_char>();
    }

    let args = (*cf).args;
    if args.is_null() || (*args).nelts < 2 {
        return ngx::core::NGX_CONF_ERROR.cast::<c_char>();
    }

    // elts is a pointer to an array of ngx_str_t, not an array of pointers
    let elts = (*args).elts.cast::<ngx_str_t>();
    let value_str = *elts.add(1);

    match copy_string_to_pool(cf, value_str) {
        Some(allocated_str) => {
            (*conf).asset_str = allocated_str;
        }
        None => {
            return ngx::core::NGX_CONF_ERROR.cast::<c_char>();
        }
    }

    std::ptr::null_mut()
}

/// Parse `x402_asset_decimals` directive
///
/// Specifies the number of decimal places for the token (default: 6 for USDC).
/// Most ERC-20 tokens use 18 decimals. This is required when using custom tokens
/// to ensure correct amount calculation.
///
/// # Example
/// ```nginx
/// x402_asset_decimals 18;  # For standard ERC-20 tokens
/// x402_asset_decimals 6;   # For USDC
/// ```
pub(crate) unsafe extern "C" fn ngx_http_x402_asset_decimals(
    cf: *mut ngx_conf_t,
    _cmd: *mut ngx_command_t,
    conf: *mut core::ffi::c_void,
) -> *mut c_char {
    let conf = conf.cast::<X402Config>();
    if conf.is_null() {
        return ngx::core::NGX_CONF_ERROR.cast::<c_char>();
    }

    let args = (*cf).args;
    if args.is_null() || (*args).nelts < 2 {
        return ngx::core::NGX_CONF_ERROR.cast::<c_char>();
    }

    // elts is a pointer to an array of ngx_str_t, not an array of pointers
    let elts = (*args).elts.cast::<ngx_str_t>();
    let value_str = *elts.add(1);

    match copy_string_to_pool(cf, value_str) {
        Some(allocated_str) => {
            (*conf).asset_decimals_str = allocated_str;
        }
        None => {
            return ngx::core::NGX_CONF_ERROR.cast::<c_char>();
        }
    }

    std::ptr::null_mut()
}

