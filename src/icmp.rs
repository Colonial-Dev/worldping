#![allow(dead_code)]

use std::fmt::Write;
use std::ops::Range;
use std::net::Ipv4Addr;
use std::time::{Duration, UNIX_EPOCH};

use exemplar::*;

/// The size, in bytes, of an IPv4 header.
pub const IPV4_HEADER_SIZE: usize = 20;

/// The size, in bytes, of our custom ICMP packet.
pub const ICMP_PACKET_SIZE: usize = 14;

/// The total size, in bytes, of `worldping`'s echo requests and responses.
/// 
/// Equivalent to [`IPV4_HEADER_SIZE`] + [`ICMP_PACKET_SIZE`].
pub const FULL_PACKET_SIZE: usize = IPV4_HEADER_SIZE + ICMP_PACKET_SIZE;

/// The byte index (in a received packet, with IPV4 header) of the ICMP message type byte.
pub const ICMP_TYPE_INDEX: usize = 20;

/// The byte range (in a received packet, with IPV4 header) of the ICMP payload.
pub const ICMP_DATA_RANGE: Range<usize> = 26..FULL_PACKET_SIZE;

/// The byte range (in a received packet, with IPV4 header) of the sender's IP address.
pub const IPV4_IP_RANGE: Range<usize> = 12..16;

/// The byte range (in a received packet, with IPV4 header) of the echo "identifier" field.
pub const ICMP_ID_RANGE: Range<usize> = 24..26;

#[derive(Debug)]
pub struct Reply {
    pub from: Ipv4Addr,
    pub sent: u64,
}

impl Reply {
    pub fn from_bytes(buf: &[u8; FULL_PACKET_SIZE]) -> Self {
        let from = Ipv4Addr::from(
            slice_array::<4>(&buf[IPV4_IP_RANGE])
        );

        let sent = u64::from_be_bytes(
            slice_array::<8>(&buf[ICMP_DATA_RANGE])
        );


        Self { from, sent }
    }

    pub fn roundtrip_time(&self) -> Duration {
        use std::time::SystemTime;

        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .as_ref()
            .map(Duration::as_millis)
            .map(|x| {
                (x as u64).saturating_sub(self.sent)
            })
            .map(Duration::from_millis)
            .unwrap()
    }
}

#[derive(Debug, Model)]
#[table("replies")]
pub struct Record {
    pub address: String,
    pub time: Option<f64>,
    pub seen: bool,
}

impl Record {
    pub fn overwrite(&mut self, source: &Reply) {
        self.address.clear();

        let _ = write!(
            self.address,
            "{}",
            source.from
        );

        self.time = Some( source.roundtrip_time().as_micros() as f64 / 1000.0 );
        self.seen = true;
    }
}

fn slice_array<const N: usize>(slice: &[u8]) -> [u8; N] {
    assert!(
        slice.len() == N,
        "Tried to convert an incorrectly-sized slice to an array!"
    );

    slice.try_into().unwrap()
}

/// Write a new ICMP echo request packet into the provided buffer.
///
/// ICMP packet layout:
/// - 1 byte for type (constant `8`)
/// - 1 byte for code (constant `0`)
/// - 2 bytes for checksum ([`icmp_checksum`])
/// - 2 bytes for identifier (constant `"WP"`)
/// - 8 bytes for payload (Unix time of dispatch, in milliseconds)
///   - Note that the first two bytes of payload are "packed" into the unneeded sequence number field.
pub fn write_packet(buffer: &mut [u8; ICMP_PACKET_SIZE]) {
    use std::time::SystemTime;

    const ICMP_TYPE_ECHO_REQ: u8 = 8;
    const ICMP_CODE_ECHO_REQ: u8 = 0;
    const ICMP_ID: [u8; 2] = [b'W', b'P'];

    buffer[0] = ICMP_TYPE_ECHO_REQ;
    buffer[1] = ICMP_CODE_ECHO_REQ;

    // The checksum bytes should be zeroed before the sum is calculated.
    buffer[2] = 0;
    buffer[3] = 0;

    buffer[4..6].copy_from_slice(&ICMP_ID);

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .as_ref()
        .map(Duration::as_millis)
        .map(|x| x as u64)
        .map(u64::to_be_bytes)
        .unwrap();

    buffer[6..ICMP_PACKET_SIZE].copy_from_slice(&now);

    icmp_checksum(buffer);
}

/// Compute the checksum of an ICMP packet.
pub fn icmp_checksum(buffer: &mut [u8; ICMP_PACKET_SIZE]) {
    let mut sum = 0_u32;

    for chunk in buffer.chunks(2) {
        let mut part = (chunk[0] as u16) << 8;

        if chunk.len() > 1 {
            part += chunk[1] as u16;
        }

        sum = sum.wrapping_add(part as u32);
    }

    while (sum >> 16) > 0 {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }

    let sum = !sum as u16;

    buffer[2] = (sum >> 8) as u8;
    buffer[3] = (sum & 0xFF) as u8;
}