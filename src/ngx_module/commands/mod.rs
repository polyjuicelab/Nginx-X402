//! Configuration command definitions for x402 module
//!
//! This module contains the nginx configuration directive handlers. These functions
//! are called by nginx during configuration parsing to process directives like
//! `x402 on;`, `x402_amount`, `x402_pay_to`, etc.
//!
//! # Important Notes
//!
//! - Logging functions cannot be used here because there is no request context
//!   during configuration parsing. Logging requires a Request object, which is only
//!   available during request processing.
//! - Handler functions are set directly in the location configuration structure
//!   (`clcf->handler`) when directives are parsed in location context.
//!
//! # Module Structure
//!
//! This module is organized into submodules:
//!
//! - `common`: Shared utilities (string copying, etc.)
//! - `basic`: Basic configuration commands (x402, amount, pay_to, etc.)
//! - `network`: Network-related commands (network, network_id)
//! - `asset`: Asset-related commands (asset, asset_decimals, resource)
//! - `other`: Other commands (timeout, facilitator_fallback, metrics)

mod asset;
mod basic;
mod common;
mod network;
mod other;

use ngx::ffi::{ngx_command_t, ngx_str_t};
use ngx::ngx_string;
use std::ptr;

// Re-export command handlers
use asset::{ngx_http_x402_asset, ngx_http_x402_asset_decimals, ngx_http_x402_resource};
use basic::{
    ngx_http_x402, ngx_http_x402_amount, ngx_http_x402_description, ngx_http_x402_facilitator_url,
    ngx_http_x402_pay_to,
};
use network::{ngx_http_x402_network, ngx_http_x402_network_id};
use other::{
    ngx_http_x402_facilitator_fallback, ngx_http_x402_metrics, ngx_http_x402_timeout,
    ngx_http_x402_ttl,
};

/// Configuration commands array
///
/// This array defines all the configuration directives supported by the module.
/// Each command specifies:
/// - Name: the directive name
/// - Type: where it can be used (location, server, etc.)
/// - Set: offset to the config field and handler function
/// - Conf: configuration structure offset
/// - Offset: offset within the config structure
/// - Post: post-processing function (if any)
#[no_mangle]
pub static mut ngx_http_x402_commands: [ngx_command_t; 15] = [
    ngx_command_t {
        name: ngx_string!("x402"),
        type_: (ngx::ffi::NGX_HTTP_MAIN_CONF
            | ngx::ffi::NGX_HTTP_SRV_CONF
            | ngx::ffi::NGX_HTTP_LOC_CONF
            | ngx::ffi::NGX_CONF_TAKE1) as usize,
        set: Some(ngx_http_x402),
        conf: ngx::ffi::NGX_HTTP_LOC_CONF_OFFSET,
        offset: 0,
        post: ptr::null_mut(),
    },
    ngx_command_t {
        name: ngx_string!("x402_amount"),
        type_: (ngx::ffi::NGX_HTTP_LOC_CONF | ngx::ffi::NGX_CONF_TAKE1) as usize,
        set: Some(ngx_http_x402_amount),
        conf: ngx::ffi::NGX_HTTP_LOC_CONF_OFFSET,
        offset: 0,
        post: ptr::null_mut(),
    },
    ngx_command_t {
        name: ngx_string!("x402_pay_to"),
        type_: (ngx::ffi::NGX_HTTP_LOC_CONF | ngx::ffi::NGX_CONF_TAKE1) as usize,
        set: Some(ngx_http_x402_pay_to),
        conf: ngx::ffi::NGX_HTTP_LOC_CONF_OFFSET,
        offset: 0,
        post: ptr::null_mut(),
    },
    ngx_command_t {
        name: ngx_string!("x402_facilitator_url"),
        type_: (ngx::ffi::NGX_HTTP_LOC_CONF | ngx::ffi::NGX_CONF_TAKE1) as usize,
        set: Some(ngx_http_x402_facilitator_url),
        conf: ngx::ffi::NGX_HTTP_LOC_CONF_OFFSET,
        offset: 0,
        post: ptr::null_mut(),
    },
    ngx_command_t {
        name: ngx_string!("x402_description"),
        type_: (ngx::ffi::NGX_HTTP_LOC_CONF | ngx::ffi::NGX_CONF_TAKE1) as usize,
        set: Some(ngx_http_x402_description),
        conf: ngx::ffi::NGX_HTTP_LOC_CONF_OFFSET,
        offset: 0,
        post: ptr::null_mut(),
    },
    ngx_command_t {
        name: ngx_string!("x402_network"),
        type_: (ngx::ffi::NGX_HTTP_LOC_CONF | ngx::ffi::NGX_CONF_TAKE1) as usize,
        set: Some(ngx_http_x402_network),
        conf: ngx::ffi::NGX_HTTP_LOC_CONF_OFFSET,
        offset: 0,
        post: ptr::null_mut(),
    },
    ngx_command_t {
        name: ngx_string!("x402_network_id"),
        type_: (ngx::ffi::NGX_HTTP_LOC_CONF | ngx::ffi::NGX_CONF_TAKE1) as usize,
        set: Some(ngx_http_x402_network_id),
        conf: ngx::ffi::NGX_HTTP_LOC_CONF_OFFSET,
        offset: 0,
        post: ptr::null_mut(),
    },
    ngx_command_t {
        name: ngx_string!("x402_resource"),
        type_: (ngx::ffi::NGX_HTTP_LOC_CONF | ngx::ffi::NGX_CONF_TAKE1) as usize,
        set: Some(ngx_http_x402_resource),
        conf: ngx::ffi::NGX_HTTP_LOC_CONF_OFFSET,
        offset: 0,
        post: ptr::null_mut(),
    },
    ngx_command_t {
        name: ngx_string!("x402_asset"),
        type_: (ngx::ffi::NGX_HTTP_LOC_CONF | ngx::ffi::NGX_CONF_TAKE1) as usize,
        set: Some(ngx_http_x402_asset),
        conf: ngx::ffi::NGX_HTTP_LOC_CONF_OFFSET,
        offset: 0,
        post: ptr::null_mut(),
    },
    ngx_command_t {
        name: ngx_string!("x402_asset_decimals"),
        type_: (ngx::ffi::NGX_HTTP_LOC_CONF | ngx::ffi::NGX_CONF_TAKE1) as usize,
        set: Some(ngx_http_x402_asset_decimals),
        conf: ngx::ffi::NGX_HTTP_LOC_CONF_OFFSET,
        offset: 0,
        post: ptr::null_mut(),
    },
    ngx_command_t {
        name: ngx_string!("x402_timeout"),
        type_: (ngx::ffi::NGX_HTTP_LOC_CONF | ngx::ffi::NGX_CONF_TAKE1) as usize,
        set: Some(ngx_http_x402_timeout),
        conf: ngx::ffi::NGX_HTTP_LOC_CONF_OFFSET,
        offset: 0,
        post: ptr::null_mut(),
    },
    ngx_command_t {
        name: ngx_string!("x402_facilitator_fallback"),
        type_: (ngx::ffi::NGX_HTTP_LOC_CONF | ngx::ffi::NGX_CONF_TAKE1) as usize,
        set: Some(ngx_http_x402_facilitator_fallback),
        conf: ngx::ffi::NGX_HTTP_LOC_CONF_OFFSET,
        offset: 0,
        post: ptr::null_mut(),
    },
    ngx_command_t {
        name: ngx_string!("x402_ttl"),
        type_: (ngx::ffi::NGX_HTTP_LOC_CONF | ngx::ffi::NGX_CONF_TAKE1) as usize,
        set: Some(ngx_http_x402_ttl),
        conf: ngx::ffi::NGX_HTTP_LOC_CONF_OFFSET,
        offset: 0,
        post: ptr::null_mut(),
    },
    ngx_command_t {
        name: ngx_string!("x402_metrics"),
        type_: (ngx::ffi::NGX_HTTP_LOC_CONF | ngx::ffi::NGX_CONF_FLAG) as usize,
        set: Some(ngx_http_x402_metrics),
        conf: ngx::ffi::NGX_HTTP_LOC_CONF_OFFSET,
        offset: 0,
        post: ptr::null_mut(),
    },
    ngx_command_t {
        name: ngx_str_t {
            len: 0,
            data: ptr::null_mut(),
        },
        type_: 0,
        set: None,
        conf: 0,
        offset: 0,
        post: ptr::null_mut(),
    },
];
