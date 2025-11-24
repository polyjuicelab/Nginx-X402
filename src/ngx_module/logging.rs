//! Logging functions for the Nginx module
//!
//! This module provides logging functionality for the x402 module.
//! Uses Rust's standard `log` crate with a custom logger that writes to stderr.

use log::{Log, Metadata, Record};
use ngx::http::Request;
use std::sync::Once;

/// Simple logger that writes to stderr (appears in Docker logs and Nginx error log)
struct NginxLogger;

impl Log for NginxLogger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            // Use eprintln! to write to stderr
            // This will appear in Docker logs and Nginx error log
            eprintln!("[x402][{}] {}", record.level(), record.args());
        }
    }

    fn flush(&self) {}
}

static LOGGER: NginxLogger = NginxLogger;
static INIT: Once = Once::new();

/// Initialize the logger
pub fn init() {
    INIT.call_once(|| {
        log::set_logger(&LOGGER)
            .map(|()| log::set_max_level(log::LevelFilter::Debug))
            .expect("Failed to initialize logger");
    });
}

/// Log a message using Rust's log crate
#[inline]
pub fn log_message(_r: Option<&Request>, level: &str, message: &str) {
    // Initialize logger if not already done
    init();

    // Use Rust's log crate
    match level {
        "error" => log::error!("{}", message),
        "warn" => log::warn!("{}", message),
        "info" => log::info!("{}", message),
        "debug" => log::debug!("{}", message),
        _ => log::error!("{}", message),
    }
}

/// Log an error message
#[inline]
pub fn log_error(r: Option<&Request>, message: &str) {
    log_message(r, "error", message);
}

/// Log a warning message
#[inline]
pub fn log_warn(r: Option<&Request>, message: &str) {
    log_message(r, "warn", message);
}

/// Log an info message
#[inline]
pub fn log_info(r: Option<&Request>, message: &str) {
    log_message(r, "info", message);
}

/// Log a debug message
#[inline]
pub fn log_debug(r: Option<&Request>, message: &str) {
    log_message(r, "debug", message);
}
