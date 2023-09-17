/// Messages a client can send to the server, but also what a server forwards to
/// clients.
///
/// TODO: state transition diagram.
#[derive(serde::Serialize, serde::Deserialize)]
pub enum Message {
    /// The first message a both clients send.
    ///
    /// The server will store the first message it receives and use it as
    /// response to the second that's incoming.
    ///
    /// A client receiving this is expected to respond with `PeerResponse`.
    ConnectionRequest { id: String, session_info: Vec<u8> },
    /// The response of a client after receiving the session info from the other
    /// client that wants to establish a peer connection.
    ///
    /// The server expects this as a response by Bob after the server has
    /// forwarded a `ConnectionRequest` from Alice to Bob.
    ///
    /// When the client receives this message, it has finished the ntmy
    /// handshake and should be able to finalize the p2p connection.
    PeerResponse { id: String, session_info: Vec<u8> },
    /// TODO
    IncrementalInfo { id: String, extra_info: Vec<u8> },
    /// TODO
    Done { id: String },
}

impl Message {
    pub fn connection_id(&self) -> &String {
        match self {
            Message::ConnectionRequest { id, .. }
            | Message::PeerResponse { id, .. }
            | Message::IncrementalInfo { id, .. }
            | Message::Done { id } => id,
        }
    }
}

/* WebRTC specifics */
#[derive(serde::Serialize, serde::Deserialize)]
pub struct WebRtcIncrementalInfo {
    pub candidate: String,
    pub sdp_m_line_index: u16,
    pub sdp_mid: String,
}
