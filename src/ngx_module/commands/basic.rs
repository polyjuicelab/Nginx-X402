//! Basic configuration command handlers
//!
//! This module contains handlers for basic x402 configuration directives:
//! - `x402` (on/off)
//! - `x402_amount`
//! - `x402_pay_to`
//! - `x402_facilitator_url`
//! - `x402_description`

use crate::ngx_module::commands::common::copy_string_to_pool;
use crate::ngx_module::config::X402Config;
use ngx::core::{NgxStr, Pool};
use ngx::ffi::{
    ngx_command_t, ngx_conf_t, ngx_http_core_loc_conf_t, ngx_http_handler_pt, ngx_str_t,
};
use std::ffi::c_char;
use std::ptr;

// Import handler functions for setting in location configuration
extern "C" {
    pub fn x402_ngx_handler(r: *mut ngx::ffi::ngx_http_request_t) -> ngx::ffi::ngx_int_t;
}

/// Parse x402 on/off directive
///
/// This function is called by nginx when parsing the `x402` directive in the
/// configuration file. It enables or disables the x402 module for a location
/// and sets the content handler when enabled.
///
/// # Arguments
///
/// * `cf` - Nginx configuration context
/// * `_cmd` - Command structure (unused)
/// * `conf` - Module configuration structure
///
/// # Returns
///
/// * `ptr::null_mut()` - Success
/// * `*mut c_char` - Error message (allocated from pool) on failure
///
/// # Safety
///
/// This function is marked `unsafe` because it performs raw pointer operations
/// and accesses nginx internal structures.
pub(crate) unsafe extern "C" fn ngx_http_x402(
    cf: *mut ngx_conf_t,
    _cmd: *mut ngx_command_t,
    conf: *mut core::ffi::c_void,
) -> *mut c_char {
    let conf = conf.cast::<X402Config>();
    if conf.is_null() {
        return ptr::null_mut();
    }

    let args = (*cf).args;
    if args.is_null() || (*args).nelts < 2 {
        return ptr::null_mut();
    }

    // Get the second argument (value: "on" or "off")
    // elts is a pointer to an array of ngx_str_t, not an array of pointers
    let elts = (*args).elts.cast::<ngx_str_t>();
    let value_str = NgxStr::from_ngx_str(*elts.add(1));

    match value_str.to_str().ok().map(str::to_lowercase) {
        Some(ref s) if s == "on" => {
            (*conf).enabled = 1;

            // CRITICAL: Verify we're in location context before setting handler
            // Handler MUST be set in location context, not in server or main context
            // Check context by verifying loc_conf is available
            let ctx = (*cf).ctx.cast::<ngx::ffi::ngx_http_conf_ctx_t>();
            if ctx.is_null() {
                // Not in HTTP context - cannot set handler
                // This should not happen for HTTP module, but we check anyway
                return ptr::null_mut();
            }

            let loc_conf = (*ctx).loc_conf;
            if loc_conf.is_null() {
                // Not in location context - cannot set handler
                // This means x402 on; was called in server or main context
                // Handler cannot be set here, it must be in location context
                // Return error string to indicate configuration error
                // Nginx will report this as a configuration error
                // We need to allocate error message from pool
                let pool = Pool::from_ngx_pool((*cf).pool);
                let error_msg = "\"x402\" directive is not allowed here";
                let msg_len = error_msg.len();
                let msg_ptr = pool.alloc(msg_len).cast::<u8>();
                if !msg_ptr.is_null() {
                    ptr::copy_nonoverlapping(error_msg.as_ptr(), msg_ptr, msg_len);
                    return msg_ptr.cast::<c_char>();
                }
                return ptr::null_mut();
            }

            // We're in location context - proceed to set handler
            // Get core module's location config using ctx_index
            // loc_conf is *mut *mut c_void (array of pointers)
            // Use loc_conf.add() directly, not (*loc_conf).add()
            let core_ctx_index = ngx::ffi::ngx_http_core_module.ctx_index;
            unsafe {
                let ptr_to_ptr = loc_conf.add(core_ctx_index);
                if !ptr_to_ptr.is_null() {
                    // Read the pointer value
                    let clcf_void: *mut core::ffi::c_void = ptr::read(ptr_to_ptr.cast_const());
                    if !clcf_void.is_null() {
                        let clcf: *mut ngx_http_core_loc_conf_t = core::mem::transmute(clcf_void);

                        // Set the content handler
                        // According to Nginx documentation, handler should be set in command handler
                        // when directive is parsed in location context
                        let handler_ptr: ngx_http_handler_pt = Some(x402_ngx_handler);
                        (*clcf).handler = handler_ptr;

                        // Verify handler was successfully set
                        if (*clcf).handler.is_none() {
                            // Handler was not set - this is a critical error
                            // Return error message to indicate configuration failure
                            let pool = Pool::from_ngx_pool((*cf).pool);
                            let error_msg = "failed to set x402 handler";
                            let msg_len = error_msg.len();
                            let msg_ptr = pool.alloc(msg_len).cast::<u8>();
                            if !msg_ptr.is_null() {
                                ptr::copy_nonoverlapping(error_msg.as_ptr(), msg_ptr, msg_len);
                                return msg_ptr.cast::<c_char>();
                            }
                            return ptr::null_mut();
                        }
                    }
                }
            }
        }
        Some(ref s) if s == "off" => {
            (*conf).enabled = 0;

            // Clear the content handler only if we're in a location context
            let ctx = (*cf).ctx.cast::<ngx::ffi::ngx_http_conf_ctx_t>();
            if !ctx.is_null() {
                let loc_conf = (*ctx).loc_conf;
                if !loc_conf.is_null() {
                    let core_ctx_index = ngx::ffi::ngx_http_core_module.ctx_index;
                    unsafe {
                        let ptr_to_ptr = loc_conf.add(core_ctx_index);
                        if !ptr_to_ptr.is_null() {
                            // Dereference to get *mut c_void
                            let clcf_void: *mut core::ffi::c_void =
                                ptr::read(ptr_to_ptr.cast_const());
                            if !clcf_void.is_null() {
                                let clcf: *mut ngx_http_core_loc_conf_t =
                                    core::mem::transmute(clcf_void);
                                // Clear the content handler
                                (*clcf).handler = None;
                            }
                        }
                    }
                }
            }
        }
        _ => {
            return ptr::null_mut();
        }
    }

    ptr::null_mut()
}

/// Parse `x402_amount` directive
pub(crate) unsafe extern "C" fn ngx_http_x402_amount(
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
            (*conf).amount_str = allocated_str;
        }
        None => {
            return ngx::core::NGX_CONF_ERROR.cast::<c_char>();
        }
    }

    ptr::null_mut()
}

/// Parse `x402_pay_to` directive
pub(crate) unsafe extern "C" fn ngx_http_x402_pay_to(
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
            (*conf).pay_to_str = allocated_str;
        }
        None => {
            return ngx::core::NGX_CONF_ERROR.cast::<c_char>();
        }
    }

    ptr::null_mut()
}

/// Parse `x402_facilitator_url` directive
pub(crate) unsafe extern "C" fn ngx_http_x402_facilitator_url(
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
            (*conf).facilitator_url_str = allocated_str;
        }
        None => {
            return ngx::core::NGX_CONF_ERROR.cast::<c_char>();
        }
    }

    ptr::null_mut()
}

/// Parse `x402_description` directive
pub(crate) unsafe extern "C" fn ngx_http_x402_description(
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
            (*conf).description_str = allocated_str;
        }
        None => {
            return ngx::core::NGX_CONF_ERROR.cast::<c_char>();
        }
    }

    ptr::null_mut()
}

