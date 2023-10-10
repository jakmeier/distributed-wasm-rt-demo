use chrono::NaiveDateTime;

use crate::network;
use crate::p2p_proto::{Message, StealWorkBody};

const MIN_REQUEST_DELAY: chrono::Duration = chrono::Duration::milliseconds(300);

/// Manages outgoing requests to peers for work stealing.
#[derive(Default)]
pub(crate) struct PeerProxy {
    request_in_flight: bool,
    last_request_sent: NaiveDateTime,
}

impl PeerProxy {
    pub(crate) fn steal_work(&mut self, num_jobs: usize) {
        if self.request_in_flight {
            return;
        }
        let now = paddle::utc_now();
        if now > self.last_request_sent + MIN_REQUEST_DELAY {
            self.last_request_sent = now;
            self.request_in_flight = true;
            let request = Message::StealWork(StealWorkBody {
                num_jobs: num_jobs as u32,
            });
            let size_guess = 1 + num_jobs * 8 * 4;
            network::broadcast_async(request, Some(size_guess));
        }
    }

    /// indirect paddle event listener
    pub(crate) fn peer_message(&mut self, msg: &Message) {
        match msg {
            Message::Job(_) => self.request_in_flight = false,
            _ => (),
        }
    }

    /// indirect paddle event listener
    pub(crate) fn new_peer(&mut self, _msg: &network::NewPeerMsg) {
        // TODO: This is done to trigger sending requests to the new peer. This
        // kind of just works for a single peer. But I really should be tracking
        // each peer connection in-flight status separately.
        self.request_in_flight = false;
    }
}
