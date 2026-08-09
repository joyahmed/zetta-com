//! UDP transport.
//!
//! Slice 1: prove datagrams move between two instances. No header, no audio —
//! just enough to confirm the socket, the ports and the firewall are right
//! before anything that matters rides on them.

use std::net::UdpSocket;
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result};

/// Bind an IPv4 socket and start talking to `peer`.
///
/// The bind address is written `0.0.0.0` on purpose. An empty host binds
/// IPv6-only on Windows and then silently discards every IPv4 datagram — the
/// bug that made every v1 listener play perfect silence.
pub fn start(port: u16, peer: String) -> Result<()> {
    let socket =
        UdpSocket::bind(("0.0.0.0", port)).with_context(|| format!("binding 0.0.0.0:{port}"))?;
    eprintln!("[net] bound 0.0.0.0:{port}, peer {peer}");

    let tx = socket.try_clone().context("cloning socket for sender")?;
    thread::Builder::new().name("net-tx".into()).spawn(move || {
        let mut n: u64 = 0;
        loop {
            n += 1;
            let msg = format!("hello {n}");
            if let Err(e) = tx.send_to(msg.as_bytes(), &peer) {
                eprintln!("[net] tx failed: {e}");
            }
            thread::sleep(Duration::from_millis(200));
        }
    })?;

    thread::Builder::new().name("net-rx".into()).spawn(move || {
        let mut buf = [0u8; 2048];
        loop {
            match socket.recv_from(&mut buf) {
                Ok((len, from)) => eprintln!(
                    "[net] rx {len} bytes from {from}: {}",
                    String::from_utf8_lossy(&buf[..len])
                ),
                // Don't spin hot on a broken socket.
                Err(e) => {
                    eprintln!("[net] rx failed: {e}");
                    thread::sleep(Duration::from_millis(200));
                }
            }
        }
    })?;

    Ok(())
}