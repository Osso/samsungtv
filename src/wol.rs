use std::net::{Ipv4Addr, UdpSocket};

use anyhow::{Context, Result, bail};

const WOL_PORT: u16 = 9;

/// Wake the TV: send the magic packet to every address that might reach it
/// (unicast to the host, limited broadcast, and the host's /24 broadcast).
/// Returns the destinations actually sent to.
pub fn wake(host: &str, mac: &str) -> Result<Vec<String>> {
    let packet = magic_packet(&parse_mac(mac)?);
    let socket = UdpSocket::bind("0.0.0.0:0").context("binding UDP socket")?;
    socket.set_broadcast(true).context("enabling broadcast")?;

    let mut targets = vec![
        format!("{host}:{WOL_PORT}"),
        format!("255.255.255.255:{WOL_PORT}"),
    ];
    if let Some(broadcast) = subnet_broadcast(host) {
        targets.push(format!("{broadcast}:{WOL_PORT}"));
    }

    let mut sent = Vec::new();
    for target in targets {
        socket
            .send_to(&packet, &target)
            .with_context(|| format!("sending magic packet to {target}"))?;
        sent.push(target);
    }
    // Keep the socket open briefly: application firewalls (appfw) attribute new
    // flows by scanning /proc for the owning process; if we exit immediately
    // the lookup races and the packets are silently denied.
    std::thread::sleep(std::time::Duration::from_millis(300));
    Ok(sent)
}

/// 6x 0xFF header followed by the MAC repeated 16 times (102 bytes total).
fn magic_packet(mac: &[u8; 6]) -> Vec<u8> {
    let mut packet = Vec::with_capacity(102);
    packet.extend_from_slice(&[0xFF; 6]);
    for _ in 0..16 {
        packet.extend_from_slice(mac);
    }
    packet
}

/// Parse "aa:bb:cc:dd:ee:ff" or "AA-BB-CC-DD-EE-FF" into bytes.
fn parse_mac(mac: &str) -> Result<[u8; 6]> {
    let parts: Vec<&str> = mac.split([':', '-']).collect();
    if parts.len() != 6 {
        bail!("invalid MAC address {mac:?}: expected 6 colon/dash-separated octets");
    }
    let mut bytes = [0u8; 6];
    for (byte, part) in bytes.iter_mut().zip(&parts) {
        *byte = u8::from_str_radix(part, 16)
            .with_context(|| format!("invalid MAC address {mac:?}: bad octet {part:?}"))?;
    }
    Ok(bytes)
}

/// The /24 directed broadcast for an IPv4 host (None for hostnames).
fn subnet_broadcast(host: &str) -> Option<Ipv4Addr> {
    let ip: Ipv4Addr = host.parse().ok()?;
    let [a, b, c, _] = ip.octets();
    Some(Ipv4Addr::new(a, b, c, 255))
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAC: [u8; 6] = [0xC0, 0x97, 0x27, 0xAA, 0xBB, 0xCC];

    #[test]
    fn magic_packet_is_102_bytes() {
        assert_eq!(magic_packet(&MAC).len(), 102);
    }

    #[test]
    fn magic_packet_starts_with_ff_header() {
        assert_eq!(&magic_packet(&MAC)[..6], &[0xFF; 6]);
    }

    #[test]
    fn magic_packet_repeats_mac_16_times() {
        let packet = magic_packet(&MAC);
        for repetition in packet[6..].chunks(6) {
            assert_eq!(repetition, MAC);
        }
        assert_eq!(packet[6..].len(), 16 * 6);
    }

    #[test]
    fn parses_colon_separated_lowercase() {
        assert_eq!(parse_mac("c0:97:27:aa:bb:cc").unwrap(), MAC);
    }

    #[test]
    fn parses_dash_separated_uppercase() {
        assert_eq!(parse_mac("C0-97-27-AA-BB-CC").unwrap(), MAC);
    }

    #[test]
    fn rejects_wrong_octet_count() {
        assert!(parse_mac("c0:97:27:aa:bb").is_err());
    }

    #[test]
    fn rejects_non_hex_octets() {
        assert!(parse_mac("c0:97:27:aa:bb:zz").is_err());
    }

    #[test]
    fn subnet_broadcast_for_ipv4_host() {
        assert_eq!(
            subnet_broadcast("192.168.1.42"),
            Some(Ipv4Addr::new(192, 168, 1, 255))
        );
    }

    #[test]
    fn no_subnet_broadcast_for_hostname() {
        assert_eq!(subnet_broadcast("tv.localdomain"), None);
    }
}
