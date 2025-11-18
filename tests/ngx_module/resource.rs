//! Resource path tests

use super::common::{create_requirements_test, TestConfig};

#[test]
fn test_create_requirements_long_resource_path() {
    let mut config = TestConfig::new();
    let long_path = "/".to_string() + &"a".repeat(1000);
    config.resource = Some(long_path.clone());

    let requirements = create_requirements_test(&config, "/test").unwrap();
    assert_eq!(requirements.resource, long_path);
}

#[test]
fn test_create_requirements_special_characters_in_resource() {
    let mut config = TestConfig::new();
    config.resource = Some("/api/v1/payment?amount=0.0001&token=abc123".to_string());

    let requirements = create_requirements_test(&config, "/test").unwrap();
    assert_eq!(
        requirements.resource,
        "/api/v1/payment?amount=0.0001&token=abc123"
    );
}

#[test]
fn test_create_requirements_unicode_in_description() {
    let mut config = TestConfig::new();
    config.description = Some("测试支付 🚀".to_string());

    let requirements = create_requirements_test(&config, "/test").unwrap();
    assert_eq!(requirements.description, "测试支付 🚀");
}
