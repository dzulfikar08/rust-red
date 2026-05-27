use std::sync::atomic::{AtomicU16, Ordering};
use tokio::sync::mpsc;

/// Raw bytes to be written to a client's TCP stream.
pub type RawPacket = Vec<u8>;

/// A message to be sent outbound to a connected client (structured form).
#[derive(Debug, Clone)]
pub struct OutboundMessage {
    pub topic: String,
    pub payload: Vec<u8>,
    pub qos: crate::protocol::packets::QoS,
    pub retain: bool,
    pub packet_id: Option<u16>,
}

static PACKET_ID_COUNTER: AtomicU16 = AtomicU16::new(1);

pub fn next_packet_id() -> u16 {
    // Wrap safely: skip 0 (invalid packet ID in MQTT)
    let val = PACKET_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
    if val == 0 {
        1
    } else {
        val
    }
}

pub struct Session {
    client_id: String,
    keep_alive: u16,
    connected_at: chrono::DateTime<chrono::Utc>,
    /// Channel for sending raw encoded packets to this client's writer task.
    outbound_tx: mpsc::Sender<RawPacket>,
}

impl Session {
    pub fn new(client_id: String, keep_alive: u16, outbound_tx: mpsc::Sender<RawPacket>) -> Self {
        Self { client_id, keep_alive, connected_at: chrono::Utc::now(), outbound_tx }
    }

    pub fn client_id(&self) -> &str {
        &self.client_id
    }
    pub fn keep_alive(&self) -> u16 {
        self.keep_alive
    }
    pub fn connected_at(&self) -> chrono::DateTime<chrono::Utc> {
        self.connected_at
    }

    /// Try to send a raw encoded packet to this session's TCP stream.
    /// Returns false if the channel is closed or full (client likely disconnected).
    pub fn send_raw(&self, packet: RawPacket) -> bool {
        self.outbound_tx.try_send(packet).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_new_fields() {
        let (tx, _rx) = mpsc::channel(16);
        let s = Session::new("client-1".into(), 30, tx);
        assert_eq!(s.client_id(), "client-1");
        assert_eq!(s.keep_alive(), 30);
    }

    #[test]
    fn session_send_raw_succeeds() {
        let (tx, mut rx) = mpsc::channel(16);
        let s = Session::new("c".into(), 60, tx);
        assert!(s.send_raw(vec![1, 2, 3]));
        let received = rx.try_recv().unwrap();
        assert_eq!(received, vec![1, 2, 3]);
    }

    #[test]
    fn session_send_raw_full_channel_returns_false() {
        let (tx, rx) = mpsc::channel(1);
        let s = Session::new("c".into(), 60, tx);
        assert!(s.send_raw(vec![1])); // fills channel
        assert!(!s.send_raw(vec![2])); // should fail
        drop(rx); // suppress unused warning
    }

    #[test]
    fn session_send_raw_closed_channel_returns_false() {
        let (tx, rx) = mpsc::channel(16);
        let s = Session::new("c".into(), 60, tx);
        drop(rx);
        assert!(!s.send_raw(vec![1]));
    }

    #[test]
    fn next_packet_id_monotonic() {
        let first = next_packet_id();
        let second = next_packet_id();
        assert!(second > first || (first == u16::MAX && second == 1));
    }
}
