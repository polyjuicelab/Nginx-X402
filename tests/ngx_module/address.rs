//! Address validation tests

use super::common::{create_requirements_test, TestConfig};

#[test]
fn test_create_requirements_long_pay_to_address() {
    let mut config = TestConfig::new();
    // Valid Ethereum address format (42 chars)
    config.pay_to = Some("0x1234567890123456789012345678901234567890".to_string());

    let requirements = create_requirements_test(&config, "/test").unwrap();
    assert_eq!(
        requirements.pay_to,
        "0x1234567890123456789012345678901234567890"
    );
}

#[test]
fn test_create_requirements_asset_address_validation() {
    let config = TestConfig::new();
    let requirements = create_requirements_test(&config, "/test").unwrap();

    // Strict validation of asset address format
    assert!(
        requirements.asset.starts_with("0x"),
        "Asset address must start with 0x"
    );
    assert_eq!(
        requirements.asset.len(),
        42,
        "Asset address must be exactly 42 characters (0x + 40 hex)"
    );

    // Verify all characters after 0x are valid hex
    let hex_part = &requirements.asset[2..];
    assert!(
        hex_part.chars().all(|c| c.is_ascii_hexdigit()),
        "Asset address must contain only hex characters after 0x"
    );
}

#[test]
fn test_create_requirements_pay_to_address_validation() {
    let config = TestConfig::new();
    let requirements = create_requirements_test(&config, "/test").unwrap();

    // Strict validation of pay_to address format
    assert!(
        requirements.pay_to.starts_with("0x"),
        "Pay-to address must start with 0x"
    );
    assert_eq!(
        requirements.pay_to.len(),
        42,
        "Pay-to address must be exactly 42 characters (0x + 40 hex)"
    );
    assert_eq!(
        requirements.pay_to,
        requirements.pay_to.to_lowercase(),
        "Pay-to address must be lowercase"
    );

    // Verify all characters after 0x are valid hex
    let hex_part = &requirements.pay_to[2..];
    assert!(
        hex_part.chars().all(|c| c.is_ascii_hexdigit()),
        "Pay-to address must contain only hex characters after 0x"
    );
}
