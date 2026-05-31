use objc2_event_kit::EKEventStore;

#[allow(dead_code)]
pub fn eventkit_link_probe() {
    let _store = unsafe { EKEventStore::new() };
}
