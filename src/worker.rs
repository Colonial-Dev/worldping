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
    use libc::sock_filter as Op;

    // ldh [20]
    // jne #0x0, drop
    // ret #-1
    // drop: ret #0
    static BPF: &[Op] = &[
        Op { code: 0x28,  jt: 0,  jf: 0, k: 0x00000014 },
        Op { code: 0x15,  jt: 0,  jf: 1, k: 0000000000 },
        Op { code: 0x06,  jt: 0,  jf: 0, k: 0xffffffff },
        Op { code: 0x06,  jt: 0,  jf: 0, k: 0000000000 },
    ];

    let sock = Socket::new(
        Domain::IPV4,
        Type::RAW,
        Some(Protocol::ICMPV4)
    )?;

    sock.attach_filter(BPF)?;

    let mut buf = [0_u8; FULL_PACKET_SIZE];

    use std::io::Write;
    let mut stdout = std::io::stdout().lock();

    loop {
        (&sock).read_exact(&mut buf)?;

        let reply = Reply::from_bytes(&buf);

        writeln!(
            stdout,
            "{reply:?} (RTT: {:?})", 
            reply.roundtrip_time()
        )?;
    }

    #[allow(unreachable_code)]
    Ok(())
}