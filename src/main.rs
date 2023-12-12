mod args;
mod icmp;
mod worker;

use std::net::Ipv4Addr;
use std::path::PathBuf;
use std::thread;
use std::time::*;

use std::sync::Arc;
use std::sync::atomic::*;

use anyhow::Result;
use clap::Parser;
use exemplar::Model;
use rusqlite::Connection;
use socket2::*;

use crate::args::*;

fn main() -> Result<()> {
    let args = Arguments::parse();

    check_perms()?;

    let n_threads = args
        .workers
        .unwrap_or_else(worker_threads);

    let (rxs, jhs) = (0..n_threads)
        .map(|_| {
            use std::sync::mpsc::*;

            let (tx, rx) = sync_channel(64);

            (
                tx,
                std::thread::spawn(move || worker::send_worker(rx) )
            )
        })
        .fold((vec![], vec![]), |(mut rxs, mut jhs), (rx, jh)| {
            rxs.push(rx);
            jhs.push(jh);

            (rxs, jhs)
        });

    let mut conn = open_db(args.output)?;

    if args.prefill {
        print!("Prefilling database... ");

        let txn = conn.transaction()?;

        addr_iter(args.start_addr, args.end_addr)
            .map(|addr| {
                let address = addr
                    .as_socket_ipv4()
                    .unwrap()
                    .ip()
                    .to_string();

                icmp::Record {
                    address,
                    time: None,
                    seen: false
                }
            })
            .try_for_each(|r| r.insert(&txn) )?;

        txn.commit()?;
        
        println!("done.")
    }

    let token = AtomicBool::new(false);
    let token = Arc::new(token);
    let clone = Arc::clone(&token);
    
    let receiver = std::thread::spawn(move || worker::recv_worker(clone, conn) );

    println!(
        "Pinging {} addresses using {} worker threads...\n",
        indicatif::HumanCount( addr_iter(args.start_addr, args.end_addr).len() as u64 ),
        // N send threads and one receiving thread.
        n_threads + 1
    );

    let now = Instant::now();

    worker::dispatch(
        &rxs, 
        addr_iter(args.start_addr, args.end_addr)
    )?;

    token.store(true, Ordering::Relaxed);

    let later = Instant::now();

    println!(
        "\n\nDone! (took {})\nWaiting for worker threads to time out...",
        indicatif::HumanDuration(later - now)
    );

    receiver
        .join()
        .expect("Worker thread panicked!")?;

    drop(rxs);

    for handle in jhs {
        handle.join().expect("Worker thread panicked!")?;
    }

    Ok(())
}

fn addr_iter(start: Ipv4Addr, end: Ipv4Addr) -> impl ExactSizeIterator<Item = SockAddr> {
    use std::net::SocketAddrV4;

    // Needed for two reasons:
    // - When using u32, it's not possible to include 255.255.255.255 in the range (as RangeInclusive<u32> doesn't impl ExactSizeIterator.)
    // - u64 doesn't work, because its range also doesn't impl ExactSizeIterator (???)
    let start = u32::from(start) as usize;
    let end = u32::from(end) as usize;

    (start..end + 1)
        .map(|x| x as u32)
        .map(Ipv4Addr::from)
        .map(|ip| SocketAddrV4::new(ip, 0) )
        .map(SockAddr::from)
}

fn worker_threads() -> usize {
    use std::num::NonZeroUsize;

    let threads = thread::available_parallelism()
        .unwrap_or( unsafe { NonZeroUsize::new_unchecked(1) } )
        .get();

    threads / 3
}

fn check_perms() -> Result<()> {
    use anyhow::*;

    let sock = Socket::new(
        Domain::IPV4,
        Type::RAW,
        Some(Protocol::ICMPV4)
    );

    if let Err(e) = sock {
        let err = anyhow!(e)
            .context("Failed to open test socket - are you not running worldping as root or with CAP_NET_RAW?");

        bail!(err)
    }

    Ok(())
}

fn open_db(path: Option<PathBuf>,) -> Result<Connection> {
    use std::fs;
    use std::path::Path;

    let default_path = || unsafe {
        let time = libc::time( std::ptr::null_mut() );
        let time = libc::localtime( &time as *const i64 );
        let time = *time;

        format!(
            "output-{}-{}-{}-{}-{}.db",
            // C moment
            time.tm_year + 1900,
            time.tm_mon,
            time.tm_mday,
            time.tm_hour,
            time.tm_min
        )
    };

    let path = match path {
        Some(path) => path,
        None => default_path().into()
    };

    if Path::new(&path).exists() {
        fs::remove_file(&path)?
    }
    
    let conn = Connection::open(path)?;

    conn.execute_batch("
        PRAGMA journal_mode = WAL;
        PRAGMA synchronous = NORMAL;

        CREATE TABLE replies (
            address  TEXT, 
            time     REAL,
            seen     BOOLEAN
        );
    ")?;

    Ok(conn)
}