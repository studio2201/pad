use super::*;

#[test]
fn test_fuzzy_match_subsequence() {
    let score_exact = search::fuzzy_match_subsequence("log", "log");
    assert!(score_exact.is_some());

    let score_sub = search::fuzzy_match_subsequence("rapids", "rpd");
    assert!(score_sub.is_some());

    let score_none = search::fuzzy_match_subsequence("log", "xyz");
    assert!(score_none.is_none());
}

#[test]
fn test_origin_allowed_in_development() {
    // Development is the documented convenience: any origin works.
    assert!(ws::is_origin_allowed(
        Some("https://evil.example"),
        "http://localhost:4402",
        "development"
    ));
    assert!(ws::is_origin_allowed(
        None,
        "http://localhost:4402",
        "development"
    ));
}

#[test]
fn test_origin_allowed_in_production_matches_base() {
    assert!(ws::is_origin_allowed(
        Some("https://pad.example.com"),
        "https://pad.example.com",
        "production"
    ));
    // Trailing slash on either side should not matter.
    assert!(ws::is_origin_allowed(
        Some("https://pad.example.com/"),
        "https://pad.example.com",
        "production"
    ));
    assert!(ws::is_origin_allowed(
        Some("https://pad.example.com"),
        "https://pad.example.com/",
        "production"
    ));
}

#[test]
fn test_origin_rejected_in_production() {
    // No Origin header in production is rejected (browsers always send one).
    assert!(!ws::is_origin_allowed(
        None,
        "https://pad.example.com",
        "production"
    ));

    // Mismatched origin is rejected.
    assert!(!ws::is_origin_allowed(
        Some("https://evil.example"),
        "https://pad.example.com",
        "production"
    ));
    assert!(!ws::is_origin_allowed(
        Some("http://pad.example.com"),
        "https://pad.example.com",
        "production"
    ));
}

#[test]
fn test_origin_does_not_honor_wildcard_base_in_production() {
    // The previous implementation had a `base_url == "*"` shortcut that
    // disabled the check. That shortcut is gone — a wildcard base in
    // production now rejects everything (forcing the operator to either
    // set the real base URL or run with NODE_ENV=development).
    assert!(!ws::is_origin_allowed(
        Some("https://anything"),
        "*",
        "production"
    ));
    assert!(!ws::is_origin_allowed(
        Some("https://anything"),
        "*",
        "production"
    ));
}

#[test]
fn test_sanitize_filename_rules() {
    use crate::migration::sanitize_filename;

    assert_eq!(sanitize_filename("hello").unwrap(), "hello");
    assert_eq!(sanitize_filename("my notepad").unwrap(), "my notepad");
    assert_eq!(sanitize_filename("a/b\\c").unwrap(), "a_b_c");

    // Path-traversal-class inputs are rejected.
    assert!(sanitize_filename("..").is_err());
    assert!(sanitize_filename(".").is_err());
    assert!(sanitize_filename("....").is_err());
    assert!(sanitize_filename(".hidden").is_err());

    // Windows-reserved names are rejected.
    assert!(sanitize_filename("CON").is_err());
    assert!(sanitize_filename("nul").is_err());

    // Empty / whitespace-only is rejected.
    assert!(sanitize_filename("").is_err());
    assert!(sanitize_filename("   ").is_err());
    assert!(sanitize_filename("///").is_err());
}
