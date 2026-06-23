// Sync screen (RFC 0004 T18/T20): manual trigger + activity indicator,
// unresolved conflicts, and device pairing. One concern per child module.

mod conflicts;
mod controls;
mod pairing;

use conflicts::SyncConflicts;
use controls::SyncControls;
use dioxus::prelude::*;
use pairing::SyncPairingView;

pub fn pending_pairing_uri_exists() -> bool {
    crate::infrastructure::sync::deeplink::peek().is_some()
}

#[component]
pub fn SyncView() -> Element {
    rsx! {
        div { class: "space-y-6 pb-20",
            SyncControls {}
            SyncConflicts {}
            SyncPairingView {}
        }
    }
}
