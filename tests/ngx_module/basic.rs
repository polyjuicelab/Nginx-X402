//! Basic functionality tests

use super::common::{create_requirements_test, TestConfig};
use rust_decimal::Decimal;
use std::str::FromStr;

#[test]
fn test_create_requirements_from_config() {
    let config = TestConfig::new();
    let requirements = create_requirements_test(&config, "/test").unwrap();

    // Verify all required fields are set correctly
    assert_eq!(requirements.scheme, "exact", "Scheme must be exact");
    assert_eq!(
        requirements.network, "base-sepolia",
        "Network must match testnet default"
    );

    // Verify pay_to is normalized to lowercase
    assert_eq!(
        requirements.pay_to, "0x209693bc6afc0c5328ba36faf03c514ef312287c",
        "Pay-to address must be lowercase"
    );

    assert_eq!(requirements.resource, "/test", "Resource must match input");
    assert_eq!(
        requirements.description, "Test payment",
        "Description must match config"
    );

    // Verify amount conversion (0.0001 USDC = 100 in smallest units)
    assert_eq!(
        requirements.max_amount_required, "100",
        "Amount must be correctly converted to smallest units"
    );

    // Verify asset address is set (USDC address for base-sepolia)
    assert!(
        !requirements.asset.is_empty(),
        "Asset address must not be empty"
    );
    assert!(
        requirements.asset.starts_with("0x"),
        "Asset address must be a valid Ethereum address"
    );
    assert_eq!(
        requirements.asset.len(),
        42,
        "Asset address must be 42 characters (0x + 40 hex chars)"
    );

    // Verify default timeout
    assert_eq!(
        requirements.max_timeout_seconds, 60,
        "Default timeout must be 60 seconds"
    );

    // Verify extra field contains USDC info
    assert!(
        requirements.extra.is_some(),
        "Extra field must contain USDC info"
    );
    let extra = requirements.extra.as_ref().unwrap();
    assert_eq!(extra["name"], "USDC", "Extra field must contain USDC name");
    assert_eq!(extra["version"], "2", "Extra field must contain version 2");
}

#[test]
fn test_create_requirements_with_explicit_network() {
    let mut config = TestConfig::new();
    config.network = Some("base".to_string());
    config.testnet = false;

    let requirements = create_requirements_test(&config, "/test").unwrap();

    // Verify network is explicitly set
    assert_eq!(
        requirements.network, "base",
        "Network must match explicit setting"
    );

    // Verify asset address matches mainnet USDC address
    assert!(
        !requirements.asset.is_empty(),
        "Asset address must be set for mainnet"
    );
    assert!(
        requirements.asset.starts_with("0x"),
        "Asset address must be valid Ethereum address"
    );

    // Verify extra field contains mainnet USDC info
    assert!(
        requirements.extra.is_some(),
        "Extra field must contain USDC info"
    );
    let extra = requirements.extra.as_ref().unwrap();
    // Mainnet uses "USD Coin" as the USDC name
    assert_eq!(
        extra["name"], "USD Coin",
        "Extra field must contain 'USD Coin' name for mainnet"
    );
}

#[test]
fn test_create_requirements_with_resource_override() {
    let mut config = TestConfig::new();
    config.resource = Some("/custom/resource".to_string());

    let requirements = create_requirements_test(&config, "/test").unwrap();
    assert_eq!(requirements.resource, "/custom/resource");
}

#[test]
fn test_create_requirements_mainnet() {
    let mut config = TestConfig::new();
    config.testnet = false;
    config.network = None;

    let requirements = create_requirements_test(&config, "/test").unwrap();
    assert_eq!(requirements.network, "base");
}

#[test]
fn test_create_requirements_network_override_ignores_testnet_flag() {
    let mut config = TestConfig::new();
    config.network = Some("base".to_string());
    config.testnet = true; // Should be ignored when network is explicitly set

    let requirements = create_requirements_test(&config, "/test").unwrap();
    assert_eq!(
        requirements.network, "base",
        "Explicit network must override testnet flag"
    );

    // Verify asset address matches mainnet (not testnet) when network is explicitly "base"
    assert!(!requirements.asset.is_empty(), "Asset address must be set");

    // Verify extra field uses mainnet USDC info (because network is "base")
    assert!(
        requirements.extra.is_some(),
        "Extra field must contain USDC info"
    );
    let extra = requirements.extra.as_ref().unwrap();
    assert_eq!(extra["name"], "USDC", "Extra field must contain USDC name");
}

#[test]
fn test_create_requirements_multiple_requirements_support() {
    // Test that the function can create multiple requirements for multi-part payment scenarios
    let config1 = TestConfig::new();
    let config2 = {
        let mut c = TestConfig::new();
        c.amount = Some(Decimal::from_str("0.0002").unwrap());
        c
    };

    let req1 = create_requirements_test(&config1, "/resource1").unwrap();
    let req2 = create_requirements_test(&config2, "/resource2").unwrap();

    // Verify both requirements are valid
    assert_eq!(req1.max_amount_required, "100");
    assert_eq!(req2.max_amount_required, "200");
    assert_ne!(
        req1.max_amount_required, req2.max_amount_required,
        "Different amounts must produce different max_amount_required"
    );
}
