//! Network configuration command handlers
//!
//! This module contains handlers for network-related configuration directives:
//! - `x402_network`
//! - `x402_network_id`

use crate::ngx_module::commands::common::copy_string_to_pool;
use crate::ngx_module::config::X402Config;
use ngx::ffi::{ngx_command_t, ngx_conf_t, ngx_str_t};
use std::ffi::c_char;
use std::ptr;

/// Parse `x402_network` directive
pub(crate) unsafe extern "C" fn ngx_http_x402_network(
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
            (*conf).network_str = allocated_str;
        }
        None => {
            return ngx::core::NGX_CONF_ERROR.cast::<c_char>();
        }
    }

    ptr::null_mut()
}

/// Parse `x402_network_id` directive
///
/// Allows specifying network by chainId instead of network name.
/// Takes precedence over `x402_network` if both are specified.
///
/// # Example
/// ```nginx
/// x402_network_id 8453;  # Base Mainnet
/// x402_network_id 84532;  # Base Sepolia
/// ```
pub(crate) unsafe extern "C" fn ngx_http_x402_network_id(
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
            (*conf).network_id_str = allocated_str;
        }
        None => {
            return ngx::core::NGX_CONF_ERROR.cast::<c_char>();
        }
    }

    ptr::null_mut()
}
