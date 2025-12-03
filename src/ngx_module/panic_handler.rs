//! Panic handling for FFI boundaries
//!
//! This module provides panic handling utilities to prevent panics from crossing
//! FFI boundaries, which would cause undefined behavior. It also provides detailed
//! logging when panics occur to help with debugging.

use crate::ngx_module::logging::log_error;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::Once;

static PANIC_HOOK_INIT: Once = Once::new();

/// Initialize panic hook for FFI safety
///
/// This sets up a custom panic hook that logs panics to nginx error log
/// instead of stderr. This is important because panics in FFI code can cause
/// undefined behavior if they cross the FFI boundary.
pub fn init_panic_hook() {
    PANIC_HOOK_INIT.call_once(|| {
        std::panic::set_hook(Box::new(|panic_info| {
            log_panic(panic_info);
        }));
    });
}

/// Log panic information to nginx error log
fn log_panic(panic_info: &std::panic::PanicHookInfo) {
    let message = if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
        format!("Panic: {}", s)
    } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
        format!("Panic: {}", s)
    } else {
        "Panic: unknown panic payload".to_string()
    };

    let location = if let Some(location) = panic_info.location() {
        format!(
            "{}:{}:{}",
            location.file(),
            location.line(),
            location.column()
        )
    } else {
        "unknown location".to_string()
    };

    log_error(
        None,
        &format!("[x402] FFI PANIC DETECTED: {} at {}", message, location),
    );

    // Try to get backtrace if available (requires RUST_BACKTRACE=1)
    if std::env::var("RUST_BACKTRACE").is_ok() {
        // Backtrace is automatically captured by panic hook when RUST_BACKTRACE is set
        // We can't easily access it here, but it will be printed to stderr
        log_error(
            None,
            "[x402] Panic occurred. Set RUST_BACKTRACE=1 for detailed backtrace.",
        );
    }
}

/// Execute a function with panic protection
///
/// This wrapper catches any panics and converts them to error logs,
/// preventing panics from crossing FFI boundaries.
///
/// # Arguments
///
/// * `f` - The function to execute
/// * `context` - Context string for logging (e.g., function name)
///
/// # Returns
///
/// * `Some(T)` - Function result if successful
/// * `None` - If panic occurred
pub fn catch_panic<F, T>(f: F, context: &str) -> Option<T>
where
    F: FnOnce() -> T,
{
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(result) => Some(result),
        Err(e) => {
            let message = if let Some(s) = e.downcast_ref::<&str>() {
                format!("Panic in {}: {}", context, s)
            } else if let Some(s) = e.downcast_ref::<String>() {
                format!("Panic in {}: {}", context, s)
            } else {
                format!("Panic in {}: unknown panic payload", context)
            };

            log_error(None, &format!("[x402] {}", message));

            // Log backtrace hint if RUST_BACKTRACE is set
            if std::env::var("RUST_BACKTRACE").is_ok() {
                log_error(
                    None,
                    &format!(
                        "[x402] Panic in {}. Backtrace available in stderr (RUST_BACKTRACE=1).",
                        context
                    ),
                );
            }

            None
        }
    }
}

/// Execute a function with panic protection and return a default value on panic
///
/// This is useful for FFI functions that need to return a specific value
/// (like NGX_ERROR) when a panic occurs.
///
/// # Arguments
///
/// * `f` - The function to execute
/// * `context` - Context string for logging
/// * `default` - Default value to return if panic occurs
///
/// # Returns
///
/// * Function result if successful, or `default` if panic occurred
pub fn catch_panic_or_default<F, T>(f: F, context: &str, default: T) -> T
where
    F: FnOnce() -> T,
{
    catch_panic(f, context).unwrap_or(default)
}

/// Execute a function with panic protection and detailed error logging
///
/// This version includes additional context information in the error log.
///
/// # Arguments
///
/// * `f` - The function to execute
/// * `context` - Context string for logging
/// * `additional_info` - Additional information to include in logs
///
/// # Returns
///
/// * `Some(T)` - Function result if successful
/// * `None` - If panic occurred
pub fn catch_panic_with_info<F, T>(f: F, context: &str, additional_info: &str) -> Option<T>
where
    F: FnOnce() -> T,
{
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(result) => Some(result),
        Err(e) => {
            let message = if let Some(s) = e.downcast_ref::<&str>() {
                format!("Panic in {}: {} | Context: {}", context, s, additional_info)
            } else if let Some(s) = e.downcast_ref::<String>() {
                format!("Panic in {}: {} | Context: {}", context, s, additional_info)
            } else {
                format!(
                    "Panic in {}: unknown panic payload | Context: {}",
                    context, additional_info
                )
            };

            log_error(None, &format!("[x402] {}", message));

            // Log backtrace hint if RUST_BACKTRACE is set
            if std::env::var("RUST_BACKTRACE").is_ok() {
                log_error(
                    None,
                    &format!(
                        "[x402] Panic in {} ({}). Backtrace available in stderr (RUST_BACKTRACE=1).",
                        context, additional_info
                    ),
                );
            }

            None
        }
    }
}
