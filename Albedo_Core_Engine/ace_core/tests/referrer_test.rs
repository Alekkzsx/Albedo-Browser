use ace_core::security::{compute_referrer, Origin, ReferrerPolicy};

#[test]
fn test_referrer_policy_same_origin() {
    let origin = Origin::try_from_url("https://example.com/page.html").unwrap();
    let current_url = "https://example.com/page.html#heading";
    let same_target = "https://example.com/api/data";
    let cross_target = "https://other.com/api/data";

    let ref_same = compute_referrer(&origin, current_url, same_target, ReferrerPolicy::SameOrigin);
    assert_eq!(ref_same, Some("https://example.com/page.html".to_string()));

    let ref_cross = compute_referrer(&origin, current_url, cross_target, ReferrerPolicy::SameOrigin);
    assert_eq!(ref_cross, None);
}

#[test]
fn test_referrer_policy_downgrade_protection() {
    let origin = Origin::try_from_url("https://secure.com").unwrap();
    let current_url = "https://secure.com/checkout";
    let insecure_target = "http://insecure.com/tracker";

    let ref_downgrade = compute_referrer(
        &origin,
        current_url,
        insecure_target,
        ReferrerPolicy::StrictOriginWhenCrossOrigin,
    );
    assert_eq!(ref_downgrade, None); // Bloqueia vazamento em downgrade HTTPS -> HTTP
}

#[test]
fn test_referrer_policy_origin_only() {
    let origin = Origin::try_from_url("https://example.com").unwrap();
    let current_url = "https://example.com/private/path?secret=123";
    let cross_target = "https://cdn.example.org/image.png";

    let ref_origin = compute_referrer(
        &origin,
        current_url,
        cross_target,
        ReferrerPolicy::StrictOriginWhenCrossOrigin,
    );
    assert_eq!(ref_origin, Some("https://example.com".to_string()));
}
