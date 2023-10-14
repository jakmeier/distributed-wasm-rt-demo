use std::rc::Rc;

use wasm_bindgen::prelude::*;
use web_sys::{BinaryType, ErrorEvent, MessageEvent, WebSocket};

#[derive(Clone)]
pub(crate) struct WebSocketWrapper {
    ws: WebSocket,
    /// closure called on each message
    ///
    /// must be stored here to give it a lifetime tied to the connection
    _onmessage: Rc<Closure<dyn FnMut(MessageEvent)>>,
    /// closure called on error events of the underlying web socket
    ///
    /// must be stored here to give it a lifetime tied to the connection
    _onerror: Rc<Closure<dyn FnMut(ErrorEvent)>>,
}

impl WebSocketWrapper {
    pub fn new(url: &str, onmessage: Rc<Closure<dyn FnMut(MessageEvent)>>) -> Self {
        let ws = WebSocket::new(url).unwrap();
        ws.set_binary_type(BinaryType::Arraybuffer);
        ws.set_onmessage(Some(onmessage.as_ref().as_ref().unchecked_ref()));

        // print error to console for ease of debugging
        let onerror = Closure::<dyn FnMut(_)>::new(move |e: ErrorEvent| {
            paddle::println!("SignalingServerConnection(error event): {e:?}");
            web_sys::console::log_1(&e);
        });
        ws.set_onerror(Some(onerror.as_ref().unchecked_ref()));

        WebSocketWrapper {
            ws,
            _onmessage: onmessage,
            _onerror: Rc::new(onerror),
        }
    }

    pub fn send(&self, msg: &[u8]) -> Result<(), JsValue> {
        self.ws.send_with_u8_array(msg)
    }

    pub fn state_change(&self) -> ReadyStateChange {
        ReadyStateChange::new(self.ws.clone())
    }
}

pub(crate) struct ReadyStateChange {
    ws: WebSocket,
    current_state: Option<ReadyState>,
    waker_fn: Option<Rc<Closure<dyn FnMut()>>>,
}

impl ReadyStateChange {
    fn new(ws: WebSocket) -> Self {
        ReadyStateChange {
            current_state: None,
            waker_fn: None,
            ws,
        }
    }

    /// Syntactic sugar to replace `(&mut future).await` with `future.next().await`
    pub fn next(&mut self) -> &mut Self {
        self
    }
}

impl std::future::Future for &mut ReadyStateChange {
    type Output = ReadyState;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let new_state = self.ws.ready_state().try_into().unwrap();
        if self.current_state == Some(new_state) {
            self.waker_fn.get_or_insert_with(|| {
                let waker = cx.waker().clone();
                let waker_closure = Closure::<dyn FnMut()>::new(move || {
                    waker.wake_by_ref();
                });
                Rc::new(waker_closure)
            });
            let callback = Rc::clone(self.waker_fn.as_ref().unwrap());
            match new_state {
                ReadyState::Connecting => {
                    self.ws
                        .set_onopen(Some(callback.as_ref().as_ref().unchecked_ref()));
                }
                ReadyState::Open | ReadyState::Closing => {
                    self.ws
                        .set_onclose(Some(callback.as_ref().as_ref().unchecked_ref()));
                }
                ReadyState::Closed => {
                    // ain't gonna change from here
                    return std::task::Poll::Ready(new_state);
                }
            }
            std::task::Poll::Pending
        } else {
            self.current_state = Some(new_state);
            std::task::Poll::Ready(new_state)
        }
    }
}

#[repr(u16)]
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub(crate) enum ReadyState {
    /// Socket has been created. The connection is not yet open.
    Connecting = 0,
    /// The connection is open and ready to communicate.
    Open = 1,
    /// The connection is in the process of closing.
    Closing = 2,
    /// The connection is closed or couldn't be opened.
    Closed = 3,
}

impl TryFrom<u16> for ReadyState {
    type Error = ();
    fn try_from(value: u16) -> Result<Self, Self::Error> {
        let state = match value {
            0 => Self::Connecting,
            1 => Self::Open,
            2 => Self::Closing,
            3 => Self::Closed,
            _ => return Err(()),
        };
        assert_eq!(value, state as u16, "bug: wrong parsing of WS read state");
        Ok(state)
    }
}
