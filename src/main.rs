mod icmp;

use std::io::Read;
use std::sync::mpsc::*;
use std::thread::JoinHandle;

use anyhow::Result;
use socket2::*;

use crate::icmp::*;

struct Worker {
    pub rx: Receiver<SockAddr>,
    pub buf: [u8; ICMP_PACKET_SIZE],
    pub sock: Socket,
}

struct WorkerHandle {
    pub tx: Sender<SockAddr>,
    pub jh: JoinHandle<()>
}

impl Worker {
    pub fn start() -> Result<WorkerHandle> {
        let (tx, rx) = channel();

        let buf = [0_u8; ICMP_PACKET_SIZE];

        let sock = Socket::new(
            Domain::IPV4,
            Type::RAW,
            Some(Protocol::ICMPV4)
        )?;

        let worker = Self { rx, buf, sock };

        let jh = std::thread::spawn(move || {
            
        });

        Ok(WorkerHandle { tx, jh })
    }
    
    fn ping(&mut self, addr: SockAddr) -> Result<()> {
        write_packet(&mut self.buf);

        self.sock.send_to(
            &self.buf,
            &addr
        )?;

        Ok(())
    }
}

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

    // Note: IPv4 headers are 20 bytes
    // The last 8 bytes are source and destination address, respectively.
    // Together with our 12 bytes of ICMP data, this makes each packet a static 32 bytes.

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