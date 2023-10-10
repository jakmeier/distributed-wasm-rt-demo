use std::collections::HashMap;

use paddle::quicksilver_compat::Color;
use paddle::{Frame, FrameHandle, Rectangle, UiElement};
use wasm_bindgen::JsCast;
use web_sys::{Element, HtmlInputElement, MessageEvent, RtcDataChannel};

use crate::webrtc_signaling::{PeerConnection, SignalingServerConnection};
use crate::{SCREEN_H, SCREEN_W};

const BUTTON_COLOR: Color = Color::new(0.5, 0.5, 0.5);
const PEER_COLOR: Color = Color::new(0.5, 0.5, 1.0);

/// Shows connected peers and allows connecting with more.
pub(crate) struct NetworkView {
    html_active: bool,
    html: Element,
    new_id_field: HtmlInputElement,
    button: UiElement,
    peers: HashMap<String, PeerConnection>,
    peer_ui: Vec<UiElement>,
}

pub(crate) struct NewPeerMsg;

#[derive(Clone)]
struct NewPeerConnectionMsg;

/// Send some serialized data to all peers.
struct BroadcastMsg(Vec<u8>);

impl NetworkView {
    pub(crate) fn init() -> FrameHandle<Self> {
        // Connect WebSocket to signaling server for setting up connections later.
        crate::webrtc_signaling::SignalingServerConnection::start();

        let doc = web_sys::window().unwrap().document().unwrap();
        let html = doc.create_element("div").unwrap();
        let label = doc.create_element("label").unwrap();
        label.set_inner_html("Connection ID (must match that of the peer)");
        let new_id_field = paddle::html::text_input("new_peer_id");
        new_id_field.set_value(&generate_key());
        html.append_child(&label).unwrap();
        html.append_child(&new_id_field).unwrap();

        let button = UiElement::new(
            Rectangle::new((100, 100), (Self::WIDTH - 200, 200)),
            BUTTON_COLOR,
        )
        .with_pointer_interaction(paddle::PointerEventType::PrimaryClick, NewPeerConnectionMsg)
        .with_rounded_corners(15.0)
        .with_text("Find Peer".to_owned())
        .unwrap()
        .with_text_alignment(paddle::FitStrategy::Center)
        .unwrap();

        let data = Self {
            html_active: false,
            html,
            new_id_field,
            button,
            peers: HashMap::new(),
            peer_ui: vec![],
        };
        let handle = paddle::register_frame_no_state(data, (0, 0));
        handle
            .activity()
            .set_status(paddle::nuts::LifecycleStatus::Inactive);
        handle.listen(Self::new_peer_connection);
        handle.register_receiver(Self::broadcast);
        handle
    }

    fn new_peer_connection(&mut self, _state: &mut (), _msg: &NewPeerConnectionMsg) {
        let id = self.new_id_field.value();
        if self.peers.contains_key(&id) {
            paddle::TextBoard::display_error_message("Already has peer.".to_owned()).unwrap();
        } else {
            let peer = SignalingServerConnection::new_connection(id.clone(), on_open, on_message);
            self.peers.insert(id.clone(), peer);

            let i = self.peer_ui.len();
            let area = Rectangle::new((100, 400 + 110 * i), (Self::WIDTH - 200, 100));
            self.peer_ui
                .push(UiElement::new(area, PEER_COLOR).with_text(id).unwrap());
        }
    }

    /// paddle event listener: forward png parts when they are produced
    pub(crate) fn new_png_part(&mut self, _state: &mut (), png: &crate::PngPart) {
        let msg = crate::p2p_proto::Message::RenderedPart(png.clone());
        let num_pixels = png.screen_area.width() as usize * png.screen_area.height() as usize;
        // Best effort pre-allocation: One byte per pixel, which is most likely
        // too much due to PNG compression. Hence it should avoid re-allocation
        // in most cases.
        broadcast_async(msg, Some(num_pixels));
    }

    /// Send a message to all peers.
    fn broadcast(&mut self, _state: &mut (), BroadcastMsg(data): BroadcastMsg) {
        for (_id, peer) in &self.peers {
            peer.send(&data).unwrap();
        }
    }
}

impl Frame for NetworkView {
    type State = ();

    const WIDTH: u32 = SCREEN_W;
    const HEIGHT: u32 = SCREEN_H;

    fn draw(
        &mut self,
        _state: &mut Self::State,
        canvas: &mut paddle::DisplayArea,
        _timestamp: f64,
    ) {
        canvas.draw(&Self::area(), &Color::INDIGO);
        if !self.html_active {
            canvas.add_html(self.html.clone().into());
            self.html_active = true;
        }
        self.button.draw(canvas);

        for peer in &self.peer_ui {
            peer.draw(canvas);
        }
    }

    fn pointer(&mut self, _state: &mut Self::State, event: paddle::PointerEvent) {
        self.button.pointer(event);
    }

    fn enter(&mut self, _state: &mut Self::State) {
        self.button.active();
        for peer in &self.peer_ui {
            peer.active();
        }
    }

    fn leave(&mut self, _state: &mut Self::State) {
        self.button.inactive();
        for peer in &self.peer_ui {
            peer.inactive();
        }
    }
}

/// Entry point for new messages arriving through the WebRTC channel.
fn on_message(_data_channel: &RtcDataChannel, ev: MessageEvent) {
    if let Some(message) = ev.data().as_string() {
        // strings can be used for debugging
        paddle::println!("onmessage: {:?}", message);
    } else if let Some(blob) = ev.data().dyn_into::<web_sys::Blob>().ok() {
        let future = async {
            match crate::p2p_proto::Message::from_blob(blob).await {
                Ok(msg) => paddle::share(msg),
                Err(e) => paddle::println!("failed to parse received message: {e:?}"),
            }
        };
        wasm_bindgen_futures::spawn_local(future);
    } else {
        paddle::println!(
            "unexpected message type received: {}",
            ev.data().js_typeof().as_string().unwrap()
        );
    }
}

/// Entry point for new WebRTC connections opening.
fn on_open(data_channel: &RtcDataChannel) {
    paddle::println!("sending a ping over rtc");
    data_channel.send_with_str("Ping from pc2.dc!").unwrap();
    paddle::share(NewPeerMsg);
}

/// Send a message to all connected peers.
pub(crate) fn broadcast_async(msg: crate::p2p_proto::Message, size_hint: Option<usize>) {
    let future = async move {
        let mut buf = if let Some(size) = size_hint {
            Vec::with_capacity(size)
        } else {
            Vec::new()
        };
        msg.serialize(&mut buf)
            .await
            .expect("failed to serialize message");
        paddle::send::<_, NetworkView>(BroadcastMsg(buf));
    };
    wasm_bindgen_futures::spawn_local(future);
}

fn generate_key() -> String {
    let mut random_bytes = [0; 4];
    web_sys::window()
        .unwrap()
        .crypto()
        .expect("no crypto on window")
        .get_random_values_with_u8_array(&mut random_bytes)
        .expect("failed to generate random numbers");

    let n = WORDS.len();
    format!(
        "{}-{}-{}-{}",
        WORDS[random_bytes[0] as usize % n],
        WORDS[random_bytes[1] as usize % n],
        WORDS[random_bytes[2] as usize % n],
        random_bytes[3],
    )
}

// List of 128 child-friendly words (according to ChatGPT)
#[rustfmt::skip]
const WORDS: [&str; 128] = [
    "apple", "banana", "candy", "cloud", "dance", "dream", "funny", "giggle", "happy", "jelly",
    "kitty", "laugh", "magic", "puppy", "quick", "smile", "sunny", "sweet", "teddy", "unicorn",
    "balloon", "bubble", "bouncy", "cookie", "cuddle", "ducky", "fairy", "fluffy", "hoppy", "jumpy",
    "lolly", "lucky", "marsh", "melon", "piggy", "poppy", "puddle", "skippy", "spark", "spotty", "sprinkle",
    "starry", "tickle", "turtle", "wiggly", "bunny", "comet", "cozy", "cuddly", "daisy", "dazzle",
    "funny", "hopper", "jolly", "kiddy", "lemon", "lucky", "monkey", "noodle", "paws", "pebble",
    "plump", "puffy", "skippy", "smiley", "snappy", "snuggle", "spotty", "squishy", "stripe", "sunny",
    "tiny", "twinkle", "whisker", "wiggle", "zippy", "bambo", "blinky", "boing", "boppy", "cherub",
    "chuckle", "dingle", "doozy", "dumby", "fuzzy", "giggly", "holly", "hooty", "jingle", "kooky",
    "little", "mellow", "nifty", "peppy", "perky", "pixie", "plinky", "polka", "quirks", "rascal",
    "skunky", "snappy", "sneezy", "snoopy", "sparky", "spunky", "sunny", "thumpy", "tipsy", "trilly",
    "twirly", "whimsy", "wiggly", "zappy", "zesty", "zippy", "ziggy", "bitty", "boppy", "cheery", "chicky",
    "clappy", "cuddle", "curly", "dandy", "doodle", "giggle",
];
