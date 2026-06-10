use super::apply::{apply_batch, ApplyCtx};
use super::collect::{collect_batch, collect_full_batch};
use super::wire::{recv_msg, send_msg, Msg, SyncHint, SyncRow, SyncStats};
use super::{log, proto_err, sql_err, PROTOCOL_VERSION};
use crate::db::sync_meta::DEVICE_ID_KEY;
use crate::db::Database;
use crate::services::sync::transport::{self, SecureChannel};
use crate::services::sync::{gc, peers, SyncError};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use std::io::{Read, Write};
use std::net::TcpListener;

fn my_device_id(db: &Database) -> Result<String, SyncError> {
    db.get_setting(DEVICE_ID_KEY)
        .filter(|v| !v.is_empty())
        .ok_or_else(|| proto_err("device_id missing"))
}

fn my_next_seq(db: &Database, my_device: &str) -> Result<i64, SyncError> {
    db.conn()
        .query_row(
            "SELECT next_seq FROM sync_seq WHERE device_id = ?1",
            [my_device],
            |r| r.get(0),
        )
        .map_err(|e| sql_err("read own seq", e))
}

fn my_hello(db: &Database, peer_device: &str) -> Result<Msg, SyncError> {
    let device_id = my_device_id(db)?;
    let peer = db
        .get_peer(peer_device)
        .map_err(SyncError::Protocol)?
        .ok_or_else(|| proto_err(format!("unknown peer {peer_device}")))?;
    let next_seq = my_next_seq(db, &device_id)?;
    Ok(Msg::Hello {
        protocol_version: PROTOCOL_VERSION,
        device_id,
        last_acked_seq: peer.last_acked_seq,
        gc_horizon: peer.gc_horizon,
        next_seq,
    })
}

struct PeerHello {
    protocol_version: i64,
    device_id: String,
    last_acked_seq: i64,
    gc_horizon: i64,
    next_seq: i64,
}

fn expect_hello(msg: Msg) -> Result<PeerHello, SyncError> {
    match msg {
        Msg::Hello {
            protocol_version,
            device_id,
            last_acked_seq,
            gc_horizon,
            next_seq,
        } => Ok(PeerHello {
            protocol_version,
            device_id,
            last_acked_seq,
            gc_horizon,
            next_seq,
        }),
        _ => Err(proto_err("expected HELLO")),
    }
}

// Session-level decisions, taken on BOTH sides right after the HELLO
// exchange. Every input is symmetric (each side sees the other's HELLO plus
// its own state), so the two devices independently reach the SAME plan
// without negotiation.
//
// - version skew: a peer speaking another protocol version could ship entity
//   kinds we would consume-and-drop; refuse the session instead.
// - restored device: a side whose own counter is BELOW what the other acked
//   was restored from an older backup. Its counter is reseeded above the ack
//   (or its new writes would hide below the peer watermark forever) and the
//   session goes full-state.
// - GC horizon: a side whose ack regressed below the other's gc_horizon has
//   missed tombstones that no longer exist for incremental push; the session
//   goes full-state and the intact side re-authors the deletions
//   (missing-locally = deleted), archiving the resurrected content.
struct SessionPlan {
    full_state: bool,
    ctx: ApplyCtx,
}

fn plan_session(
    db: &Database,
    my_device: &str,
    peer_device: &str,
    hello: &PeerHello,
) -> Result<SessionPlan, SyncError> {
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
    // Freshest truth about how far the peer consumed MY space; feeds the
    // tombstone GC. Overwrites (a restored peer's ack legitimately regresses).
    gc::record_peer_ack(db, peer_device, hello.last_acked_seq)
        .map_err(SyncError::Protocol)?;
    let me_restored = hello.last_acked_seq > my_next_seq(db, my_device)?;
    let peer_restored = peer.last_acked_seq > hello.next_seq;
    let me_stale_gc = peer.last_acked_seq < hello.gc_horizon;
    let peer_stale_gc = hello.last_acked_seq < peer.gc_horizon;
    let me_broken = me_restored || me_stale_gc;
    let peer_broken = peer_restored || peer_stale_gc;
    if me_restored {
        reseed_after_restore(db, my_device, hello.last_acked_seq)?;
    }
    let full_state = me_broken || peer_broken;
    if full_state {
        log(&format!(
            "full-state session with {peer_device} (me: restored={me_restored} \
             missed_gc={me_stale_gc}, peer: restored={peer_restored} \
             missed_gc={peer_stale_gc})"
        ));
    }
    Ok(SessionPlan {
        full_state,
        ctx: ApplyCtx {
            full_state,
            authority: peer_broken && !me_broken,
            peer_acked_of_mine: hello.last_acked_seq,
        },
    })
}

// This device came back from an older backup: its seq counter is BELOW what
// the peer already acked, so everything it wrote since the restore (and will
// write next) sits under the peer's watermark = invisible to incremental
// push. Re-sequence every locally-authored meta row above the peer's ack
// (order preserved) and move the counter past them. The version vectors are
// untouched - seqs are enumeration, not causality - so the row-level merge
// stays exact; the rows simply become pushable again.
fn reseed_after_restore(
    db: &Database,
    my_device: &str,
    peer_acked: i64,
) -> Result<(), SyncError> {
    let conn = db.conn();
    let tx = conn
        .unchecked_transaction()
        .map_err(|e| sql_err("reseed tx", e))?;
    tx.execute(
        "WITH ranked AS (
            SELECT rowid AS rid,
                   ROW_NUMBER() OVER (ORDER BY origin_seq) AS rn
            FROM sync_row_meta WHERE origin_device = ?1
         )
         UPDATE sync_row_meta SET origin_seq = ?2 + ranked.rn
         FROM ranked WHERE sync_row_meta.rowid = ranked.rid",
        rusqlite::params![my_device, peer_acked],
    )
    .map_err(|e| sql_err("reseed reorder", e))?;
    let moved: i64 = tx
        .query_row(
            "SELECT COUNT(*) FROM sync_row_meta WHERE origin_device = ?1",
            [my_device],
            |r| r.get(0),
        )
        .map_err(|e| sql_err("reseed count", e))?;
    tx.execute(
        "UPDATE sync_seq SET next_seq = MAX(next_seq, ?2)
         WHERE device_id = ?1",
        rusqlite::params![my_device, peer_acked + moved],
    )
    .map_err(|e| sql_err("reseed counter", e))?;
    tx.commit().map_err(|e| sql_err("reseed commit", e))?;
    log(&format!(
        "restored device detected: re-sequenced {moved} rows above the \
         peer ack {peer_acked}"
    ));
    Ok(())
}

// Receive one ACK and persist what it proves: the peer has consumed my seq
// space up to upto_seq (feeds the tombstone GC).
fn expect_ack<S: Read + Write>(
    chan: &mut SecureChannel<S>,
    db: &Database,
    peer_device: &str,
) -> Result<(), SyncError> {
    match recv_msg(chan)? {
        Msg::Ack { upto_seq } => {
            gc::record_peer_ack(db, peer_device, upto_seq)
                .map_err(SyncError::Protocol)?;
            Ok(())
        }
        _ => Err(proto_err("expected ACK")),
    }
}

fn push_all<S: Read + Write>(
    chan: &mut SecureChannel<S>,
    db: &Database,
    my_device: &str,
    peer_device: &str,
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
        expect_ack(chan, db, peer_device)?;
        if done {
            return Ok(());
        }
    }
}

// Full-state push (T19): replay EVERY meta row (any origin) by rowid. The
// receiver's merge skips what it already has; what it lost comes back; what
// it resurrected gets re-deleted by the authority side. Resumable like the
// incremental push: a cut just replays already-skipped rows next time.
fn push_full<S: Read + Write>(
    chan: &mut SecureChannel<S>,
    db: &Database,
    peer_device: &str,
    batch_rows: usize,
    stats: &mut SyncStats,
) -> Result<(), SyncError> {
    let mut cursor = 0i64;
    loop {
        let (rows, last_rowid) = collect_full_batch(db, cursor, batch_rows)?;
        let done = rows.is_empty();
        cursor = last_rowid;
        stats.pushed += rows.len();
        send_msg(chan, &Msg::Push { rows, done })?;
        expect_ack(chan, db, peer_device)?;
        if done {
            return Ok(());
        }
    }
}

fn push_phase<S: Read + Write>(
    chan: &mut SecureChannel<S>,
    db: &Database,
    my_device: &str,
    peer_device: &str,
    plan: &SessionPlan,
    batch_rows: usize,
    stats: &mut SyncStats,
) -> Result<(), SyncError> {
    if plan.full_state {
        push_full(chan, db, peer_device, batch_rows, stats)
    } else {
        // The incremental cursor is the peer's ack of my space, snapshotted
        // from its HELLO into the plan.
        push_all(
            chan,
            db,
            my_device,
            peer_device,
            plan.ctx.peer_acked_of_mine,
            batch_rows,
            stats,
        )
    }
}

fn recv_all<S: Read + Write>(
    chan: &mut SecureChannel<S>,
    db: &Database,
    my_device: &str,
    peer_device: &str,
    ctx: &ApplyCtx,
    abort_after_batches: Option<usize>,
    stats: &mut SyncStats,
) -> Result<(), SyncError> {
    let mut batches = 0usize;
    loop {
        let msg = recv_msg(chan)?;
        let Msg::Push { rows, done } = msg else {
            return Err(proto_err("expected PUSH"));
        };
        let watermark =
            apply_batch(db, my_device, peer_device, ctx, &rows, stats)?;
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
    let plan = plan_session(db, &identity.device_id, peer_device, &hello)?;
    let mut stats = SyncStats::default();
    push_phase(
        &mut chan,
        db,
        &identity.device_id,
        peer_device,
        &plan,
        batch_rows,
        &mut stats,
    )?;
    recv_all(
        &mut chan,
        db,
        &identity.device_id,
        peer_device,
        &plan.ctx,
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

// What a served session learned about the caller: who it was and from which
// LAN address it dialed in. The engine uses the address to refresh its
// peer-address book (DHCP leases move devices around).
pub struct ServeOutcome {
    pub peer_device: String,
    pub peer_ip: String,
    pub stats: SyncStats,
}

// Responder: accept ONE connection on the listener and serve a full session
// (receive theirs, then push mine). abort_after_batches is a test seam that
// simulates an iOS suspension cutting the transfer mid-PUSH.
pub fn serve_sync_once(
    db: &Database,
    listener: &TcpListener,
    batch_rows: usize,
    abort_after_batches: Option<usize>,
) -> Result<ServeOutcome, SyncError> {
    let (stream, remote_addr) = listener
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
    let plan = plan_session(db, &identity.device_id, &hint.device_id, &hello)?;
    send_msg(&mut chan, &my_hello(db, &hint.device_id)?)?;
    let mut stats = SyncStats::default();
    recv_all(
        &mut chan,
        db,
        &identity.device_id,
        &hint.device_id,
        &plan.ctx,
        abort_after_batches,
        &mut stats,
    )?;
    push_phase(
        &mut chan,
        db,
        &identity.device_id,
        &hint.device_id,
        &plan,
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
    Ok(ServeOutcome {
        peer_device: hint.device_id,
        peer_ip: remote_addr.ip().to_string(),
        stats,
    })
}
