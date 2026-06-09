use super::apply::apply_batch;
use super::collect::collect_batch;
use super::wire::{recv_msg, send_msg, Msg, SyncHint, SyncRow, SyncStats};
use super::{log, proto_err, sql_err, PROTOCOL_VERSION};
use crate::db::sync_meta::DEVICE_ID_KEY;
use crate::db::Database;
use crate::services::sync::transport::{self, SecureChannel};
use crate::services::sync::{peers, SyncError};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use std::io::{Read, Write};
use std::net::TcpListener;

fn my_device_id(db: &Database) -> Result<String, SyncError> {
    db.get_setting(DEVICE_ID_KEY)
        .filter(|v| !v.is_empty())
        .ok_or_else(|| proto_err("device_id missing"))
}

fn my_hello(db: &Database, peer_device: &str) -> Result<Msg, SyncError> {
    let device_id = my_device_id(db)?;
    let peer = db
        .get_peer(peer_device)
        .map_err(SyncError::Protocol)?
        .ok_or_else(|| proto_err(format!("unknown peer {peer_device}")))?;
    Ok(Msg::Hello {
        protocol_version: PROTOCOL_VERSION,
        device_id,
        last_acked_seq: peer.last_acked_seq,
        gc_horizon: peer.gc_horizon,
    })
}

struct PeerHello {
    protocol_version: i64,
    device_id: String,
    last_acked_seq: i64,
    gc_horizon: i64,
}

fn expect_hello(msg: Msg) -> Result<PeerHello, SyncError> {
    match msg {
        Msg::Hello {
            protocol_version,
            device_id,
            last_acked_seq,
            gc_horizon,
        } => Ok(PeerHello {
            protocol_version,
            device_id,
            last_acked_seq,
            gc_horizon,
        }),
        _ => Err(proto_err("expected HELLO")),
    }
}

// Session-level guards run on BOTH sides right after the HELLO exchange.
// Each one converts a silent-loss scenario into a visible, recoverable error:
// - version skew: a peer speaking another protocol version could ship entity
//   kinds we would consume-and-drop; refuse the session instead.
// - gc horizon: a peer that GC'd tombstones beyond our watermark would skip
//   deletions; needs the T19 full-state reconcile (gc stays 0 until then).
// - restore detection: if the peer has acked MORE of our seqs than our
//   counter has issued, this device was restored from an older backup; its
//   new writes would land below the peer watermark and never sync.
fn check_session(
    db: &Database,
    my_device: &str,
    peer_device: &str,
    hello: &PeerHello,
) -> Result<(), SyncError> {
    if hello.protocol_version != PROTOCOL_VERSION {
        return Err(proto_err(format!(
            "protocol version mismatch (peer {}, ours {PROTOCOL_VERSION}): \
             update the app on both devices",
            hello.protocol_version
        )));
    }
    let peer = db
        .get_peer(peer_device)
        .map_err(SyncError::Protocol)?
        .ok_or_else(|| proto_err(format!("unknown peer {peer_device}")))?;
    if peer.last_acked_seq < hello.gc_horizon {
        return Err(proto_err(
            "peer gc_horizon beyond our watermark: full-state reconcile \
             required (T19, not yet implemented)",
        ));
    }
    let my_next_seq: i64 = db
        .conn()
        .query_row(
            "SELECT next_seq FROM sync_seq WHERE device_id = ?1",
            [my_device],
            |r| r.get(0),
        )
        .map_err(|e| sql_err("read own seq", e))?;
    if hello.last_acked_seq > my_next_seq {
        return Err(proto_err(format!(
            "peer acked our seq {} but our counter is at {my_next_seq}: \
             this device looks restored from an older backup; full-state \
             reconcile required (T19) to avoid silent divergence",
            hello.last_acked_seq
        )));
    }
    Ok(())
}

fn push_all<S: Read + Write>(
    chan: &mut SecureChannel<S>,
    db: &Database,
    my_device: &str,
    start_after: i64,
    batch_rows: usize,
    stats: &mut SyncStats,
) -> Result<(), SyncError> {
    let mut cursor = start_after;
    loop {
        let rows: Vec<SyncRow> =
            collect_batch(db, my_device, cursor, batch_rows)?;
        let done = rows.is_empty();
        if let Some(last) = rows.last() {
            cursor = last.origin_seq;
        }
        stats.pushed += rows.len();
        send_msg(chan, &Msg::Push { rows, done })?;
        match recv_msg(chan)? {
            Msg::Ack { .. } => {}
            _ => return Err(proto_err("expected ACK")),
        }
        if done {
            return Ok(());
        }
    }
}

fn recv_all<S: Read + Write>(
    chan: &mut SecureChannel<S>,
    db: &Database,
    my_device: &str,
    peer_device: &str,
    abort_after_batches: Option<usize>,
    stats: &mut SyncStats,
) -> Result<(), SyncError> {
    let mut batches = 0usize;
    loop {
        let msg = recv_msg(chan)?;
        let Msg::Push { rows, done } = msg else {
            return Err(proto_err("expected PUSH"));
        };
        let watermark = apply_batch(db, my_device, peer_device, &rows, stats)?;
        batches += 1;
        if let Some(limit) = abort_after_batches {
            if batches >= limit && !done {
                return Err(proto_err("aborted after batch limit (test seam)"));
            }
        }
        send_msg(
            chan,
            &Msg::Ack {
                upto_seq: watermark,
            },
        )?;
        if done {
            return Ok(());
        }
    }
}

fn load_peer_credentials(
    db: &Database,
    peer_device: &str,
) -> Result<(Vec<u8>, [u8; peers::PSK_LEN]), SyncError> {
    let peer = db
        .get_peer(peer_device)
        .map_err(SyncError::Protocol)?
        .ok_or_else(|| proto_err(format!("unknown peer {peer_device}")))?;
    let peer_static = URL_SAFE_NO_PAD
        .decode(&peer.static_pubkey)
        .map_err(|e| proto_err(format!("decode peer static key: {e}")))?;
    let psk = peers::load_peer_psk(db, peer_device)?
        .ok_or_else(|| proto_err(format!("missing PSK for {peer_device}")))?;
    Ok((peer_static, psk))
}

// Initiator: connect to a paired peer and run one full bidirectional sync
// session (push mine, then receive theirs).
pub fn sync_with_peer(
    db: &Database,
    peer_device: &str,
    host: &str,
    port: u16,
    batch_rows: usize,
) -> Result<SyncStats, SyncError> {
    let identity = peers::ensure_sync_identity(db)?;
    if identity.device_id == peer_device {
        return Err(proto_err("cannot sync with self"));
    }
    let (peer_static, psk) = load_peer_credentials(db, peer_device)?;
    let mut stream = transport::connect_tcp(host, port)?;
    let hint = serde_json::to_vec(&SyncHint {
        kind: "sync_hint".into(),
        device_id: identity.device_id.clone(),
    })
    .map_err(|e| proto_err(format!("encode hint: {e}")))?;
    transport::write_frame(&mut stream, &hint)?;
    let mut chan = transport::initiator_handshake(
        stream,
        &identity.private_key,
        &psk,
        Some(&peer_static),
    )?;
    send_msg(&mut chan, &my_hello(db, peer_device)?)?;
    let hello = expect_hello(recv_msg(&mut chan)?)?;
    if hello.device_id != peer_device {
        return Err(proto_err("peer HELLO identity mismatch"));
    }
    check_session(db, &identity.device_id, peer_device, &hello)?;
    let mut stats = SyncStats::default();
    push_all(
        &mut chan,
        db,
        &identity.device_id,
        hello.last_acked_seq,
        batch_rows,
        &mut stats,
    )?;
    recv_all(
        &mut chan,
        db,
        &identity.device_id,
        peer_device,
        None,
        &mut stats,
    )?;
    log(&format!(
        "session done with {peer_device}: pushed {} applied {} \
         skipped {} conflicts {}",
        stats.pushed, stats.applied, stats.skipped, stats.conflicts
    ));
    Ok(stats)
}

// Responder: accept ONE connection on the listener and serve a full session
// (receive theirs, then push mine). abort_after_batches is a test seam that
// simulates an iOS suspension cutting the transfer mid-PUSH.
pub fn serve_sync_once(
    db: &Database,
    listener: &TcpListener,
    batch_rows: usize,
    abort_after_batches: Option<usize>,
) -> Result<SyncStats, SyncError> {
    let (stream, _) = listener
        .accept()
        .map_err(|e| SyncError::Transport(format!("accept: {e}")))?;
    transport::configure_stream(&stream)?;
    let mut stream = stream;
    let raw = transport::read_frame(&mut stream)?;
    let hint: SyncHint = serde_json::from_slice(&raw)
        .map_err(|e| proto_err(format!("bad sync hint: {e}")))?;
    if hint.kind != "sync_hint" {
        return Err(proto_err("expected sync_hint"));
    }
    let identity = peers::ensure_sync_identity(db)?;
    if hint.device_id == identity.device_id {
        return Err(proto_err("cannot sync with self"));
    }
    let (peer_static, psk) = load_peer_credentials(db, &hint.device_id)?;
    let mut chan = transport::responder_handshake(
        stream,
        &identity.private_key,
        &psk,
        Some(&peer_static),
    )?;
    let hello = expect_hello(recv_msg(&mut chan)?)?;
    if hello.device_id != hint.device_id {
        return Err(proto_err("peer HELLO does not match sync hint"));
    }
    check_session(db, &identity.device_id, &hint.device_id, &hello)?;
    send_msg(&mut chan, &my_hello(db, &hint.device_id)?)?;
    let mut stats = SyncStats::default();
    recv_all(
        &mut chan,
        db,
        &identity.device_id,
        &hint.device_id,
        abort_after_batches,
        &mut stats,
    )?;
    push_all(
        &mut chan,
        db,
        &identity.device_id,
        hello.last_acked_seq,
        batch_rows,
        &mut stats,
    )?;
    log(&format!(
        "served {}: pushed {} applied {} skipped {} conflicts {}",
        hint.device_id,
        stats.pushed,
        stats.applied,
        stats.skipped,
        stats.conflicts
    ));
    Ok(stats)
}
