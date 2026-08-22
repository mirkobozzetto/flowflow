// Account-heal wire exchange (RFC 0026), run after the note phases when
// BOTH Hellos advertised heal=true. Symmetric and unconditional: each side
// always sends one JoinRequest and one JoinToken (fields None when idle),
// in a role-fixed order, so neither side ever blocks waiting for a frame
// the other will not send. Any failure here is logged and swallowed: the
// notes already synced, and the next session retries.

use super::wire::{recv_msg, send_msg, Msg};
use super::{log, proto_err};
use crate::application::account_heal;
use crate::infrastructure::persistence::Database;
use crate::infrastructure::sync::transport::SecureChannel;
use crate::infrastructure::sync::SyncError;
use std::io::{Read, Write};

pub(super) struct PeerHealInfo {
    pub premium: bool,
    pub backend_host: Option<String>,
    pub device_name: String,
}

fn my_request(db: &Database, peer: &PeerHealInfo, hash: &[u8]) -> Msg {
    let (pubkey, sig) = if account_heal::wants_join(
        db,
        peer.premium,
        peer.backend_host.as_deref(),
    ) {
        match account_heal::sign_join_request(db, hash) {
            Some((p, s)) => (Some(p), Some(s)),
            None => (None, None),
        }
    } else {
        (None, None)
    };
    Msg::JoinRequest { pubkey, sig }
}

fn token_for(
    db: &Database,
    request: Msg,
    hash: &[u8],
) -> Result<Msg, SyncError> {
    let Msg::JoinRequest { pubkey, sig } = request else {
        return Err(proto_err("expected JOIN_REQUEST"));
    };
    let token = match (pubkey, sig) {
        (Some(pubkey), Some(sig))
            if account_heal::verify_join_request(&pubkey, &sig, hash) =>
        {
            account_heal::mint_for(db, &pubkey)
        }
        (Some(_), _) => {
            log(
                "heal: join request with an invalid session signature, ignored",
            );
            None
        }
        _ => None,
    };
    Ok(Msg::JoinToken { token })
}

fn consume_token(
    db: &Database,
    msg: Msg,
    peer: &PeerHealInfo,
) -> Result<(), SyncError> {
    let Msg::JoinToken { token } = msg else {
        return Err(proto_err("expected JOIN_TOKEN"));
    };
    if let Some(token) = token {
        account_heal::redeem(db, &token, &peer.device_name);
    }
    Ok(())
}

// Role-fixed frame order over one serial channel:
//   initiator: send REQ, recv REQ, send TOKEN, recv TOKEN
//   responder: recv REQ, send REQ, recv TOKEN(answer), ... symmetric mirror
fn run<S: Read + Write>(
    chan: &mut SecureChannel<S>,
    db: &Database,
    peer: &PeerHealInfo,
    initiator: bool,
) -> Result<(), SyncError> {
    let hash = chan.handshake_hash().to_vec();
    if initiator {
        send_msg(chan, &my_request(db, peer, &hash))?;
        let their_request = recv_msg(chan)?;
        let answer = token_for(db, their_request, &hash)?;
        send_msg(chan, &answer)?;
        let their_token = recv_msg(chan)?;
        consume_token(db, their_token, peer)?;
    } else {
        let their_request = recv_msg(chan)?;
        send_msg(chan, &my_request(db, peer, &hash))?;
        let answer = token_for(db, their_request, &hash)?;
        send_msg(chan, &answer)?;
        let their_token = recv_msg(chan)?;
        consume_token(db, their_token, peer)?;
    }
    Ok(())
}

/// Best-effort wrapper: the sync session already succeeded; a heal failure
/// must never turn it into an error.
pub(super) fn exchange<S: Read + Write>(
    chan: &mut SecureChannel<S>,
    db: &Database,
    peer: &PeerHealInfo,
    initiator: bool,
) {
    if let Err(e) = run(chan, db, peer, initiator) {
        log(&format!("heal exchange: {e}"));
    }
}
