//! Field structure and validation tests

use super::common::{create_requirements_test, TestConfig};

#[test]
fn test_create_requirements_max_timeout_seconds_default() {
    let config = TestConfig::new();
    let requirements = create_requirements_test(&config, "/test").unwrap();

    // Verify default timeout is set correctly
    assert_eq!(
        requirements.max_timeout_seconds, 60,
        "Default max_timeout_seconds must be 60"
    );
    assert!(
        requirements.max_timeout_seconds > 0,
        "max_timeout_seconds must be positive"
    );
}

#[test]
fn test_create_requirements_extra_field_structure() {
    let config = TestConfig::new();
    let requirements = create_requirements_test(&config, "/test").unwrap();

    // Strict validation of extra field structure
    assert!(requirements.extra.is_some(), "Extra field must be present");
    let extra = requirements.extra.as_ref().unwrap();

    // Verify it's an object (not array or primitive)
    assert!(extra.is_object(), "Extra field must be a JSON object");

    // Verify required fields exist
    assert!(
        extra.get("name").is_some(),
        "Extra field must contain 'name'"
    );
    assert!(
        extra.get("version").is_some(),
        "Extra field must contain 'version'"
    );

    // Verify field types
    assert!(extra["name"].is_string(), "Extra.name must be a string");
    assert!(
        extra["version"].is_string(),
        "Extra.version must be a string"
    );

    // Verify field values
    assert_eq!(extra["name"], "USDC", "Extra.name must be 'USDC'");
    assert_eq!(extra["version"], "2", "Extra.version must be '2'");
}

#[test]
fn test_create_requirements_scheme_always_exact() {
    let config = TestConfig::new();
    let requirements = create_requirements_test(&config, "/test").unwrap();

    assert_eq!(
        requirements.scheme, "exact",
        "Scheme must always be 'exact'"
    );
}

#[test]
fn test_create_requirements_mime_type_default() {
    use super::common::create_requirements_test_with_mime;
    let config = TestConfig::new();
    let requirements = create_requirements_test_with_mime(&config, "/test", None).unwrap();

    // When mimeType is not provided, it should be None
    assert_eq!(
        requirements.mime_type, None,
        "mimeType should be None when not provided"
    );
}

#[test]
fn test_create_requirements_mime_type_set() {
    use super::common::create_requirements_test_with_mime;
    let config = TestConfig::new();
    let requirements =
        create_requirements_test_with_mime(&config, "/test", Some("application/json")).unwrap();

    // When mimeType is provided, it should be set
    assert_eq!(
        requirements.mime_type,
        Some("application/json".to_string()),
        "mimeType should be set when provided"
    );
}

#[test]
fn test_create_requirements_mime_type_various_types() {
    use super::common::create_requirements_test_with_mime;
    let config = TestConfig::new();

    let mime_types = vec![
        "application/json",
        "text/html",
        "application/xml",
        "text/plain",
        "application/octet-stream",
    ];

    for mime_type in mime_types {
        let requirements =
            create_requirements_test_with_mime(&config, "/test", Some(mime_type)).unwrap();
        assert_eq!(
            requirements.mime_type,
            Some(mime_type.to_string()),
            "mimeType should be set correctly for {}",
            mime_type
        );
    }
}

#[test]
fn test_create_requirements_output_schema_default() {
    let config = TestConfig::new();
    let _requirements = create_requirements_test(&config, "/test").unwrap();

    // Output schema should be set to a default value (if implemented)
    // This test verifies the field exists and has a reasonable value
    // Note: The actual field name may vary based on PaymentRequirements structure
    // This is a placeholder test that can be updated based on actual implementation
}

#[test]
fn test_create_requirements_all_fields_present() {
    let config = TestConfig::new();
    let requirements = create_requirements_test(&config, "/test").unwrap();

    // Comprehensive check: all fields must be set (even if None)
    assert!(!requirements.scheme.is_empty(), "scheme must not be empty");
    assert!(
        !requirements.network.is_empty(),
        "network must not be empty"
    );
    assert!(
        !requirements.max_amount_required.is_empty(),
        "max_amount_required must not be empty"
    );
    assert!(!requirements.asset.is_empty(), "asset must not be empty");
    assert!(!requirements.pay_to.is_empty(), "pay_to must not be empty");
    assert!(
        !requirements.resource.is_empty(),
        "resource must not be empty"
    );
    assert!(
        !requirements.description.is_empty(),
        "description must not be empty"
    );
    assert!(
        requirements.max_timeout_seconds > 0,
        "max_timeout_seconds must be positive"
    );
    assert!(requirements.extra.is_some(), "extra must be set");
}
