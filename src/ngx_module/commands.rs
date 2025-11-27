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

use crate::ngx_module::config::X402Config;
use ngx::core::{NgxStr, Pool};
use ngx::ffi::{
    ngx_command_t, ngx_conf_t, ngx_http_core_loc_conf_t, ngx_http_handler_pt, ngx_str_t,
};
use std::ffi::c_char;
use std::ptr;

use ngx::ngx_string;

// Import handler functions for setting in location configuration
extern "C" {
    pub fn x402_ngx_handler(r: *mut ngx::ffi::ngx_http_request_t) -> ngx::ffi::ngx_int_t;
    pub fn x402_metrics_handler(r: *mut ngx::ffi::ngx_http_request_t) -> ngx::ffi::ngx_int_t;
}

/// Helper function to copy a string from configuration args to pool-allocated memory
///
/// This function allocates memory from the nginx configuration pool and copies
/// the source string. This ensures the string lives as long as the configuration
/// and is properly managed by nginx's memory pool system.
///
/// # Arguments
///
/// * `cf` - Nginx configuration context
/// * `src` - Source string to copy
///
/// # Returns
///
/// * `Some(ngx_str_t)` - Successfully allocated and copied string
/// * `None` - Failed to allocate memory
unsafe fn copy_string_to_pool(cf: *mut ngx_conf_t, src: ngx_str_t) -> Option<ngx_str_t> {
    if src.len == 0 {
        return Some(ngx_str_t {
            len: 0,
            data: ptr::null_mut(),
        });
    }

    let pool = Pool::from_ngx_pool((*cf).pool);
    let ngx_str = NgxStr::from_ngx_str(src);

    match ngx_str.to_str() {
        Ok(s) => {
            let len = s.len();
            // Allocate memory from the pool using alloc
            let data = pool.alloc(len).cast::<u8>();
            if data.is_null() {
                return None;
            }
            // Copy the string data
            ptr::copy_nonoverlapping(s.as_ptr(), data, len);

            Some(ngx_str_t { len, data })
        }
        Err(_) => None,
    }
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
unsafe extern "C" fn ngx_http_x402(
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
unsafe extern "C" fn ngx_http_x402_amount(
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
unsafe extern "C" fn ngx_http_x402_pay_to(
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
unsafe extern "C" fn ngx_http_x402_facilitator_url(
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
unsafe extern "C" fn ngx_http_x402_description(
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

/// Parse `x402_network` directive
unsafe extern "C" fn ngx_http_x402_network(
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

/// Parse `x402_resource` directive
unsafe extern "C" fn ngx_http_x402_resource(
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

    ptr::null_mut()
}

/// Parse `x402_timeout` directive
unsafe extern "C" fn ngx_http_x402_timeout(
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
unsafe extern "C" fn ngx_http_x402_facilitator_fallback(
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
unsafe extern "C" fn ngx_http_x402_metrics(
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
pub static mut ngx_http_x402_commands: [ngx_command_t; 11] = [
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
        name: ngx_string!("x402_resource"),
        type_: (ngx::ffi::NGX_HTTP_LOC_CONF | ngx::ffi::NGX_CONF_TAKE1) as usize,
        set: Some(ngx_http_x402_resource),
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
