// LAN presence beacon: heals stale peer addresses after a network change.
// Both address books can go stale at once (both devices moved Wi-Fi), and
// the TCP path only refreshes an address after a session succeeds - a
// deadlock this beacon breaks. A burst announces {device_id, port} in UDP
// broadcast; a paired peer hearing it rewrites the sender's endpoint from
// the packet's source IP and dials back. Spoofing buys nothing: a fake
// beacon only makes us dial a host that cannot complete the Noise+PSK
// handshake; identity stays key-based, the beacon is a hint.

use super::engine::{
    peer_endpoint, set_peer_endpoint, sync_now_if_live, sync_port,
};
use crate::infrastructure::persistence::Database;
use serde::{Deserialize, Serialize};
use std::net::UdpSocket;
use std::sync::Arc;
use std::time::Duration;

const BURST_PACKETS: u32 = 3;
const BURST_SPACING: Duration = Duration::from_millis(200);

pub fn beacon_port() -> u16 {
    sync_port() + 1
}

#[derive(Serialize, Deserialize)]
struct Announce {
    device_id: String,
    port: u16,
}

fn own_device_id(db: &Database) -> Option<String> {
    db.get_setting("sync_device_id").filter(|v| !v.is_empty())
}

// Bind the well-known beacon port and serve it forever. Bind failure is
// log-only (another instance on this host already listens; sync still works
// through the TCP path).
pub fn spawn_listener(db: Arc<Database>) {
    let socket = match UdpSocket::bind(("0.0.0.0", beacon_port())) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[sync] beacon bind {}: {e}", beacon_port());
            return;
        }
    };
    std::thread::spawn(move || listen_on(socket, &db));
}

fn listen_on(socket: UdpSocket, db: &Database) {
    let mut buf = [0u8; 512];
    loop {
        let Ok((n, src)) = socket.recv_from(&mut buf) else {
            std::thread::sleep(Duration::from_millis(500));
            continue;
        };
        let Ok(ann) = serde_json::from_slice::<Announce>(&buf[..n]) else {
            continue;
        };
        if own_device_id(db).as_deref() == Some(ann.device_id.as_str()) {
            continue;
        }
        // Only a PAIRED peer may move its endpoint; strangers are ignored.
        match db.get_peer(&ann.device_id) {
            Ok(Some(_)) => {}
            _ => continue,
        }
        let host = src.ip().to_string();
        let known = peer_endpoint(db, &ann.device_id);
        if known.as_ref() == Some(&(host.clone(), ann.port)) {
            continue;
        }
        eprintln!(
            "[sync] beacon: {} moved to {host}:{}",
            ann.device_id, ann.port
        );
        let _ = set_peer_endpoint(db, &ann.device_id, &host, ann.port);
        sync_now_if_live();
    }
}

// Announce ourselves a few times on the local broadcast address. Called at
// app open, on foreground resume, and after an outbound pass that could not
// reach anyone - exactly the moments a network change surfaces.
pub fn burst(db: &Database, listen_port: u16) {
    let Some(device_id) = own_device_id(db) else {
        return;
    };
    std::thread::spawn(move || {
        let Ok(socket) = UdpSocket::bind("0.0.0.0:0") else {
            return;
        };
        let _ = socket.set_broadcast(true);
        let payload = match serde_json::to_vec(&Announce {
            device_id,
            port: listen_port,
        }) {
            Ok(p) => p,
            Err(_) => return,
        };
        for i in 0..BURST_PACKETS {
            let _ =
                socket.send_to(&payload, ("255.255.255.255", beacon_port()));
            if i + 1 < BURST_PACKETS {
                std::thread::sleep(BURST_SPACING);
            }
        }
    });
}

// Test seam: one announce straight to a known beacon listener address.
pub fn announce_to(
    addr: (&str, u16),
    device_id: &str,
    listen_port: u16,
) -> Result<(), String> {
    let socket =
        UdpSocket::bind("0.0.0.0:0").map_err(|e| format!("bind: {e}"))?;
    let payload = serde_json::to_vec(&Announce {
        device_id: device_id.to_string(),
        port: listen_port,
    })
    .map_err(|e| format!("encode: {e}"))?;
    socket
        .send_to(&payload, addr)
        .map_err(|e| format!("send: {e}"))?;
    Ok(())
}

// Test seam: run the listener on an already-bound socket (ephemeral port),
// so tests never race for the well-known beacon port.
pub fn spawn_listener_on(socket: UdpSocket, db: Arc<Database>) {
    std::thread::spawn(move || listen_on(socket, &db));
}
