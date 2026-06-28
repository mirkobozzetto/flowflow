mod account_join;
mod codec;
mod host;
mod identity;
mod join;
mod lan;
mod peer_store;

use std::time::Duration;

pub use codec::{
    decode_pairing_uri, encode_pairing_uri, generate_psk, new_pairing_payload,
    pairing_qr_svg, parse_manual_addr, PairingPayload, PAIRING_SCHEME,
};
pub use host::{start_pairing_host, PairingHost, PairingStatus};
pub use identity::{
    ensure_sync_identity, SyncIdentity, STATIC_PRIVKEY_KEY, STATIC_PUBKEY_KEY,
};
pub use join::join_pairing;
pub use peer_store::{
    authorize_rebind, load_peer_psk, unpair, REBIND_REFUSED_MARKER,
};

pub const PSK_LEN: usize = 32;
pub const PAIRING_WINDOW: Duration = Duration::from_secs(300);
