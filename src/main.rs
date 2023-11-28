mod icmp;
mod worker;

use std::net::Ipv4Addr;

use anyhow::Result;
use socket2::*;

use crate::icmp::*;

fn main() -> Result<()> {
    let sock = Socket::new(
        Domain::IPV4,
        Type::RAW,
        Some(Protocol::ICMPV4)
    )?;

    let start: Ipv4Addr = "8.8.8.8".parse().unwrap();
    let end: Ipv4Addr = "8.8.255.255".parse().unwrap();

    let jh = std::thread::spawn(|| {
        worker::recv_worker().unwrap();
    });

    let rxs: Vec<_> = (0..4)
        .map(|_| {
            use std::sync::mpsc::*;

            let (tx, rx) = sync_channel(64);

            std::thread::spawn(move || {
                worker::send_worker(rx)
            });

            tx
        })
        .collect();

    worker::dispatch(
        &rxs, 
        addr_iter(start, end)
    )?;

    println!("done");

    std::thread::sleep(std::time::Duration::from_secs(5));

    Ok(())
}

fn addr_iter(start: Ipv4Addr, end: Ipv4Addr) -> impl Iterator<Item = SockAddr> {
    use std::net::SocketAddrV4;

    let s = u32::from(start);
    let f = u32::from(end);

    (s..=f)
        .map(Ipv4Addr::from)
        .map(|ip| SocketAddrV4::new(ip, 0) )
        .map(SockAddr::from)
}