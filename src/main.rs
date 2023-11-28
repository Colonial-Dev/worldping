mod icmp;
mod worker;

use std::net::Ipv4Addr;

use anyhow::Result;
use socket2::*;

fn main() -> Result<()> {
    let start: Ipv4Addr = "8.0.0.1".parse().unwrap();
    let end: Ipv4Addr = "8.255.255.255".parse().unwrap();
    
    std::thread::spawn(|| {
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

    eprintln!("done dispatching");

    std::thread::sleep(std::time::Duration::from_secs(5));

    Ok(())
}

fn addr_iter(start: Ipv4Addr, end: Ipv4Addr) -> impl Iterator<Item = SockAddr> {
    use std::net::SocketAddrV4;

    let start = u32::from(start);
    let end = u32::from(end);

    (start..=end)
        .map(Ipv4Addr::from)
        .map(|ip| SocketAddrV4::new(ip, 0) )
        .map(SockAddr::from)
}