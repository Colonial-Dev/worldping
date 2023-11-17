/// The size, in bytes, of an IPv4 header.
pub const IPV4_HEADER_SIZE: usize = 20;

/// The size, in bytes, of our custom ICMP packet.
pub const ICMP_PACKET_SIZE: usize = 12;

/// The total size, in bytes, of `worldping`'s echo requests and responses.
/// 
/// Equivalent to [`IPV4_HEADER_SIZE`] + [`ICMP_PACKET_SIZE`].
pub const FULL_PACKET_SIZE: usize = IPV4_HEADER_SIZE + ICMP_PACKET_SIZE;

/// Write a new ICMP echo request packet into the provided buffer.
///
/// ICMP packet layout:
/// - 1 byte for type (constant `8`)
/// - 1 byte for code (constant `0`)
/// - 2 bytes for checksum ([`icmp_checksum`])
/// - 8 bytes for payload (Unix time of dispatch, in milliseconds)
///   - Note that the first four bytes of payload are "packed" into the unneeded 2-byte "identifier" and
///     "sequence number" fields.
pub fn write_packet(buffer: &mut [u8; ICMP_PACKET_SIZE]) {
    use std::time::Duration;
    use std::time::SystemTime;
    use std::time::UNIX_EPOCH;

    const ICMP_TYPE_ECHO_REQ: u8 = 8;
    const ICMP_CODE_ECHO_REQ: u8 = 0;

    buffer[0] = ICMP_TYPE_ECHO_REQ;
    buffer[1] = ICMP_CODE_ECHO_REQ;

    // The checksum bytes should be zeroed before the sum is calculated.
    buffer[2] = 0;
    buffer[3] = 0;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .as_ref()
        .map(Duration::as_millis)
        .map(|x| x as u64)
        .map(u64::to_be_bytes)
        .expect("Failed to compute UNIX time");

    buffer[4..ICMP_PACKET_SIZE].copy_from_slice(&now);

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