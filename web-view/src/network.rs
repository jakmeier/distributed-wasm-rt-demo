use std::collections::HashMap;

use paddle::quicksilver_compat::Color;
use paddle::{Frame, FrameHandle, Rectangle, TextBoard, UiElement};
use wasm_bindgen::JsCast;
use web_sys::{Element, HtmlInputElement, MessageEvent, RtcDataChannel};

use crate::bottom_tabs::Tabs;
use crate::webrtc_signaling::{PeerConnection, SignalingServerConnection};
use crate::{palette, Main, PADDING, SCREEN_H, SCREEN_W, SECONDARY_X, SECONDARY_Y};

const BACKGROUND: Color = palette::NEUTRAL_DARK;
const BUTTON_COLOR: Color = palette::MAIN;
const PEER_COLOR: Color = palette::NEUTRAL;
const PEER_CONNECTED_COLOR: Color = palette::SHADE;

/// Shows connected peers and allows connecting with more.
pub(crate) struct NetworkView {
    html_active: bool,
    html: Element,
    new_id_field: HtmlInputElement,
    button: UiElement,
    text: UiElement,
    peers: HashMap<String, Peer>,
}

/// Network tab representation of a peer
struct Peer {
    connection: PeerConnection,
    ui: UiElement,
}

/// Event emitted when a peer connection has been established.
pub(crate) struct NewPeerEstablishedConnectionMsg(String);

/// Request to open a new peer, sent on button click.
#[derive(Clone)]
struct OpenNewPeerConnectionMsg;

/// Send some serialized data to all peers.
struct BroadcastMsg(Vec<u8>);

impl NetworkView {
    pub(crate) fn init() -> FrameHandle<Self> {
        // Connect WebSocket to signaling server for setting up connections later.
        crate::webrtc_signaling::SignalingServerConnection::start();

        let doc = web_sys::window().unwrap().document().unwrap();
        let html = doc.create_element("div").unwrap();
        let new_id_field = paddle::html::text_input("new_peer_id");
        new_id_field.set_value(&generate_key());
        html.append_child(&new_id_field).unwrap();

        let mut text = UiElement::new(
            Rectangle::new((100, 100), (Self::WIDTH - 200, 100)),
            palette::NEUTRAL,
        )
        .with_text("Match the ID above  on two devices and click below to connect.".to_owned())
        .unwrap()
        .with_text_alignment(paddle::FitStrategy::Center)
        .unwrap();
        text.add_text_css("color", palette::CSS_FONT_DARK);

        let button = crate::button(
            Rectangle::new((100, 250), (Self::WIDTH - 200, 100)),
            BUTTON_COLOR,
            OpenNewPeerConnectionMsg,
            "Find Peer".to_owned(),
            50.0,
        );

        let data = Self {
            html_active: false,
            html,
            new_id_field,
            button,
            peers: HashMap::new(),
            text,
        };
        let handle = paddle::register_frame_no_state(data, (SECONDARY_X, SECONDARY_Y));
        handle
            .activity()
            .set_status(paddle::nuts::LifecycleStatus::Inactive);
        handle.listen(Self::request_new_connection);
        handle.listen(Self::connected);
        handle.register_receiver(Self::broadcast);
        handle
    }

    fn request_new_connection(&mut self, _state: &mut (), _msg: &OpenNewPeerConnectionMsg) {
        if self.peers.len() > 0 {
            // This limitation is necessary because a network of three peers or
            // more is not handled properly.
            // Rendered parts (and control messages) would have to be forwarded
            // to all peers without duplication. This requires some knowledge
            // about the topology, or some sort of duplication detection. None
            // of this is super interesting but requires some work.
            // For the demo, two peers are sufficient to show the necessary
            // points, so let's avoid the extra work.
            TextBoard::display_error_message("Only 1 peer supported.".to_owned()).unwrap();
            return;
        }
        let id = self.new_id_field.value();
        if self.peers.contains_key(&id) {
            paddle::TextBoard::display_error_message(format!(
                "Already has peer with id {id}, please use a different id."
            ))
            .unwrap();
        } else {
            let connection =
                SignalingServerConnection::new_connection(id.clone(), on_open, on_message);
            let i = self.peers.len();
            let area = Rectangle::new((Self::WIDTH / 4, 380 + 110 * i), (Self::WIDTH / 2, 100));
            let mut ui = UiElement::new(area, PEER_COLOR)
                .with_text(id.clone())
                .unwrap()
                .with_text_alignment(paddle::FitStrategy::Center)
                .unwrap();
            ui.add_text_css("color", palette::CSS_FONT_DARK);
            self.peers.insert(id, Peer { connection, ui });
            self.text
                .set_text(Some("Connecting...".to_owned()))
                .unwrap();
        }
    }

    /// paddle event listener
    pub(crate) fn connected(&mut self, _state: &mut (), msg: &NewPeerEstablishedConnectionMsg) {
        let id = &msg.0;
        if let Some(peer) = self.peers.get_mut(id) {
            peer.ui.set_paint(PEER_CONNECTED_COLOR);
            self.text.set_paint(PEER_CONNECTED_COLOR);
            self.text.set_text(Some("Connected".to_owned())).unwrap();
        } else {
            paddle::println!("got connection for {id} without a stored peer object");
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
            peer.connection.send(&data).unwrap();
        }
    }
}

impl Frame for NetworkView {
    type State = ();

    const WIDTH: u32 = SCREEN_W - 2 * PADDING;
    const HEIGHT: u32 = SCREEN_H - Main::HEIGHT - Tabs::HEIGHT - 2 * PADDING;

    fn draw(
        &mut self,
        _state: &mut Self::State,
        canvas: &mut paddle::DisplayArea,
        _timestamp: f64,
    ) {
        canvas.draw(&Self::area(), &BACKGROUND);
        if !self.html_active {
            canvas.add_html(self.html.clone().into());
            self.html_active = true;
        }
        self.button.draw(canvas);
        self.text.draw(canvas);

        for peer in self.peers.values() {
            peer.ui.draw(canvas);
        }
    }

    fn pointer(&mut self, _state: &mut Self::State, event: paddle::PointerEvent) {
        self.button.pointer(event);
    }

    fn enter(&mut self, _state: &mut Self::State) {
        self.button.active();
        self.text.active();
        for peer in self.peers.values() {
            peer.ui.active();
        }
    }

    fn leave(&mut self, _state: &mut Self::State) {
        self.button.inactive();
        self.text.inactive();
        for peer in self.peers.values() {
            peer.ui.inactive();
        }
    }
}

/// Entry point for new messages arriving through the WebRTC channel.
fn on_message(_data_channel: &RtcDataChannel, id: &str, ev: MessageEvent) {
    if let Some(message) = ev.data().as_string() {
        // strings can be used for debugging
        paddle::println!("onmessage({id}): {:?}", message);
    }
    // Handling Blobs directly is most efficient and works in FF. In fact, it's
    // the default in FF.
    else if let Some(blob) = ev.data().dyn_into::<web_sys::Blob>().ok() {
        let future = async {
            match crate::p2p_proto::Message::from_blob(blob).await {
                Ok(msg) => paddle::share(msg),
                Err(e) => paddle::println!("failed to parse received message: {e:?}"),
            }
        };
        wasm_bindgen_futures::spawn_local(future);
    }
    // Chrome hasn't implemented blobs on RTC data channels, so we have to use
    // ArrayBuffer instead. FF also implements that, so I should probably just
    // set the channel binaryType to ArrayBuffer and always use that. But I
    // started with FF, so I'll stubbornly keep both implementations.
    else if let Some(array_buffer) = ev.data().dyn_into::<js_sys::ArrayBuffer>().ok() {
        match crate::p2p_proto::Message::from_array(array_buffer) {
            Ok(msg) => paddle::share(msg),
            Err(e) => paddle::println!("failed to parse received message: {e:?}"),
        }
    } else {
        paddle::println!(
            "unexpected message type received: {}",
            ev.data().js_typeof().as_string().unwrap()
        );
    }
}

/// Entry point for new WebRTC connections opening.
fn on_open(_data_channel: &RtcDataChannel, id: &str) {
    paddle::share(NewPeerEstablishedConnectionMsg(id.to_owned()));
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

    format!(
        "{}-{}-{}-{}",
        ADVERBS[random_bytes[0] as usize % ADVERBS.len()],
        ADJECTIVES[random_bytes[1] as usize % ADJECTIVES.len()],
        NOUNS[random_bytes[2] as usize % NOUNS.len()],
        random_bytes[3],
    )
}

// Lists of child-friendly words (according to ChatGPT)
#[rustfmt::skip]
const ADJECTIVES: [&str; 76] = [
    "bold", "boppy", "bouncy", "brave", "bright", "cheery", "chicky", "clappy", "colorly", "cozy",
    "crazy", "cuddly", "curious", "dandy", "daring", "dingle", "dizzy", "dreamy", "easy", "fair",
    "fancy", "fluffy", "fondly", "frisky", "funny", "fuzzy", "giddy", "giggly", "happy", "honest",
    "honey", "hoppie", "humming", "jitter", "jolly", "jovial", "jungle", "kind", "kindly",
    "lively", "lolly", "loud", "low", "lucky", "mellow", "nervous", "noisy", "perky", "playful",
    "plump", "puffy", "quick", "quiet", "quirky", "shyly", "silly", "smiley", "sneaky", "soft",
    "sparky", "spunky", "squeaky", "sunny", "swift", "tasty", "vividly", "wacky", "whimsy",
    "wiggly", "wiggy", "wise", "witty", "wobbly", "zappy", "zesty", "zippy",
];

#[rustfmt::skip]
const NOUNS: [&str; 93] = [
    "alpaca", "badger", "bee", "beetle", "bottle", "bunny", "butterfly", "cat", "cactus", "cheetah",
    "cherry", "chicken", "chinchilla", "chipmunk", "clouds", "cookie", "corgi", "crystal", "cupcake",
    "dancer", "diamond", "dog", "dolphin", "donkey", "doodle", "dragon", "fairy", "fish", "flower",
    "fluffy", "friend", "gazelle", "giggles", "giraffe", "gopher", "hamster", "hedgehog", "honey",
    "hum", "igloo", "jaguar", "jelly", "kangaroo", "kitten", "kiwi", "ladybug", "lizard",
    "lobster", "lollipop", "magic", "monkey", "muffin", "octopus", "otter", "panda", "pandas",
    "parakeet", "parrot", "pebble", "penguin", "pigeon", "platypus", "poodle", "pumpkin", "puppies",
    "puppy", "rabbit", "raccoon", "rainbow", "robot", "seagull", "seahorse", "skunk", "snail",
    "sparkle", "squirrel", "starfish", "sunshine", "teddy", "tiger", "turtle", "unicorn",
    "waffle", "whisker", "wombat", "year", "yeti", "zebra", "zero", "zombie", "zone", "zoo", "zulu"
];

#[rustfmt::skip]
const ADVERBS: [&str; 95] = [
    "bitter", "boldly", "boppy", "bouncy", "bravely", "bright", "brightly", "bunny", "cheerly",
    "cheery", "chicky", "chucky", "clappy", "colorly", "cozy", "crazy", "cuddly", "curious",
    "dandy", "daring", "dearly", "dingle", "dizzy", "dreamy", "easily", "fairly", "fancy",
    "fluffy", "fondly", "freely", "frisky", "funny", "fuzzy", "gaily", "gently", "giddy", "giga",
    "giggly", "happily", "happy", "hastily", "honest", "honey", "hoppie", "humming", "jitter",
    "jolly", "jovial", "jungle", "kindly", "lively", "lolly", "loudly", "lucky", "madly", "mellow",
    "merrily", "nervous", "noisy", "perky", "playful", "plump", "politely", "puffy", "quickly",
    "quietly", "quirky", "safely", "sharply", "shyly", "silly", "slowly", "smiley", "snappy",
    "sneaky", "softly", "sorely", "sparky", "spunky", "squeaky", "sunny", "super", "swiftly",
    "tasty", "vividly", "wacky", "whimsy", "wiggly", "wiggy", "wisely", "witty", "wobbly", "zappy",
    "zesty", "zippy",
];
