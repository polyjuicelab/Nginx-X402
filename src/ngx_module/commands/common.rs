//! Common utilities for command parsing
//!
//! This module provides shared helper functions used by all command handlers.

use ngx::core::{NgxStr, Pool};
use ngx::ffi::{ngx_conf_t, ngx_str_t};
use std::ptr;

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
///
/// # Safety
///
/// The caller must ensure that:
/// * `cf` is a valid pointer to a `ngx_conf_t` structure
/// * `src.data` points to valid memory if `src.len > 0`
/// * The source string is valid UTF-8 if it will be converted to a Rust string
pub unsafe fn copy_string_to_pool(cf: *mut ngx_conf_t, src: ngx_str_t) -> Option<ngx_str_t> {
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
