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
    // Development is an explicit opt-in (NODE_ENV=development): any origin works.
    // The process default is production so containers stay fail-closed.
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

#[test]
fn test_property_sanitize_filename_never_panics() {
    use crate::migration::sanitize_filename;

    let long_str = "a".repeat(1000);
    let malformed_inputs = vec![
        "\0",
        "\x00\x01\x02",
        "../../../../etc/passwd",
        "C:\\Windows\\System32\\cmd.exe",
        "CON.txt",
        "AUX",
        "PRN.tar.gz",
        "COM1",
        "   ..   ",
        "🔥🦀🎉",
        long_str.as_str(),
        "../../..//..//",
        "hello\0world",
    ];

    for input in malformed_inputs {
        let res = sanitize_filename(input);
        if let Ok(clean) = res {
            assert!(!clean.contains('/'));
            assert!(!clean.contains('\\'));
            assert!(!clean.contains('\0'));
            assert!(!clean.starts_with('.'));
        }
    }
}

#[test]
fn test_property_origin_allowed_never_panics() {
    let origins = vec![
        None,
        Some(""),
        Some("http://localhost"),
        Some("https://pad.example.com"),
        Some("javascript:alert(1)"),
        Some("data:text/html,test"),
        Some("\0"),
    ];

    let bases = vec!["", "http://localhost:4402", "https://pad.example.com", "*"];
    let envs = vec!["production", "development", "staging", ""];

    for origin in &origins {
        for base in &bases {
            for env in &envs {
                let _ = ws::is_origin_allowed(*origin, base, env);
            }
        }
    }
}
