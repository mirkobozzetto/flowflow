// Owns WHEN a sync runs - a foreground
// listener serving inbound peers, an outbound pass at app open, the "Sync
// now" button, and a debounced pass after each save - plus the shared
// activity state the UI indicator polls. The protocol itself (T17) stays
// untouched: this module only orchestrates it.

use super::peers::parse_manual_addr;
use super::protocol::{
    self, serve_sync_once, sync_with_peer, SyncStats, DEFAULT_BATCH_ROWS,
};
use super::reconcile::spawn_reconcile_pass;
use crate::infrastructure::persistence::Database;
use std::net::TcpListener;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock, Weak};
use std::time::Duration;

// One well-known port per device, so a peer is reachable from just its IP.
// Overridable for tests / multiple instances on one machine.
pub const DEFAULT_SYNC_PORT: u16 = 53127;
const SAVE_DEBOUNCE: Duration = Duration::from_secs(3);
pub const PEER_ADDR_KEY_PREFIX: &str = "sync_peer_addr_";

pub fn sync_port() -> u16 {
    std::env::var("FLOWFLOW_SYNC_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_SYNC_PORT)
}

fn peer_addr_key(device_id: &str) -> String {
    format!("{PEER_ADDR_KEY_PREFIX}{device_id}")
}

// Last known "host:port" for a paired peer. Seeded at pairing time, refreshed
// whenever the peer dials in (DHCP moves devices around). Plain settings
// rows: no schema change, wiped with the pairing.
pub fn set_peer_endpoint(
    db: &Database,
    device_id: &str,
    host: &str,
    port: u16,
) -> Result<(), String> {
    db.set_setting(&peer_addr_key(device_id), &format!("{host}:{port}"))
}

pub fn peer_endpoint(db: &Database, device_id: &str) -> Option<(String, u16)> {
    let raw = db
        .get_setting(&peer_addr_key(device_id))
        .filter(|v| !v.is_empty())?;
    parse_manual_addr(&raw).ok()
}

pub fn clear_peer_endpoint(db: &Database, device_id: &str) {
    let _ = db.set_setting(&peer_addr_key(device_id), "");
}

// What the UI indicator shows. Done/Error carry a local wall-clock label so
// the user sees WHEN the last sync happened, and Error keeps the message
// visible: a sync that cannot progress (unreachable peer, poison row,
// version skew) must be a visible stall, never a silent one. Done.partial
// surfaces a peer that failed while another succeeded - a permanently
// failing peer must not hide behind a green Done.
#[derive(Debug, Clone, PartialEq)]
pub enum SyncActivity {
    Idle,
    Syncing,
    Done {
        at: String,
        pushed: usize,
        applied: usize,
        conflicts: usize,
        partial: Option<String>,
    },
    Error {
        at: String,
        message: String,
    },
}

fn local_clock() -> String {
    chrono::Local::now().format("%H:%M").to_string()
}

// The app's single live engine, registered at start so non-UI data writers
// (the embed threads bumping owner meta after fresh BLOBs) can request a
// debounced push without holding an engine handle. Weak: an engine dropped
// by a test never keeps its threads' world alive through the global.
static LIVE_ENGINE: OnceLock<Mutex<Weak<SyncEngine>>> = OnceLock::new();

fn register_engine(engine: &Arc<SyncEngine>) {
    let slot = LIVE_ENGINE.get_or_init(|| Mutex::new(Weak::new()));
    *slot.lock().unwrap() = Arc::downgrade(engine);
}

// Called after any local data change made OUTSIDE the UI flow (embed thread
// persisting vectors, conflict restore): without it, a meta bump that
// happens after the save-triggered push would strand the new rows until an
// unrelated trigger fires.
pub fn poke_after_data_change() {
    let Some(slot) = LIVE_ENGINE.get() else {
        return;
    };
    if let Some(engine) = slot.lock().unwrap().upgrade() {
        engine.schedule_debounced();
    }
}

// Called when the app returns to the foreground. After a Wi-Fi/DHCP change the
// stored peer endpoint can be stale; only an inbound contact refreshes it. An
// outbound pass on resume lets the device that moved re-reach the peer whose
// address is still valid, and the peer's serve_loop then heals the stale record
// in both directions.
pub fn sync_now_if_live() {
    let Some(slot) = LIVE_ENGINE.get() else {
        return;
    };
    if let Some(engine) = slot.lock().unwrap().upgrade() {
        // Foreground resume is when a Wi-Fi change surfaces: re-announce so
        // peers with a stale address for us heal even if our own dial fails.
        super::beacon::burst(
            &engine.db,
            engine.listen_port.unwrap_or_else(sync_port),
        );
        engine.sync_now();
    }
}

pub struct SyncEngine {
    db: Arc<Database>,
    activity: Arc<Mutex<SyncActivity>>,
    debounce_gen: Arc<AtomicU64>,
    session_lock: Arc<Mutex<()>>,
    // A sync request that lands while a pass is running must not be lost:
    // the running pass re-runs once more if this was set after its snapshot.
    pending: Arc<AtomicBool>,
    listen_port: Option<u16>,
    data_version: Arc<AtomicU64>,
}

impl SyncEngine {
    // Bind the listener synchronously (a port clash surfaces immediately in
    // the activity state), then serve inbound sessions on a worker thread
    // and kick one outbound pass (the "sync at app open" trigger).
    pub fn start(db: Arc<Database>) -> Arc<SyncEngine> {
        let engine = Self::start_listener(db, sync_port());
        engine.sync_now();
        engine
    }

    pub fn start_listener(db: Arc<Database>, port: u16) -> Arc<SyncEngine> {
        let activity = Arc::new(Mutex::new(SyncActivity::Idle));
        let listener = match TcpListener::bind(("0.0.0.0", port)) {
            Ok(l) => Some(l),
            Err(e) => {
                *activity.lock().unwrap() = SyncActivity::Error {
                    at: crate::infrastructure::persistence::now_iso(),
                    message: format!("listen on {port}: {e}"),
                };
                eprintln!("[sync] listener bind {port}: {e}");
                None
            }
        };
        let listen_port = listener
            .as_ref()
            .and_then(|l| l.local_addr().ok())
            .map(|a| a.port());
        let engine = Arc::new(SyncEngine {
            db,
            activity,
            debounce_gen: Arc::new(AtomicU64::new(0)),
            session_lock: Arc::new(Mutex::new(())),
            pending: Arc::new(AtomicBool::new(false)),
            listen_port,
            data_version: Arc::new(AtomicU64::new(0)),
        });
        register_engine(&engine);
        if let Some(listener) = listener {
            let serve = engine.clone();
            std::thread::spawn(move || serve.serve_loop(listener));
        }
        // LAN beacon: hear peers that moved networks, and announce ourselves
        // so a peer whose address book went stale can find us back.
        super::beacon::spawn_listener(engine.db.clone());
        super::beacon::burst(
            &engine.db,
            engine.listen_port.unwrap_or_else(sync_port),
        );
        engine
    }

    pub fn listen_port(&self) -> Option<u16> {
        self.listen_port
    }

    pub fn activity(&self) -> SyncActivity {
        self.activity.lock().unwrap().clone()
    }

    fn set_activity(&self, a: SyncActivity) {
        *self.activity.lock().unwrap() = a;
    }

    pub fn data_version(&self) -> u64 {
        self.data_version.load(Ordering::SeqCst)
    }

    fn bump_data_version(&self) {
        self.data_version.fetch_add(1, Ordering::SeqCst);
    }

    fn serve_loop(self: Arc<Self>, mut listener: TcpListener) {
        let mut accept_failures = 0u32;
        loop {
            if crate::application::backup::restore_lock_active() {
                eprintln!("[sync] restore pending: inbound listener stopped");
                return;
            }
            match serve_sync_once(&self.db, &listener, DEFAULT_BATCH_ROWS, None)
            {
                Ok(outcome) => {
                    accept_failures = 0;
                    // Keep the port we already know for this peer (its
                    // listener port, not this connection's ephemeral one).
                    let port = peer_endpoint(&self.db, &outcome.peer_device)
                        .map(|(_, p)| p)
                        .unwrap_or_else(sync_port);
                    let _ = set_peer_endpoint(
                        &self.db,
                        &outcome.peer_device,
                        &outcome.peer_ip,
                        port,
                    );
                    self.set_activity(SyncActivity::Done {
                        at: local_clock(),
                        pushed: outcome.stats.pushed,
                        applied: outcome.stats.applied,
                        conflicts: outcome.stats.conflicts,
                        partial: None,
                    });
                    if outcome.stats.applied > 0 {
                        self.bump_data_version();
                        spawn_reconcile_pass();
                    }
                    run_tombstone_gc(&self.db);
                }
                // Listener-level failure (serve_sync_once tags accept errors):
                // iOS marks sockets defunct across suspensions, so a dead fd
                // would otherwise loop on error forever and clobber the last
                // useful Done. Log without touching the activity, back off,
                // and re-bind the well-known port to recover inbound sync.
                Err(e) if is_accept_error(&e) => {
                    accept_failures += 1;
                    eprintln!("[sync] accept ({accept_failures}): {e}");
                    std::thread::sleep(Duration::from_millis(1000));
                    if accept_failures >= 3 {
                        if let Some(port) = self.listen_port {
                            match TcpListener::bind(("0.0.0.0", port)) {
                                Ok(fresh) => {
                                    eprintln!(
                                        "[sync] listener re-bound on {port}"
                                    );
                                    listener = fresh;
                                    accept_failures = 0;
                                }
                                Err(e) => {
                                    eprintln!("[sync] re-bind {port}: {e}")
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    accept_failures = 0;
                    self.set_activity(SyncActivity::Error {
                        at: local_clock(),
                        message: e.to_string(),
                    });
                    eprintln!("[sync] serve session: {e}");
                    // A refused peer or a torn session must not melt the CPU.
                    std::thread::sleep(Duration::from_millis(1000));
                }
            }
        }
    }

    // "Sync now" / app open: one outbound pass on a worker thread.
    pub fn sync_now(self: &Arc<Self>) {
        let engine = self.clone();
        std::thread::spawn(move || engine.sync_now_blocking());
    }

    // Debounced save trigger: every save arms a fresh generation; only the
    // last one still standing after the quiet period actually syncs.
    pub fn schedule_debounced(self: &Arc<Self>) {
        if crate::application::backup::restore_lock_active() {
            return;
        }
        let gen = self.debounce_gen.fetch_add(1, Ordering::SeqCst) + 1;
        let engine = self.clone();
        std::thread::spawn(move || {
            std::thread::sleep(SAVE_DEBOUNCE);
            if engine.debounce_gen.load(Ordering::SeqCst) == gen {
                engine.sync_now_blocking();
            }
        });
    }

    // One outbound pass over every paired peer. Concurrent requests collapse
    // into the running pass WITHOUT being lost: every request raises
    // `pending` first, and the lock holder keeps re-running until no request
    // arrived after its last snapshot - a save committed mid-pass is always
    // covered by a pass that started after it.
    pub fn sync_now_blocking(self: &Arc<Self>) {
        if crate::application::backup::restore_lock_active() {
            eprintln!("[sync] restore pending: outbound pass skipped");
            return;
        }
        self.pending.store(true, Ordering::SeqCst);
        let Ok(_guard) = self.session_lock.try_lock() else {
            return;
        };
        while self.pending.swap(false, Ordering::SeqCst) {
            self.run_pass();
        }
    }

    // Wrapped in an iOS background task so a backgrounding mid-session gets a
    // grace window to finish; if the window expires the transfer is resumable
    // by design (T17).
    fn run_pass(&self) {
        let peers = match self.db.list_peers() {
            Ok(p) => p,
            Err(e) => {
                self.set_activity(SyncActivity::Error {
                    at: local_clock(),
                    message: e,
                });
                return;
            }
        };
        if peers.is_empty() {
            return;
        }
        #[cfg(target_os = "ios")]
        let _bg = crate::infrastructure::platform::ios::sync_ffi::BackgroundTaskGuard::begin();
        self.set_activity(SyncActivity::Syncing);
        let mut total = SyncStats::default();
        let mut synced = 0usize;
        let mut first_error: Option<String> = None;
        for peer in &peers {
            let Some((host, port)) = peer_endpoint(&self.db, &peer.device_id)
            else {
                first_error.get_or_insert(format!(
                    "no known address for {}",
                    peer.device_id
                ));
                continue;
            };
            match sync_with_peer(
                &self.db,
                &peer.device_id,
                &host,
                port,
                protocol::DEFAULT_BATCH_ROWS,
            ) {
                Ok(stats) => {
                    synced += 1;
                    total.pushed += stats.pushed;
                    total.applied += stats.applied;
                    total.skipped += stats.skipped;
                    total.conflicts += stats.conflicts;
                }
                Err(e) => {
                    eprintln!("[sync] {}: {e}", peer.device_id);
                    first_error.get_or_insert(e.to_string());
                }
            }
        }
        if synced > 0 {
            self.set_activity(SyncActivity::Done {
                at: local_clock(),
                pushed: total.pushed,
                applied: total.applied,
                conflicts: total.conflicts,
                partial: first_error,
            });
            if total.applied > 0 {
                self.bump_data_version();
                spawn_reconcile_pass();
            }
            run_tombstone_gc(&self.db);
        } else if let Some(message) = first_error {
            // Nobody reachable: our address book may be stale in BOTH
            // directions. Announce ourselves so the peers dial back.
            super::beacon::burst(
                &self.db,
                self.listen_port.unwrap_or_else(sync_port),
            );
            self.set_activity(SyncActivity::Error {
                at: local_clock(),
                message,
            });
        }
    }
}

// serve_sync_once folds its accept() failure into SyncError::Transport with
// an "accept: " prefix; that is the listener-died signal the serve loop must
// treat differently from a failed session.
fn is_accept_error(e: &super::SyncError) -> bool {
    matches!(e, super::SyncError::Transport(m) if m.starts_with("accept: "))
}

// Opportunistic tombstone GC after every successful session: the session just
// refreshed the peer-ack book, so this is exactly when a tombstone can become
// provably consumed by everyone (T19). Failure is log-only - GC is hygiene,
// never worth failing a sync that already succeeded.
fn run_tombstone_gc(db: &Database) {
    if let Err(e) = super::gc::run_gc(db) {
        eprintln!("[sync] gc: {e}");
    }
}

// Pairing-time seed of the address book, called by peers.rs once a pairing
// persists: the joiner learns the host's LAN IP from the payload; the host
// learns the joiner's from the accepted socket. Both assume the well-known
// sync port (same build on both sides).
pub fn seed_peer_endpoint(db: &Database, device_id: &str, host: &str) {
    if let Err(e) = set_peer_endpoint(db, device_id, host, sync_port()) {
        eprintln!("[sync] seed endpoint for {device_id}: {e}");
    }
}
