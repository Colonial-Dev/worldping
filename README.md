<h1 align="center">Worldping</h1>
<h3 align="center">A command-line tool for mass IPv4 pinging.</h3>

<p align="center">
<img src="https://img.shields.io/github/license/Colonial-Dev/worldping">
<img src="https://img.shields.io/github/stars/Colonial-Dev/worldping">
</p>

## Features
- Ping the entire version 4 address space, or just a subset of it.
- Response data (address and approximate round-trip time) is written to a SQLite database for ease of analysis.
- Extremely efficient. With a good upload link, `worldping` can scan a block of `2^24` (~16.7 million) addresses in thirty seconds.
    - (For those wondering, that works out to the entire version 4 address space every two-ish hours.)

## Installation
Requirements:
- A Unix system with BPF (Berkeley packet filtering) support.
- Permission to open raw sockets. This typically means running as `root`, but on Linux attaching the `CAP_NET_RAW` capability to the binary should also work.

Dependencies:
- The [Rust programming language](https://rustup.rs/).
- A C/C++ toolchain (such as `gcc`.)

Just use `cargo install`, and `worldping` will be compiled and added to your `PATH`.
```sh
cargo install --locked --git https://github.com/Colonial-Dev/worldping --branch master
```

## Command Line Options
- `--start-addr` - the start of the IPv4 address range to ping, inclusive.
    - Defaults to `0.0.0.0`.
- `--end-addr` - the end of the IPv4 address range to ping, inclusive.
    - Defaults to `255.255.255.255`.
- `-w`/`--workers` - the number of worker threads to use for dispatch.
    - Defaults to `system threads / 3`.
    - Note that more threads is not necessarily better. You will likely saturate your upload link with only a few threads.
- `-o`/`--output` - the path of the output database.
    - Defaults to `output-YYYY-MM-DD-HH-MM.db` in the working directory.
- `-p`/`--prefill` - whether or not to "prefill" the output database. (Warning - disk heavy!)
    - By default, `worldping` only inserts records for the addresses it hears back from. If you want the collected data to include *all* addresses in the specified range, then `--prefill` is for you.
    - Prefilled addresses have a `NULL` `time` column and a `FALSE` `seen` column. When an address responds to a ping, these columns will be updated appropriately.
