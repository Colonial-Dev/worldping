mod icmp;

use std::io::Read;
use std::sync::mpsc::*;
use std::thread::JoinHandle;

use anyhow::Result;
use socket2::*;

use crate::icmp::*;

struct Reply<'a> {
    pub from: &'a [u8],
    pub data: &'a [u8]
}

impl<'a> Reply<'a> {
    pub fn from_bytes(buf: &'a [u8; FULL_PACKET_SIZE]) -> Self {
        Reply {
            from: &buf[12..16],
            data: &buf[IPV4_HEADER_SIZE..FULL_PACKET_SIZE]
        }
    }
}

fn main() -> Result<()> {
    let sock = Socket::new(
        Domain::IPV4,
        Type::RAW,
        Some(Protocol::ICMPV4)
    )?;

    let localhost: std::net::SocketAddr = "127.0.0.1:65535".parse().unwrap();
    let localhost = SockAddr::from(localhost);

    let mut req = [0u8; ICMP_PACKET_SIZE];
    let mut buf = [0u8; FULL_PACKET_SIZE];

    loop {
        write_packet(&mut req);
        
        sock.send_to(&req, &localhost)?;

        std::thread::sleep(std::time::Duration::from_millis(100));

        (&sock).read_exact(&mut buf)?;

        println!("{buf:?}");
    }

    Ok(())
}