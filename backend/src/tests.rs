use super::*;
use std::net::IpAddr;

#[test]
fn test_hash_pin() {
    let hashed = utils::hash_pin("1234");
    assert_ne!(hashed, "1234");
    assert_eq!(hashed.len(), 64);
    assert_eq!(hashed, utils::hash_pin("1234"));
    assert_ne!(hashed, utils::hash_pin("5678"));
}

#[test]
fn test_secure_compare() {
    assert!(utils::secure_compare("abcd", "abcd"));
    assert!(!utils::secure_compare("abcd", "abce"));
    assert!(!utils::secure_compare("abcd", "abcde"));
    assert!(!utils::secure_compare("abcde", "abcd"));
}

#[test]
fn test_fuzzy_match_subsequence() {
    let score_exact = search::fuzzy_match_subsequence("log", "log");
    assert!(score_exact.is_some());

    let score_sub = search::fuzzy_match_subsequence("log", "rpd");
    assert!(score_sub.is_some());

    let score_none = search::fuzzy_match_subsequence("log", "xyz");
    assert!(score_none.is_none());
}

#[test]
fn test_parse_trusted_proxies() {
    let raw = "127.0.0.1, 10.0.0.0/8 # private range";
    let list = utils::parse_trusted_proxies(raw);
    assert_eq!(list.len(), 2);
    assert_eq!(list[0].addr(), "127.0.0.1".parse::<IpAddr>().unwrap());
    assert_eq!(list[1].prefix_len(), 8);
}

#[test]
fn test_is_trusted_proxy() {
    let list = utils::parse_trusted_proxies("10.0.0.0/8");
    assert!(utils::is_trusted_proxy("10.0.0.1".parse().unwrap(), &list));
    assert!(!utils::is_trusted_proxy(
        "192.168.1.1".parse().unwrap(),
        &list
    ));
}
