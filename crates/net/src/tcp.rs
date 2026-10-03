//! Length-prefixed TCP transport for LAN play and local testing.
//! Every message is delivered reliably regardless of the requested `Delivery`.

use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::{Delivery, NetEvent, PeerId, Transport};

pub const DEFAULT_PORT: u16 = 47_630;
/// Peer id of the host in TCP sessions. Clients are numbered from 2.
pub const HOST_ID: PeerId = 1;
const MAX_FRAME: usize = 16 * 1024 * 1024;

fn write_frame(stream: &mut TcpStream, data: &[u8]) -> io::Result<()> {
    stream.write_all(&(data.len() as u32).to_le_bytes())?;
    stream.write_all(data)
}

fn read_frame(stream: &mut TcpStream) -> io::Result<Vec<u8>> {
    let mut len = [0u8; 4];
    stream.read_exact(&mut len)?;
    let len = u32::from_le_bytes(len) as usize;
    if len > MAX_FRAME {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "frame too large"));
    }
    let mut buf = vec![0; len];
    stream.read_exact(&mut buf)?;
    Ok(buf)
}

/// Reads frames until the stream closes, forwarding them as events from `peer`.
fn spawn_reader(mut stream: TcpStream, peer: PeerId, tx: Sender<NetEvent>) {
    thread::spawn(move || {
        while let Ok(frame) = read_frame(&mut stream) {
            if tx.send(NetEvent::Message(peer, frame)).is_err() {
                return;
            }
        }
        let _ = tx.send(NetEvent::Disconnected(peer));
    });
}

pub struct TcpHost {
    port: u16,
    clients: Arc<Mutex<HashMap<PeerId, TcpStream>>>,
    events: Mutex<Receiver<NetEvent>>,
    shutdown: Arc<AtomicBool>,
}

impl TcpHost {
    pub fn bind(port: u16) -> io::Result<TcpHost> {
        let listener = TcpListener::bind(("0.0.0.0", port))?;
        listener.set_nonblocking(true)?;
        let port = listener.local_addr()?.port();
        let clients: Arc<Mutex<HashMap<PeerId, TcpStream>>> = Default::default();
        let shutdown = Arc::new(AtomicBool::new(false));
        let (tx, rx) = channel();

        let (clients2, shutdown2) = (clients.clone(), shutdown.clone());
        thread::spawn(move || {
            let mut next_id = HOST_ID + 1;
            while !shutdown2.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        let _ = stream.set_nonblocking(false);
                        let _ = stream.set_nodelay(true);
                        let Ok(reader) = stream.try_clone() else { continue };
                        clients2.lock().unwrap().insert(next_id, stream);
                        spawn_reader(reader, next_id, tx.clone());
                        next_id += 1;
                    }
                    Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(50));
                    }
                    Err(_) => return,
                }
            }
        });

        Ok(TcpHost {
            port,
            clients,
            events: Mutex::new(rx),
            shutdown,
        })
    }
}

impl Transport for TcpHost {
    fn send(&mut self, to: PeerId, _: Delivery, data: &[u8]) {
        let mut clients = self.clients.lock().unwrap();
        if let Some(stream) = clients.get_mut(&to)
            && write_frame(stream, data).is_err()
        {
            let _ = stream.shutdown(Shutdown::Both);
            clients.remove(&to);
        }
    }

    fn poll(&mut self) -> Vec<NetEvent> {
        let events: Vec<NetEvent> = self.events.lock().unwrap().try_iter().collect();
        let mut clients = self.clients.lock().unwrap();
        for e in &events {
            if let NetEvent::Disconnected(id) = e {
                clients.remove(id);
            }
        }
        events
    }

    fn describe(&self) -> String {
        format!("LAN host on port {}", self.port)
    }
}

impl Drop for TcpHost {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
        for stream in self.clients.lock().unwrap().values() {
            let _ = stream.shutdown(Shutdown::Both);
        }
    }
}

pub struct TcpClient {
    stream: TcpStream,
    addr: String,
    events: Mutex<Receiver<NetEvent>>,
}

impl TcpClient {
    /// Connects to `addr` (`host` or `host:port`). Blocks for up to a few seconds.
    pub fn connect(addr: &str) -> io::Result<TcpClient> {
        let full = if addr.contains(':') {
            addr.to_string()
        } else {
            format!("{addr}:{DEFAULT_PORT}")
        };
        let sock = full
            .to_socket_addrs()?
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "address did not resolve"))?;
        let stream = TcpStream::connect_timeout(&sock, Duration::from_secs(4))?;
        stream.set_nodelay(true)?;
        let (tx, rx) = channel();
        spawn_reader(stream.try_clone()?, HOST_ID, tx);
        Ok(TcpClient {
            stream,
            addr: full,
            events: Mutex::new(rx),
        })
    }
}

impl Transport for TcpClient {
    fn send(&mut self, _to: PeerId, _: Delivery, data: &[u8]) {
        // A failed write surfaces as a Disconnected event from the reader thread.
        if write_frame(&mut self.stream, data).is_err() {
            let _ = self.stream.shutdown(Shutdown::Both);
        }
    }

    fn poll(&mut self) -> Vec<NetEvent> {
        self.events.lock().unwrap().try_iter().collect()
    }

    fn describe(&self) -> String {
        format!("LAN: {}", self.addr)
    }
}

impl Drop for TcpClient {
    fn drop(&mut self) {
        let _ = self.stream.shutdown(Shutdown::Both);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    fn wait_for(t: &mut dyn Transport) -> Vec<NetEvent> {
        let start = Instant::now();
        loop {
            let ev = t.poll();
            if !ev.is_empty() || start.elapsed() > Duration::from_secs(3) {
                return ev;
            }
            thread::sleep(Duration::from_millis(10));
        }
    }

    #[test]
    fn host_and_client_exchange_messages() {
        let mut host = TcpHost::bind(0).unwrap();
        let mut client = TcpClient::connect(&format!("127.0.0.1:{}", host.port)).unwrap();
        client.send(HOST_ID, Delivery::Reliable, b"hello");
        let ev = wait_for(&mut host);
        let NetEvent::Message(peer, data) = &ev[0] else {
            panic!("{ev:?}")
        };
        assert_eq!(data, b"hello");
        host.send(*peer, Delivery::Reliable, b"welcome");
        let ev = wait_for(&mut client);
        assert!(matches!(&ev[0], NetEvent::Message(HOST_ID, d) if d == b"welcome"));

        drop(client);
        let ev = wait_for(&mut host);
        assert!(matches!(ev[0], NetEvent::Disconnected(p) if p == *peer));
    }
}
