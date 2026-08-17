use ace_core::version::BuildInfo;

#[test]
fn test_build_info_and_user_agent() {
    let info = BuildInfo::current();
    assert_eq!(info.version, "0.1.0");
    assert!(!info.target_os.is_empty());
    assert!(!info.target_arch.is_empty());

    let ua = BuildInfo::default_user_agent("Albedo");
    assert!(ua.starts_with("Mozilla/5.0"));
    assert!(ua.contains("Albedo/0.1.0"));
    assert!(ua.contains("AppleWebKit/537.36"));
}
