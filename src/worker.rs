use std::io::Read;
use std::io::Error;

use std::thread;
use std::time::Duration;

use std::sync::Arc;
use std::sync::mpsc::*;
use std::sync::atomic::*;

use anyhow::Result;
use indicatif::*;
use exemplar::*;
use rusqlite::Connection;
use socket2::*;

use crate::icmp::*;

pub fn dispatch(senders: &[SyncSender<SockAddr>], addrs: impl ExactSizeIterator<Item = SockAddr>) -> Result<()> {
    let bar = ProgressBar::new(
        addrs.len() as u64
    ).with_finish(ProgressFinish::Abandon);
    
    bar.set_style(
        ProgressStyle::with_template("[{msg:^16}] {wide_bar} [{percent}% / {eta}]")?
    );

    senders
        .iter()
        .cycle()
        .zip(addrs)
        .inspect(|(_, addr)| {
            let msg = format!(
                "{}",
                addr.as_socket_ipv4().unwrap().ip()
            );

            bar.set_message(msg);
            bar.inc(1);
        })
        .try_for_each(|(tx, addr)| tx.send(addr) )?;
    
    Ok(())
}

pub fn send_worker(rx: Receiver<SockAddr>) -> Result<()> {
    const BASE_SLEEP_TIME: u64 = 2;
    
    let sock = Socket::new(
        Domain::IPV4,
        Type::RAW,
        Some(Protocol::ICMPV4)
    )?;
    
    let mut exp;
    let mut buf = [0_u8; ICMP_PACKET_SIZE];

    while let Ok(addr) = rx.recv() {
        write_packet(&mut buf);
        exp = 0;

        loop {

            match sock
                .send_to(&buf, &addr)
                .as_ref()
                .map_err(Error::raw_os_error)
                .map_err(Option::unwrap)
            {
                // All is well. Next address.
                Ok(_) => break,
                // The kernel can't buffer our send. Wait and try again.
                Err(libc::ENOBUFS) => {
                    // Cap our exponential backoff at ~16ms.
                    exp = u32::min(exp + 1, 4);

                    let sleep = Duration::from_millis(
                        BASE_SLEEP_TIME.pow(exp)
                    );

                    thread::sleep(sleep)
                }
                // Some addresses can't be sent to and throw EPERM.
                // In this case, skip.
                Err(libc::EPERM) | Err(libc::EACCES) => break,
                // Something else has gone wrong. Bail.
                Err(_) => panic!(
                    "Send error in worker thread: {}",
                    Error::last_os_error()
                )
            }
        }
    }

    Ok(())
}

pub fn recv_worker(token: Arc<AtomicBool>, mut conn: Connection) -> Result<()> {
    use libc::sock_filter as Op;

    const READ_TIMEOUT: Option<Duration> = Some( Duration::from_secs(1) );

    // # Load half word (u16) at byte offset 20 (ICMP type)
    // ldh [20]
    // # Drop the packet if the type is not 0x0 (ICMP echo reply)
    // jne #0x0, drop
    // # Load u16 at byte offset 24 (ICMP echo identifier field)
    // ldh [24]
    // # Drop the packet if the identifier is not 0x5750 ([b'W', b'P'] as big-endian u16)
    // jne #0x5750, drop
    // # If the above checks passed, send the packet up to userland.
    // ret #-1
    // # Otherwise, discard it.
    // drop: ret #0
    static PACKET_FILTER: &[Op] = &[
        Op { code: 0x28,  jt: 0,  jf: 0, k: 0x00000014 },
        Op { code: 0x15,  jt: 0,  jf: 1, k: 0000000000 },
        Op { code: 0x28,  jt: 0,  jf: 0, k: 0x00000018 },
        Op { code: 0x15,  jt: 0,  jf: 1, k: 0x00005750 },
        Op { code: 0x06,  jt: 0,  jf: 0, k: 0xffffffff },
        Op { code: 0x06,  jt: 0,  jf: 0, k: 0000000000 },
    ];

    let mut record = Record {
        address: String::with_capacity(16),
        time: Some(0),
        seen: true,
    };

    let txn = conn.transaction()?;

    let sock = Socket::new(
        Domain::IPV4,
        Type::RAW,
        Some(Protocol::ICMPV4)
    )?;

    sock.attach_filter(PACKET_FILTER)?;
    sock.set_read_timeout(READ_TIMEOUT)?;

    let mut buf = [0_u8; FULL_PACKET_SIZE];
    
    loop {
        match (&sock)
            .read_exact(&mut buf)
            .as_ref()
            .map_err(Error::raw_os_error)
            .map_err(Option::unwrap)
        {
            // All is well. Record the reply and continue.
            Ok(_) => {
                record.overwrite(
                    &Reply::from_bytes(&buf)
                );

                record.insert_or(
                    &txn,
                    OnConflict::Replace
                )?;
            },
            // Read timeout. Check cancellation token, and break if true.
            // Otherwise, continue.
            Err(libc::EAGAIN) => if token.load(Ordering::Relaxed) { 
                txn.commit()?;
                break;
            },
            // Something else has gone wrong. Bail.
            Err(_) => panic!(
                "Receive error in worker thread: {}",
                Error::last_os_error()
            )
        }
    }

    Ok(())
}
