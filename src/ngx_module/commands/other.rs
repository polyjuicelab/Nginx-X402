//! Other configuration command handlers
//!
//! This module contains handlers for miscellaneous configuration directives:
//! - `x402_timeout`
//! - `x402_facilitator_fallback`
//! - `x402_metrics`

use crate::ngx_module::commands::common::copy_string_to_pool;
use crate::ngx_module::config::X402Config;
use ngx::core::Pool;
use ngx::ffi::{
    ngx_command_t, ngx_conf_t, ngx_http_core_loc_conf_t, ngx_http_handler_pt, ngx_str_t,
};
use std::ffi::c_char;
use std::ptr;

// Import handler functions for setting in location configuration
extern "C" {
    pub fn x402_metrics_handler(r: *mut ngx::ffi::ngx_http_request_t) -> ngx::ffi::ngx_int_t;
}

/// Parse `x402_timeout` directive
pub(crate) unsafe extern "C" fn ngx_http_x402_timeout(
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
            (*conf).timeout_str = allocated_str;
        }
        None => {
            return ngx::core::NGX_CONF_ERROR.cast::<c_char>();
        }
    }

    ptr::null_mut()
}

/// Parse `x402_facilitator_fallback` directive
pub(crate) unsafe extern "C" fn ngx_http_x402_facilitator_fallback(
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
            (*conf).facilitator_fallback_str = allocated_str;
        }
        None => {
            return ngx::core::NGX_CONF_ERROR.cast::<c_char>();
        }
    }

    ptr::null_mut()
}

/// Parse `x402_metrics` directive
pub(crate) unsafe extern "C" fn ngx_http_x402_metrics(
    cf: *mut ngx_conf_t,
    _cmd: *mut ngx_command_t,
    _conf: *mut core::ffi::c_void,
) -> *mut c_char {
    // Set the metrics handler when x402_metrics on; is parsed
    // Similar to how x402 on; sets the payment handler

    // Verify we're in location context before setting handler
    let ctx = (*cf).ctx.cast::<ngx::ffi::ngx_http_conf_ctx_t>();
    if ctx.is_null() {
        return ptr::null_mut();
    }

    let loc_conf = (*ctx).loc_conf;
    if loc_conf.is_null() {
        // Not in location context - return error
        let pool = Pool::from_ngx_pool((*cf).pool);
        let error_msg = "\"x402_metrics\" directive is not allowed here";
        let msg_len = error_msg.len();
        let msg_ptr = pool.alloc(msg_len).cast::<u8>();
        if !msg_ptr.is_null() {
            ptr::copy_nonoverlapping(error_msg.as_ptr(), msg_ptr, msg_len);
            return msg_ptr.cast::<c_char>();
        }
        return ptr::null_mut();
    }

    // We're in location context - proceed to set metrics handler
    let core_ctx_index = ngx::ffi::ngx_http_core_module.ctx_index;
    unsafe {
        let ptr_to_ptr = loc_conf.add(core_ctx_index);
        if !ptr_to_ptr.is_null() {
            let clcf_void: *mut core::ffi::c_void = ptr::read(ptr_to_ptr.cast_const());
            if !clcf_void.is_null() {
                let clcf: *mut ngx_http_core_loc_conf_t = core::mem::transmute(clcf_void);

                // Set the metrics handler
                let handler_ptr: ngx_http_handler_pt = Some(x402_metrics_handler);
                (*clcf).handler = handler_ptr;
            }
        }
    }

    ptr::null_mut()
}

