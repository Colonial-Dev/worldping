use std::net::Ipv4Addr;
use std::path::PathBuf;

use clap::Parser;

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
pub struct Arguments {
    /// The start of the IPv4 address range to ping
    #[arg(long, value_parser = clap::value_parser!(Ipv4Addr), default_value = "0.0.0.0", )]
    pub start_addr: Ipv4Addr,
    /// The end of the IPv4 address range to ping
    #[arg(long, value_parser = clap::value_parser!(Ipv4Addr), default_value = "255.255.255.255", )]
    pub end_addr: Ipv4Addr,
    /// The number of worker threads to use for dispatch. Defaults to `system threads / 3`
    #[arg(short, long)]
    pub workers: Option<usize>,
    /// The path of the output database. Defaults to `output-YYYY-MM-DD-HH-MM.db`
    #[arg(short, long)]
    pub output: Option<PathBuf>,
    /// Whether or not to "prefill" the output database
    #[arg(short, long, default_value_t = false)]
    pub prefill: bool,
}