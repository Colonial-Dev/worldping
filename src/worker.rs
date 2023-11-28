use std::io::Read;
use std::sync::mpsc::*;

use anyhow::Result;
use socket2::*;

use crate::icmp::*;

pub fn dispatch(senders: &[SyncSender<SockAddr>], addrs: impl Iterator<Item = SockAddr>) -> Result<()> {
    senders
        .iter()
        .cycle()
        .zip(addrs)
        .try_for_each(|(tx, addr)| tx.send(addr) )?;
    
    Ok(())
}

pub fn send_worker(addrs: Receiver<SockAddr>) -> Result<()> {
    let sock = Socket::new(
        Domain::IPV4,
        Type::RAW,
        Some(Protocol::ICMPV4)
    )?;

    sock.set_nonblocking(false)?;

    let mut buf = [0_u8; ICMP_PACKET_SIZE];

    while let Ok(addr) = addrs.recv() {
        write_packet(&mut buf);

        sock.send_to(&buf, &addr).unwrap();

        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    
    Ok(())
}

pub fn recv_worker() -> Result<()> {
    let sock = Socket::new(
        Domain::IPV4,
        Type::RAW,
        Some(Protocol::ICMPV4)
    )?;

    let mut buf = [0_u8; FULL_PACKET_SIZE];

    loop {
        (&sock).read_exact(&mut buf)?;

        if !Reply::is_valid(&buf) {
            continue; 
        }

        let reply = Reply::from_bytes(&buf);

        println!(
            "{reply:?} (RTT: {:?})", 
            reply.roundtrip_time()
        );
    }

    #[allow(unreachable_code)]
    Ok(())
}