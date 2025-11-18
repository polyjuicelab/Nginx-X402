//! Tests for ngx-rust module implementation
//!
//! These tests verify the core payment verification logic that can be used
//! with the ngx-rust module, even without a full Nginx setup.
//!
//! Note: These tests can run without Nginx source code by testing only the
//! core logic functions that don't depend on ngx-rust types.

mod common;

mod address;
mod amount;
mod basic;
mod fields;
mod resource;
mod validation;
