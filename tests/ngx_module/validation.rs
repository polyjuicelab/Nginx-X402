//! Validation and error handling tests

use super::common::{create_requirements_test, TestConfig};

#[test]
fn test_create_requirements_missing_amount() {
    let mut config = TestConfig::new();
    config.amount = None;

    let result = create_requirements_test(&config, "/test");
    assert!(result.is_err(), "Must return error when amount is missing");
    let error = result.unwrap_err();
    assert_eq!(
        error, "Amount not configured",
        "Error message must exactly match 'Amount not configured'"
    );
}

#[test]
fn test_create_requirements_missing_pay_to() {
    let mut config = TestConfig::new();
    config.pay_to = None;

    let result = create_requirements_test(&config, "/test");
    assert!(result.is_err(), "Must return error when pay_to is missing");
    let error = result.unwrap_err();
    assert_eq!(
        error, "Pay-to address not configured",
        "Error message must exactly match 'Pay-to address not configured'"
    );
}

#[test]
fn test_create_requirements_empty_pay_to() {
    let mut config = TestConfig::new();
    config.pay_to = Some("".to_string());

    let result = create_requirements_test(&config, "/test");
    assert!(result.is_err(), "Must return error for empty pay_to");
    let error = result.unwrap_err();
    assert_eq!(
        error, "Pay-to address cannot be empty",
        "Error message must exactly match 'Pay-to address cannot be empty'"
    );
}

#[test]
fn test_create_requirements_whitespace_pay_to() {
    let mut config = TestConfig::new();
    config.pay_to = Some("   ".to_string());

    let result = create_requirements_test(&config, "/test");
    assert!(
        result.is_err(),
        "Must return error for whitespace-only pay_to"
    );
    let error = result.unwrap_err();
    assert_eq!(
        error, "Pay-to address cannot be empty",
        "Error message must exactly match 'Pay-to address cannot be empty'"
    );
}

#[test]
fn test_create_requirements_empty_resource() {
    let mut config = TestConfig::new();
    config.resource = Some("".to_string());

    let result = create_requirements_test(&config, "/test");
    assert!(result.is_err(), "Must return error for empty resource");
    let error = result.unwrap_err();
    assert_eq!(
        error, "Resource URL cannot be empty",
        "Error message must exactly match 'Resource URL cannot be empty'"
    );
}

#[test]
fn test_create_requirements_empty_resource_path() {
    let config = TestConfig::new();

    let result = create_requirements_test(&config, "");
    assert!(result.is_err(), "Must return error for empty resource path");
    let error = result.unwrap_err();
    assert_eq!(
        error, "Resource path cannot be empty",
        "Error message must exactly match 'Resource path cannot be empty'"
    );
}

#[test]
fn test_create_requirements_whitespace_resource() {
    let mut config = TestConfig::new();
    config.resource = Some("   ".to_string());

    let result = create_requirements_test(&config, "/test");
    assert!(
        result.is_err(),
        "Must return error for whitespace-only resource"
    );
    let error = result.unwrap_err();
    assert_eq!(
        error, "Resource URL cannot be empty",
        "Error message must exactly match 'Resource URL cannot be empty'"
    );
}
