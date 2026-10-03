//! Networking: the wire protocol plus pluggable transports. Steam is the
//! primary transport; plain TCP exists for LAN play and for testing two
//! instances on one machine without two Steam accounts.

pub mod protocol;
pub mod steam;
pub mod tcp;

/// Identifies a peer. For Steam this is the raw SteamID; for TCP it is a
/// session-local counter assigned by the host.
pub type PeerId = u64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Delivery {
    /// Guaranteed and ordered (world chunks, joins, edits).
    Reliable,
    /// May be dropped; for frequently resent state like positions.
    Unreliable,
}

#[derive(Debug)]
pub enum NetEvent {
    Message(PeerId, Vec<u8>),
    /// The peer left or the connection dropped. On a client, the peer is the host.
    Disconnected(PeerId),
}

/// A message pipe between the host and its clients. Clients only ever talk
/// to the host (star topology), so relaying is the host's job.
pub trait Transport: Send + Sync + 'static {
    fn send(&mut self, to: PeerId, delivery: Delivery, data: &[u8]);
    /// Drains everything received since the last call.
    fn poll(&mut self) -> Vec<NetEvent>;
    /// Human-readable description shown in the HUD (lobby id, address…).
    fn describe(&self) -> String;
}
