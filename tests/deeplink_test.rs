// Prefix-scoped deep-link mailbox: pairing and record polling must not
// swallow each other's URIs (single shared slot).

use flowflow::infrastructure::sync::deeplink;

#[test]
fn prefix_routing_keeps_consumers_apart() {
    deeplink::push("flowflow://record".to_string());
    // The pairing consumer must not see (nor consume) a record URI.
    assert!(deeplink::peek_matching("flowflow://pair").is_none());
    assert!(deeplink::take_matching("flowflow://pair").is_none());
    assert!(deeplink::peek().is_some());
    // The record consumer drains it.
    assert_eq!(
        deeplink::take_matching("flowflow://record").as_deref(),
        Some("flowflow://record")
    );
    assert!(deeplink::take_matching("flowflow://record").is_none());

    deeplink::push("flowflow://pair#abc".to_string());
    assert!(deeplink::take_matching("flowflow://record").is_none());
    assert_eq!(
        deeplink::take_matching("flowflow://pair").as_deref(),
        Some("flowflow://pair#abc")
    );
}
