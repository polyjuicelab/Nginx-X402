//! Unit tests for Prometheus metrics functionality

use nginx_x402::ngx_module::metrics::{collect_metrics, X402Metrics};

#[test]
fn test_metrics_collection() {
    // Initialize metrics
    let metrics = X402Metrics::get();

    // Record some metrics
    metrics.record_request();
    metrics.record_verification_attempt();
    metrics.record_verification_success();
    metrics.record_402_response();
    metrics.record_verification_duration(0.1);
    metrics.record_payment_amount(0.0001);

    // Collect metrics
    let output = collect_metrics();

    // Verify metrics are present
    assert!(output.contains("x402_requests_total"));
    assert!(output.contains("x402_payment_verifications_total"));
    assert!(output.contains("x402_payment_verifications_success_total"));
    assert!(output.contains("x402_responses_402_total"));
    assert!(output.contains("x402_verification_duration_seconds"));
    assert!(output.contains("x402_payment_amount"));
}

#[test]
fn test_metrics_format() {
    let metrics = X402Metrics::get();
    metrics.record_request();

    let output = collect_metrics();

    // Verify Prometheus text format
    assert!(output.contains("# TYPE"));
    assert!(output.contains("# HELP"));
    assert!(output.contains("x402_requests_total"));
}

#[test]
fn test_metrics_counters() {
    let metrics = X402Metrics::get();

    let initial_requests = metrics.requests_total.get();
    let initial_verifications = metrics.payment_verifications_total.get();
    let initial_success = metrics.payment_verifications_success_total.get();
    let initial_failed = metrics.payment_verifications_failed_total.get();
    let initial_402 = metrics.responses_402_total.get();
    let initial_errors = metrics.facilitator_errors_total.get();

    // Record multiple events
    metrics.record_request();
    metrics.record_request();
    metrics.record_verification_attempt();
    metrics.record_verification_success();
    metrics.record_verification_failed();
    metrics.record_402_response();
    metrics.record_facilitator_error();

    // Verify counters incremented
    assert_eq!(metrics.requests_total.get(), initial_requests + 2);
    assert_eq!(
        metrics.payment_verifications_total.get(),
        initial_verifications + 1
    );
    assert_eq!(
        metrics.payment_verifications_success_total.get(),
        initial_success + 1
    );
    assert_eq!(
        metrics.payment_verifications_failed_total.get(),
        initial_failed + 1
    );
    assert_eq!(metrics.responses_402_total.get(), initial_402 + 1);
    assert_eq!(metrics.facilitator_errors_total.get(), initial_errors + 1);
}

#[test]
fn test_metrics_histograms() {
    let metrics = X402Metrics::get();

    // Record various durations
    metrics.record_verification_duration(0.001);
    metrics.record_verification_duration(0.01);
    metrics.record_verification_duration(0.1);
    metrics.record_verification_duration(1.0);

    // Record various payment amounts
    metrics.record_payment_amount(0.0001);
    metrics.record_payment_amount(0.001);
    metrics.record_payment_amount(0.01);
    metrics.record_payment_amount(0.1);

    // Collect metrics and verify histograms are present
    let output = collect_metrics();
    assert!(output.contains("x402_verification_duration_seconds"));
    assert!(output.contains("x402_payment_amount"));
}

#[test]
fn test_metrics_singleton() {
    let metrics1 = X402Metrics::get();
    let metrics2 = X402Metrics::get();

    // Both should reference the same instance
    let initial = metrics1.requests_total.get();
    metrics1.record_request();
    assert_eq!(metrics2.requests_total.get(), initial + 1);
}

#[test]
fn test_metrics_registry() {
    use nginx_x402::ngx_module::metrics::get_registry;

    // Initialize metrics first to register them
    let _metrics = X402Metrics::get();
    let registry = get_registry();
    let metrics = registry.gather();

    // Should have metrics registered
    assert!(!metrics.is_empty());
}
