//! Amount-related tests

use super::common::{create_requirements_test, TestConfig};
use rust_decimal::Decimal;
use std::str::FromStr;

#[test]
fn test_create_requirements_amount_conversion() {
    let mut config = TestConfig::new();
    config.amount = Some(Decimal::from_str("1.0").unwrap());

    let requirements = create_requirements_test(&config, "/test").unwrap();
    // 1.0 USDC = 1,000,000 in smallest units (6 decimals)
    assert_eq!(
        requirements.max_amount_required, "1000000",
        "1.0 USDC must equal exactly 1,000,000 in smallest units"
    );

    // Verify the conversion is reversible
    let amount_back =
        Decimal::from_str(&requirements.max_amount_required).unwrap() / Decimal::from(1_000_000u64);
    assert_eq!(
        amount_back,
        Decimal::from_str("1.0").unwrap(),
        "Amount conversion must be reversible"
    );
}

#[test]
fn test_create_requirements_negative_amount() {
    let mut config = TestConfig::new();
    config.amount = Some(Decimal::from_str("-0.0001").unwrap());

    let result = create_requirements_test(&config, "/test");
    assert!(result.is_err(), "Must return error for negative amount");
    let error = result.unwrap_err();
    assert_eq!(
        error, "Amount cannot be negative",
        "Error message must exactly match 'Amount cannot be negative'"
    );
}

#[test]
fn test_create_requirements_zero_amount() {
    let mut config = TestConfig::new();
    config.amount = Some(Decimal::ZERO);

    let requirements = create_requirements_test(&config, "/test").unwrap();
    assert_eq!(
        requirements.max_amount_required, "0",
        "Zero amount must result in '0' string"
    );

    // Verify all other fields are still set correctly
    assert_eq!(requirements.scheme, "exact");
    assert!(!requirements.asset.is_empty());
    assert!(!requirements.pay_to.is_empty());
}

#[test]
fn test_create_requirements_very_small_amount() {
    let mut config = TestConfig::new();
    config.amount = Some(Decimal::from_str("0.000001").unwrap());

    let requirements = create_requirements_test(&config, "/test").unwrap();
    // 0.000001 USDC = 1 in smallest units (6 decimals)
    assert_eq!(
        requirements.max_amount_required, "1",
        "0.000001 USDC must equal exactly 1 in smallest units"
    );

    // Verify precision is maintained
    let amount_back =
        Decimal::from_str(&requirements.max_amount_required).unwrap() / Decimal::from(1_000_000u64);
    assert_eq!(
        amount_back,
        Decimal::from_str("0.000001").unwrap(),
        "Very small amount conversion must maintain precision"
    );
}

#[test]
fn test_create_requirements_very_large_amount() {
    let mut config = TestConfig::new();
    config.amount = Some(Decimal::from_str("1000000.0").unwrap());

    let requirements = create_requirements_test(&config, "/test").unwrap();
    // 1,000,000 USDC = 1,000,000,000,000 in smallest units
    assert_eq!(
        requirements.max_amount_required, "1000000000000",
        "1,000,000 USDC must equal exactly 1,000,000,000,000 in smallest units"
    );

    // Verify it's a valid large number and can be parsed
    let max_amount: u64 = requirements
        .max_amount_required
        .parse()
        .expect("max_amount_required must be a valid u64 number");
    assert_eq!(
        max_amount, 1_000_000_000_000u64,
        "Parsed amount must match expected value"
    );

    // Verify conversion is reversible
    let amount_back = Decimal::from(max_amount) / Decimal::from(1_000_000u64);
    assert_eq!(
        amount_back,
        Decimal::from_str("1000000.0").unwrap(),
        "Large amount conversion must be reversible"
    );
}

#[test]
fn test_create_requirements_fractional_amount() {
    let mut config = TestConfig::new();
    config.amount = Some(Decimal::from_str("0.123456").unwrap());

    let requirements = create_requirements_test(&config, "/test").unwrap();
    // 0.123456 USDC = 123456 in smallest units (6 decimals)
    assert_eq!(
        requirements.max_amount_required, "123456",
        "0.123456 USDC must equal exactly 123456 in smallest units"
    );

    // Verify fractional precision is maintained
    let amount_back =
        Decimal::from_str(&requirements.max_amount_required).unwrap() / Decimal::from(1_000_000u64);
    assert_eq!(
        amount_back,
        Decimal::from_str("0.123456").unwrap(),
        "Fractional amount conversion must maintain all 6 decimal places"
    );
}
