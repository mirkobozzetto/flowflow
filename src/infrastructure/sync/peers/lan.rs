use crate::infrastructure::sync::SyncError;
use std::net::{Ipv4Addr, UdpSocket};

pub(super) fn local_lan_ip() -> Result<String, SyncError> {
    if let Ok(addr) = std::env::var("FLOWFLOW_SYNC_ADVERTISE_ADDR") {
        if !addr.is_empty() {
            return Ok(addr);
        }
    }
    // Enumerate interfaces and pick a real Wi-Fi/LAN address first. The UDP-connect trick below
    // can latch onto a VPN/hotspot interface (utun*, e.g. 192.168.129.x) on iOS, baking an
    // unreachable address into the pairing payload, so it is only a last resort.
    if let Some(ip) = lan_ipv4_from_interfaces() {
        return Ok(ip);
    }
    let socket = UdpSocket::bind("0.0.0.0:0")
        .map_err(|e| SyncError::Pairing(format!("udp bind: {e}")))?;
    socket
        .connect("8.8.8.8:80")
        .map_err(|e| SyncError::Pairing(format!("no LAN route: {e}")))?;
    let addr = socket
        .local_addr()
        .map_err(|e| SyncError::Pairing(format!("local addr: {e}")))?;
    Ok(addr.ip().to_string())
}

// A usable LAN address is a private IPv4 on a physical Wi-Fi/Ethernet interface. Exclude Apple's
// VPN (utun*), cellular (pdp_ip*), AirDrop/awdl, low-latency-WLAN (llw*), bridge and hotspot (ap*)
// interfaces: those carry private IPs too (the utun 192.168.129.x bug) but are not reachable by a
// LAN peer.
fn is_usable_lan(name: &str, ip: Ipv4Addr) -> bool {
    const SKIP: &[&str] =
        &["utun", "pdp_ip", "awdl", "llw", "bridge", "ap", "lo"];
    ip.is_private() && !SKIP.iter().any(|p| name.starts_with(p))
}

// Best private IPv4 across up, non-loopback interfaces, preferring Wi-Fi (en*). Returns None when
// nothing qualifies, so the caller falls back to the UDP-route heuristic.
fn lan_ipv4_from_interfaces() -> Option<String> {
    use std::ffi::CStr;
    unsafe {
        let mut ifap: *mut libc::ifaddrs = std::ptr::null_mut();
        if libc::getifaddrs(&mut ifap) != 0 || ifap.is_null() {
            return None;
        }
        let mut preferred: Option<String> = None;
        let mut fallback: Option<String> = None;
        let mut cur = ifap;
        while !cur.is_null() {
            let ifa = &*cur;
            cur = ifa.ifa_next;
            if ifa.ifa_addr.is_null() || ifa.ifa_name.is_null() {
                continue;
            }
            let sa = &*ifa.ifa_addr;
            if i32::from(sa.sa_family) != libc::AF_INET {
                continue;
            }
            if ifa.ifa_flags & libc::IFF_UP as u32 == 0
                || ifa.ifa_flags & libc::IFF_LOOPBACK as u32 != 0
            {
                continue;
            }
            let name = CStr::from_ptr(ifa.ifa_name).to_string_lossy();
            let sin = ifa.ifa_addr as *const libc::sockaddr_in;
            let o = (*sin).sin_addr.s_addr.to_ne_bytes();
            let ip = Ipv4Addr::new(o[0], o[1], o[2], o[3]);
            if !is_usable_lan(&name, ip) {
                continue;
            }
            if name.starts_with("en") {
                preferred = Some(ip.to_string());
                break;
            } else if fallback.is_none() {
                fallback = Some(ip.to_string());
            }
        }
        libc::freeifaddrs(ifap);
        preferred.or(fallback)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usable_lan_accepts_wifi_rejects_vpn() {
        // real Wi-Fi / Ethernet, private range
        assert!(is_usable_lan("en0", Ipv4Addr::new(192, 168, 1, 43)));
        assert!(is_usable_lan("en1", Ipv4Addr::new(10, 0, 0, 5)));
        // the bug: utun carries a private IP but is not LAN-reachable
        assert!(!is_usable_lan("utun3", Ipv4Addr::new(192, 168, 129, 5)));
        // cellular, AirDrop, loopback, public
        assert!(!is_usable_lan("pdp_ip0", Ipv4Addr::new(10, 1, 2, 3)));
        assert!(!is_usable_lan("awdl0", Ipv4Addr::new(169, 254, 1, 1)));
        assert!(!is_usable_lan("lo0", Ipv4Addr::new(127, 0, 0, 1)));
        assert!(!is_usable_lan("en0", Ipv4Addr::new(8, 8, 8, 8)));
    }
}
