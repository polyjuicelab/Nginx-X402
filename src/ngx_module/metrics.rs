//! Prometheus metrics collection for x402 Nginx module
//!
//! This module provides Prometheus metrics for monitoring x402 payment verification.
//! Metrics are exposed via a `/metrics` endpoint that can be scraped by Prometheus
//! and visualized in Grafana.

use prometheus::{
    register_histogram_with_registry, register_int_counter_with_registry, Histogram, IntCounter,
    Registry,
};
use std::sync::OnceLock;

/// Global Prometheus registry for x402 metrics
static REGISTRY: OnceLock<Registry> = OnceLock::new();

/// Metrics structure containing all Prometheus metrics
pub struct X402Metrics {
    /// Total number of requests processed by x402 module
    pub requests_total: IntCounter,
    /// Total number of payment verifications attempted
    pub payment_verifications_total: IntCounter,
    /// Total number of successful payment verifications
    pub payment_verifications_success_total: IntCounter,
    /// Total number of failed payment verifications
    pub payment_verifications_failed_total: IntCounter,
    /// Total number of 402 responses sent
    pub responses_402_total: IntCounter,
    /// Total number of facilitator errors
    pub facilitator_errors_total: IntCounter,
    /// Payment verification duration in seconds
    pub verification_duration_seconds: Histogram,
    /// Payment amount histogram (for tracking payment amounts)
    pub payment_amount: Histogram,
}

impl X402Metrics {
    /// Initialize metrics with a new registry
    pub fn new() -> Result<Self, prometheus::Error> {
        let registry = REGISTRY.get_or_init(Registry::new);

        let requests_total = register_int_counter_with_registry!(
            "x402_requests_total",
            "Total number of requests processed by x402 module",
            registry
        )?;

        let payment_verifications_total = register_int_counter_with_registry!(
            "x402_payment_verifications_total",
            "Total number of payment verifications attempted",
            registry
        )?;

        let payment_verifications_success_total = register_int_counter_with_registry!(
            "x402_payment_verifications_success_total",
            "Total number of successful payment verifications",
            registry
        )?;

        let payment_verifications_failed_total = register_int_counter_with_registry!(
            "x402_payment_verifications_failed_total",
            "Total number of failed payment verifications",
            registry
        )?;

        let responses_402_total = register_int_counter_with_registry!(
            "x402_responses_402_total",
            "Total number of 402 Payment Required responses sent",
            registry
        )?;

        let facilitator_errors_total = register_int_counter_with_registry!(
            "x402_facilitator_errors_total",
            "Total number of facilitator service errors",
            registry
        )?;

        let verification_duration_seconds = register_histogram_with_registry!(
            "x402_verification_duration_seconds",
            "Payment verification duration in seconds",
            vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 5.0],
            registry
        )?;

        let payment_amount = register_histogram_with_registry!(
            "x402_payment_amount",
            "Payment amount in USDC",
            vec![0.0001, 0.001, 0.01, 0.1, 1.0, 10.0, 100.0],
            registry
        )?;

        Ok(Self {
            requests_total,
            payment_verifications_total,
            payment_verifications_success_total,
            payment_verifications_failed_total,
            responses_402_total,
            facilitator_errors_total,
            verification_duration_seconds,
            payment_amount,
        })
    }

    /// Get the global metrics instance
    ///
    /// # Panics
    ///
    /// This function will panic if metrics initialization fails. This is intentional
    /// because metrics are essential for monitoring and the module cannot function
    /// properly without them. In practice, Prometheus metrics initialization rarely fails
    /// unless there's a serious system issue (e.g., memory exhaustion).
    pub fn get() -> &'static Self {
        static METRICS: OnceLock<X402Metrics> = OnceLock::new();
        METRICS.get_or_init(|| {
            X402Metrics::new().expect(
                "Failed to initialize Prometheus metrics - this indicates a serious system issue",
            )
        })
    }

    /// Record a request being processed
    pub fn record_request(&self) {
        self.requests_total.inc();
    }

    /// Record a payment verification attempt
    pub fn record_verification_attempt(&self) {
        self.payment_verifications_total.inc();
    }

    /// Record a successful payment verification
    pub fn record_verification_success(&self) {
        self.payment_verifications_success_total.inc();
    }

    /// Record a failed payment verification
    pub fn record_verification_failed(&self) {
        self.payment_verifications_failed_total.inc();
    }

    /// Record a 402 response being sent
    pub fn record_402_response(&self) {
        self.responses_402_total.inc();
    }

    /// Record a facilitator error
    pub fn record_facilitator_error(&self) {
        self.facilitator_errors_total.inc();
    }

    /// Record payment verification duration
    pub fn record_verification_duration(&self, duration_seconds: f64) {
        self.verification_duration_seconds.observe(duration_seconds);
    }

    /// Record payment amount
    pub fn record_payment_amount(&self, amount: f64) {
        self.payment_amount.observe(amount);
    }
}

/// Get the Prometheus registry
pub fn get_registry() -> &'static Registry {
    REGISTRY.get_or_init(Registry::new)
}

/// Collect all metrics in Prometheus text format
#[must_use] 
pub fn collect_metrics() -> String {
    let registry = get_registry();
    let encoder = prometheus::TextEncoder::new();
    encoder
        .encode_to_string(&registry.gather())
        .unwrap_or_else(|e| format!("Error encoding metrics: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_initialization() {
        // Test that metrics can be accessed via get()
        // Note: X402Metrics::new() will fail if called multiple times because
        // metrics are registered to a global registry. Use get() instead for singleton access.
        let metrics = X402Metrics::get();
        // Should always succeed - get() initializes if needed
        // We can't assert a specific value because other tests may have incremented counters
        let _ = metrics.requests_total.get();
    }

    #[test]
    fn test_metrics_singleton() {
        let metrics1 = X402Metrics::get();
        let metrics2 = X402Metrics::get();
        // Should return the same instance
        assert_eq!(metrics1.requests_total.get(), metrics2.requests_total.get());
    }

    #[test]
    fn test_record_request() {
        let metrics = X402Metrics::get();
        let initial = metrics.requests_total.get();
        metrics.record_request();
        assert_eq!(metrics.requests_total.get(), initial + 1);
    }

    #[test]
    fn test_record_verification() {
        let metrics = X402Metrics::get();
        let initial_attempts = metrics.payment_verifications_total.get();
        let initial_success = metrics.payment_verifications_success_total.get();
        let initial_failed = metrics.payment_verifications_failed_total.get();

        metrics.record_verification_attempt();
        assert_eq!(
            metrics.payment_verifications_total.get(),
            initial_attempts + 1
        );

        metrics.record_verification_success();
        assert_eq!(
            metrics.payment_verifications_success_total.get(),
            initial_success + 1
        );

        metrics.record_verification_failed();
        assert_eq!(
            metrics.payment_verifications_failed_total.get(),
            initial_failed + 1
        );
    }

    #[test]
    fn test_record_402_response() {
        let metrics = X402Metrics::get();
        let initial = metrics.responses_402_total.get();
        metrics.record_402_response();
        assert_eq!(metrics.responses_402_total.get(), initial + 1);
    }

    #[test]
    fn test_record_facilitator_error() {
        let metrics = X402Metrics::get();
        let initial = metrics.facilitator_errors_total.get();
        metrics.record_facilitator_error();
        assert_eq!(metrics.facilitator_errors_total.get(), initial + 1);
    }

    #[test]
    fn test_record_duration() {
        let metrics = X402Metrics::get();
        metrics.record_verification_duration(0.1);
        metrics.record_verification_duration(0.5);
        // Histogram should have recorded these values
        // We can't easily test histogram internals, but we can verify it doesn't panic
    }

    #[test]
    fn test_record_payment_amount() {
        let metrics = X402Metrics::get();
        metrics.record_payment_amount(0.0001);
        metrics.record_payment_amount(0.01);
        // Histogram should have recorded these values
    }

    #[test]
    fn test_collect_metrics() {
        let metrics = X402Metrics::get();
        metrics.record_request();
        metrics.record_402_response();

        let output = collect_metrics();
        assert!(output.contains("x402_requests_total"));
        assert!(output.contains("x402_responses_402_total"));
    }
}
