use std::collections::HashMap;
use std::rc::Rc;

use js_sys::Reflect;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    MessageEvent, RtcConfiguration, RtcDataChannel, RtcDataChannelEvent, RtcIceCandidate,
    RtcIceCandidateInit, RtcPeerConnection, RtcPeerConnectionIceEvent, RtcSdpType,
    RtcSessionDescriptionInit,
};

use crate::ws::{ReadyState, WebSocketWrapper};

pub(crate) struct SignalingServerConnection {
    /// WebSocket to signalling server used for RTC connection setup.
    signaling: WebSocketWrapper,

    /// Browser objects for peer connections and the connection state, used
    /// while signalling.
    ///
    /// Note the difference to `PeerConnection` which is the type used outside
    /// this module, for interacting with the peer on an "application layer".
    peers: HashMap<String, (RtcPeerConnection, RemoteState)>,

    /// closure to re-use
    on_err_add_ice_candidate: Closure<dyn FnMut(JsValue)>,
}

pub(crate) struct PeerConnection {
    channel: RtcDataChannel,
    // closures to keep alive
    _on_data_channel: Closure<dyn FnMut(RtcDataChannelEvent)>,
    _on_ice: Closure<dyn FnMut(RtcPeerConnectionIceEvent)>,
}

enum RemoteState {
    WaitingForResponse { buffered: Vec<RtcIceCandidate> },
    Connected,
}

struct Forward(ntmy::Message);
struct RemoteConnectedMsg {
    id: String,
}
struct OpenOfferMsg {
    id: String,
    peer: RtcPeerConnection,
}

impl paddle::Frame for SignalingServerConnection {
    type State = ();
    const WIDTH: u32 = 100;
    const HEIGHT: u32 = 100;
}

impl SignalingServerConnection {
    pub fn start() {
        let handle = paddle::register_frame_no_state(Self::new(), (0, 0));
        handle.register_receiver(Self::accept_ntmy_msg);
        handle.register_receiver(Self::forward_ntmy_msg);
        handle.register_receiver(Self::open_offer);
        handle.listen(Self::on_remote_connected);
    }
}

impl SignalingServerConnection {
    const DATA_CHANNEL_NAME: &str = "my-data-channel";

    fn new() -> Self {
        let on_err_add_ice_candidate = Closure::new(move |err| {
            paddle::println!("Adding RtcPeerCandidate produced an error {err:?}.");
        });

        // TODO: Don't use static URL, use env variable or something.
        let url = "wss://demos.jakobmeier.ch/ntmy/ws";

        let onmessage = Rc::new(Closure::<dyn FnMut(_)>::new(move |e: MessageEvent| {
            if let Ok(abuf) = e.data().dyn_into::<js_sys::ArrayBuffer>() {
                let vec = js_sys::Uint8Array::new(&abuf).to_vec();
                let ntmy_msg = bendy::serde::from_bytes::<ntmy::Message>(&vec)
                    .expect("unparsable ntmy message");
                paddle::send::<_, SignalingServerConnection>(ntmy_msg);
            } else if let Ok(_blob) = e.data().dyn_into::<web_sys::Blob>() {
                paddle::println!("unexpectedly received blob");
            } else if let Ok(txt) = e.data().dyn_into::<js_sys::JsString>() {
                paddle::println!("unexpectedly received string: {txt}");
            } else {
                paddle::println!("unexpectedly received something else");
            }
        }));

        let signaling = WebSocketWrapper::new(url, onmessage);

        Self {
            signaling,
            on_err_add_ice_candidate,
            peers: HashMap::new(),
        }
    }

    fn rtc_config() -> Result<RtcConfiguration, JsValue> {
        let mut config = RtcConfiguration::new();
        let servers = js_sys::JSON::parse(r#"[{"urls": "stun:stun.l.google.com:19302"}]"#)?;
        config.ice_servers(&servers);
        Ok(config)
    }

    pub fn new_connection(
        id: String,
        on_open: fn(&RtcDataChannel, &str),
        on_msg: fn(&RtcDataChannel, &str, MessageEvent),
    ) -> PeerConnection {
        let rtc_config = Self::rtc_config().unwrap();
        let peer = RtcPeerConnection::new_with_configuration(&rtc_config).unwrap();
        let data_channel = peer.create_data_channel(Self::DATA_CHANNEL_NAME);
        let id0 = id.clone();
        let id1 = id.clone();
        // Set up callbacks to the channel
        init_data_channel(
            data_channel.clone(),
            move |c, msg| on_msg(c, &id0, msg),
            move |c| on_open(c, &id1),
        );

        let id2 = id.clone();
        let on_data_channel = Closure::<dyn FnMut(_)>::new(move |ev: RtcDataChannelEvent| {
            let id3 = id2.clone();
            let id4 = id2.clone();
            paddle::println!("data channel opened by remote");
            init_data_channel(
                ev.channel(),
                move |c, msg| on_msg(c, &id3, msg),
                move |c| on_open(c, &id4),
            );
        });

        // When the remote peer adds a data channel, set up callbacks, too
        peer.set_ondatachannel(Some(on_data_channel.as_ref().unchecked_ref()));

        let on_ice = ice_candidate_trickling_callback(id.clone());
        peer.set_onicecandidate(Some(on_ice.as_ref().unchecked_ref()));

        paddle::send::<_, SignalingServerConnection>(OpenOfferMsg { id, peer });

        PeerConnection {
            channel: data_channel,
            _on_data_channel: on_data_channel,
            _on_ice: on_ice,
        }
    }

    fn open_offer(&mut self, _state: &mut (), OpenOfferMsg { peer, id }: OpenOfferMsg) {
        self.peers.insert(
            id.clone(),
            (
                peer.clone(),
                RemoteState::WaitingForResponse { buffered: vec![] },
            ),
        );
        wasm_bindgen_futures::spawn_local(Self::async_open_offer(peer, self.signaling.clone(), id));
    }

    async fn async_open_offer(peer: RtcPeerConnection, signaling: WebSocketWrapper, id: String) {
        let offer_sdp = create_offer(peer).await;

        let msg = ntmy::Message::ConnectionRequest {
            id,
            session_info: offer_sdp.as_bytes().to_vec(),
        };

        let mut ready_state_change = signaling.state_change();
        loop {
            let state = ready_state_change.next().await;
            if state == ReadyState::Open {
                break;
            }
            paddle::println!("WS state changed to {state:?}");
            if state == ReadyState::Closed {
                return;
            }
        }

        if let Err(err) = signaling.send(&bendy::serde::to_bytes(&msg).unwrap()) {
            paddle::println!("error sending message: {:?}", err);
        }
    }

    fn accept_ntmy_msg(&mut self, _state: &mut (), msg: ntmy::Message) {
        let id = msg.connection_id();
        if !self.peers.contains_key(id) {
            paddle::println!("No peer for id {id}");
            return;
        }
        let (peer, ref mut state) = self.peers.get_mut(id).unwrap();
        match msg {
            ntmy::Message::ConnectionRequest { id, session_info } => {
                let future =
                    Self::accept_peer_offer(peer.clone(), id, session_info, self.signaling.clone());
                wasm_bindgen_futures::spawn_local(async { future.await.unwrap() });
            }
            ntmy::Message::PeerResponse { id, session_info } => {
                let future = Self::accept_peer_answer(peer.clone(), session_info, id);
                wasm_bindgen_futures::spawn_local(async { future.await.unwrap() });
            }
            ntmy::Message::IncrementalInfo { id, extra_info } => {
                let info: ntmy::WebRtcIncrementalInfo = bendy::serde::from_bytes(&extra_info)
                    .expect("not actually a WebRtcIncrementalInfo");
                let stringified = info.candidate;
                let mut rtc_init = RtcIceCandidateInit::new(&stringified);
                rtc_init.sdp_m_line_index(Some(info.sdp_m_line_index));
                rtc_init.sdp_mid(Some(&info.sdp_mid));
                match RtcIceCandidate::new(&rtc_init) {
                    Ok(candidate) => match state {
                        RemoteState::WaitingForResponse { buffered } => buffered.push(candidate),
                        RemoteState::Connected => {
                            self.add_ice_candidate(candidate, &id);
                        }
                    },
                    Err(err) => {
                        paddle::println!("Creating RtcPeerCandidate produced an error {err:?}.")
                    }
                }
            }
            ntmy::Message::Done { .. } => {
                paddle::println!("not expecting DONE");
                panic!();
            }
        }
    }

    fn add_ice_candidate(&mut self, candidate: RtcIceCandidate, id: &str) {
        if let Some((peer, _state)) = self.peers.get(id) {
            let _promise = peer
                .add_ice_candidate_with_opt_rtc_ice_candidate(Some(&candidate))
                .catch(&self.on_err_add_ice_candidate);
        } else {
            paddle::println!("No stored connection for id {id}");
        }
    }

    fn forward_ntmy_msg(&mut self, _state: &mut (), Forward(msg): Forward) {
        if let Err(e) = self.signaling.send(&bendy::serde::to_bytes(&msg).unwrap()) {
            paddle::println!("Failed to send NTMY message over web socket. {e:?}");
        }
    }

    fn on_remote_connected(&mut self, _state: &mut (), msg: &RemoteConnectedMsg) {
        if let Some((_peer, ref mut state)) = self.peers.get_mut(&msg.id) {
            match std::mem::replace(state, RemoteState::Connected) {
                RemoteState::WaitingForResponse { buffered } => {
                    for candidate in buffered {
                        self.add_ice_candidate(candidate, &msg.id)
                    }
                }
                RemoteState::Connected => {}
            }
        } else {
            paddle::println!("No stored connection for id {}", msg.id);
        }
    }

    async fn accept_peer_offer(
        peer: RtcPeerConnection,
        id: String,
        session_info: Vec<u8>,
        signaling: WebSocketWrapper,
    ) -> Result<(), JsValue> {
        let offer_sdp = std::str::from_utf8(&session_info).unwrap();
        let mut offer_obj = RtcSessionDescriptionInit::new(RtcSdpType::Offer);
        offer_obj.sdp(&offer_sdp);
        let srd_promise = peer.set_remote_description(&offer_obj);
        JsFuture::from(srd_promise).await?;
        paddle::share(RemoteConnectedMsg { id: id.clone() });

        let answer = JsFuture::from(peer.create_answer()).await?;
        let answer_sdp = Reflect::get(&answer, &JsValue::from_str("sdp"))?
            .as_string()
            .unwrap();

        let mut answer_obj = RtcSessionDescriptionInit::new(RtcSdpType::Answer);
        answer_obj.sdp(&answer_sdp);
        let sld_promise = peer.set_local_description(&answer_obj);
        JsFuture::from(sld_promise).await?;

        // Send our description to the signaling server to forward it to the peer
        let session_info = answer_sdp.into_bytes();
        let msg = ntmy::Message::PeerResponse { id, session_info };
        signaling.send(&bendy::serde::to_bytes(&msg).unwrap())?;
        // Now that we sent the local description, ICE candidate trickling begins.
        Ok(())
    }

    async fn accept_peer_answer(
        peer: RtcPeerConnection,
        session_info: Vec<u8>,
        id: String,
    ) -> Result<(), JsValue> {
        let answer_sdp = std::str::from_utf8(&session_info).unwrap();
        let mut answer_obj = RtcSessionDescriptionInit::new(RtcSdpType::Answer);
        answer_obj.sdp(answer_sdp);
        let srd_promise = peer.set_remote_description(&answer_obj);
        JsFuture::from(srd_promise).await?;
        paddle::send::<_, SignalingServerConnection>(RemoteConnectedMsg { id });
        Ok(())
    }
}

async fn create_offer(peer: RtcPeerConnection) -> String {
    let offer = JsFuture::from(peer.create_offer()).await.unwrap();
    let offer_sdp = Reflect::get(&offer, &JsValue::from_str("sdp"))
        .unwrap()
        .as_string()
        .unwrap();

    let mut offer_obj = RtcSessionDescriptionInit::new(RtcSdpType::Offer);
    offer_obj.sdp(&offer_sdp);
    let sld_promise = peer.set_local_description(&offer_obj);
    JsFuture::from(sld_promise).await.unwrap();
    offer_sdp
}

fn ice_candidate_trickling_callback(id: String) -> Closure<dyn FnMut(RtcPeerConnectionIceEvent)> {
    Closure::<dyn FnMut(_)>::new(move |ev: RtcPeerConnectionIceEvent| {
        if let Some(candidate) = ev.candidate() {
            let candidate_string = candidate.candidate();
            if candidate_string.is_empty() {
                return;
            }
            let info = ntmy::WebRtcIncrementalInfo {
                candidate: candidate_string,
                sdp_m_line_index: candidate.sdp_m_line_index().unwrap(),
                sdp_mid: candidate.sdp_mid().unwrap(),
            };

            let ntmy_msg = ntmy::Message::IncrementalInfo {
                id: id.clone(),
                extra_info: bendy::serde::to_bytes(&info).unwrap(),
            };
            paddle::send::<_, SignalingServerConnection>(Forward(ntmy_msg));
        }
    })
}

fn init_data_channel(
    data_channel: RtcDataChannel,
    mut on_msg: impl FnMut(&RtcDataChannel, MessageEvent) + 'static,
    mut on_open: impl FnMut(&RtcDataChannel) + 'static,
) {
    let data_channel_clone = data_channel.clone();
    let onmessage =
        Closure::<dyn FnMut(_)>::new(move |ev: MessageEvent| on_msg(&data_channel_clone, ev));
    data_channel.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
    onmessage.forget(); // TODO: avoid memory leak

    let data_channel_clone = data_channel.clone();
    let onopen = Closure::<dyn FnMut()>::new(move || on_open(&data_channel_clone));
    data_channel.set_onopen(Some(onopen.as_ref().unchecked_ref()));
    onopen.forget(); // TODO: avoid memory leak
}

impl PeerConnection {
    pub(crate) fn send(&self, data: &[u8]) -> Result<(), JsValue> {
        self.channel.send_with_u8_array(data)
    }
}

// TODO: Dropping should be handled properly
impl Drop for SignalingServerConnection {
    fn drop(&mut self) {
        // The handlers owned by this object are still registered, if they get
        // called it will cause panics. Also, same problem is inherited from the
        // websocket wrapper owned by the peer connection.
        paddle::println!("WARN: PeerConnection just dropped, this may cause other panics")
    }
}
