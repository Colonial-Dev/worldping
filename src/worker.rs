use std::io::Read;
use std::io::Write;
use std::io::Error;

use std::thread;
use std::time::Duration;
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
    const ENOBUF_SLEEP_TIME: Duration = Duration::from_millis(1);
    
    let sock = Socket::new(
        Domain::IPV4,
        Type::RAW,
        Some(Protocol::ICMPV4)
    )?;

    let mut buf = [0_u8; ICMP_PACKET_SIZE];

    while let Ok(addr) = addrs.recv() {
        write_packet(&mut buf);

        loop {
            match sock
                .send_to(&buf, &addr)
                .as_ref()
                .map_err(Error::raw_os_error)
                .map_err(Option::unwrap)
            {
                Ok(_) => break,
                // The kernel can't buffer our send. Wait and try again.
                Err(libc::ENOBUFS) => { thread::sleep(ENOBUF_SLEEP_TIME) },
                // Some addresses can't be sent to and throw EPERM.
                // In this case, skip.
                Err(libc::EPERM) => break,
                // Something else has gone wrong. Panic.
                Err(_) => panic!(
                    "Send error in worker thread: {}",
                    Error::last_os_error()
                )
            }
        }
    }
    
    Ok(())
}

pub fn recv_worker() -> Result<()> {
    use libc::sock_filter as Op;

    // # Load u16 at byte offset 20 (ICMP type)
    // ldh [20]
    // # Drop the packet if the type is not 0x0 (ICMP echo reply)
    // jne #0x0, drop
    // # Load u16 at byte offset 24 (ICMP echo identifier field)
    // ldh [24]
    // # Drop the packet if the identifier is not 0x5750 ([b'W', b'P'] as big-endian u16)
    // jne #0x5750, drop
    // # If the above checks passed, send the packet up to userland.
    // ret #-1
    // # Otherwise, fucking obliterate it.
    // drop: ret #0
    static BPF: &[Op] = &[
        Op { code: 0x28,  jt: 0,  jf: 0, k: 0x00000014 },
        Op { code: 0x15,  jt: 0,  jf: 1, k: 0000000000 },
        Op { code: 0x28,  jt: 0,  jf: 0, k: 0x00000018 },
        Op { code: 0x15,  jt: 0,  jf: 1, k: 0x00005750 },
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