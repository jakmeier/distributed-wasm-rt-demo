use std::{net::SocketAddr, ops::ControlFlow, sync::Arc};

use axum::{
    extract::{
        ws::{Message as WsMsg, WebSocket, WebSocketUpgrade},
        State,
    },
    response::Response,
    routing::get,
    Router,
};

use futures_util::{stream::SplitSink, SinkExt};
use ntmy::Message as NtmyMsg;

#[macro_use]
extern crate log;

#[derive(Default)]
struct SessionBroker {
    /// the first peer to send a connection request
    alice: chashmap::CHashMap<String, Session>,
    /// the second peer to send a message
    bob: chashmap::CHashMap<String, Session>,
}

struct Session {
    tx: SplitSink<WebSocket, WsMsg>,
    session_info: Vec<u8>,
    incremental_info: Vec<Vec<u8>>,
}

/// State of connection
enum Role {
    /// The first peer to send a connection request.
    Alice,
    /// The second peer to send a message.
    Bob,
    /// While the role is still undecided, holds the peer writer socket.
    Undecided { tx: SplitSink<WebSocket, WsMsg> },
    /// Role decision imminent, socket has been removed already.
    Deciding,
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let app = Router::new()
        .route("/ws", get(handler))
        .with_state(Arc::new(SessionBroker::default()));

    let addr = SocketAddr::from(([0, 0, 0, 0], 80));

    println!("Starting signaling server");
    info!("INFO is enabled");
    debug!("DEBUG is enabled");
    trace!("TRACE is enabled");

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn handler(ws: WebSocketUpgrade, State(state): State<Arc<SessionBroker>>) -> Response {
    debug!("receiving upgradable socket");
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, broker: Arc<SessionBroker>) {
    use futures_util::stream::StreamExt;
    debug!("receiving socket");
    let (tx, mut rx) = socket.split();
    // role object gives a bit of preserved per-connection context to the async loop
    let mut role = Role::Undecided { tx };
    while let Some(msg) = rx.next().await {
        debug!("received message");
        if let ControlFlow::Break(_) = handle_ws_msg(msg, &broker, &mut role).await {
            return;
        }
    }
}

async fn handle_ws_msg(
    msg: Result<WsMsg, axum::Error>,
    broker: &Arc<SessionBroker>,
    role: &mut Role,
) -> ControlFlow<()> {
    match msg {
        Ok(WsMsg::Binary(binary)) => {
            if let Ok(ntmy) = bendy::serde::from_bytes::<NtmyMsg>(&binary) {
                let res = broker.handle_ntmy(role, ntmy).await;
                if res.is_err() {
                    debug!("tx client disconnected");
                    return ControlFlow::Break(());
                }
            } else {
                warn!("invalid binary WS message received");
            }
        }
        Ok(WsMsg::Close(_)) => {
            debug!("closing by client request");
            return ControlFlow::Break(());
        }
        Ok(WsMsg::Text(txt)) => {
            warn!("invalid WS message type received: `{txt}`");
        }
        Ok(WsMsg::Ping(_)) => {
            warn!("invalid WS message type received: ping");
        }
        Ok(WsMsg::Pong(_)) => {
            warn!("invalid WS message type received: pong");
        }

        Err(_) => {
            debug!("rx client disconnected");
            return ControlFlow::Break(());
        }
    }
    ControlFlow::Continue(())
}

impl SessionBroker {
    async fn handle_ntmy(
        &self,
        role: &mut Role,
        msg: NtmyMsg,
    ) -> Result<ControlFlow<()>, axum::Error> {
        match msg {
            NtmyMsg::ConnectionRequest { id, session_info } => {
                let prev_role = std::mem::replace(role, Role::Deciding);
                let Role::Undecided { tx } = prev_role else {
                    // Someone already picked up this socket. Receiving a second
                    // connection request from the same socket (maybe with a
                    // different id) is possible but not valid.
                    warn!("socket for id `{id}` has already been picked up");
                    return Ok(ControlFlow::Continue(()));

                };
                if let Some(alice) = self.alice.get(&id) {
                    let alice_session_info = alice.session_info.clone();
                    let incremental_info = alice.incremental_info.clone();
                    // release lock
                    drop(alice);
                    *role = Role::Bob;
                    self.connect_bob(&id, tx, alice_session_info, incremental_info, session_info)
                        .await?;
                } else {
                    // Only alice here for now.
                    *role = Role::Alice;
                    debug!("store waiting connection offer");
                    self.alice.insert(id, Session::new(tx, session_info));
                }
            }
            NtmyMsg::PeerResponse { id, session_info } => {
                let Some(role_sessions) = self.peer_sessions(&role) else {
                    debug!("received PeerResponse for `{id}` but role was not known");
                    return Err(axum::Error::new("PeerResponse before ConnectionRequest"));
                };
                if let Some(mut session) = role_sessions.get_mut(&id) {
                    debug!("sending response message for connection with id `{id}`");
                    let out_msg = NtmyMsg::PeerResponse { id, session_info };
                    session.tx.send(encode(&out_msg)).await?;
                    // TODO: hang up? no
                } else {
                    warn!("received unexpected response for id `{id}`");
                }
            }
            NtmyMsg::IncrementalInfo { id, extra_info } => {
                let Some(role_sessions) = self.peer_sessions(&role) else {
                    debug!("received IncrementalInfo for `{id}` but role was not known");
                    return Err(axum::Error::new("IncrementalInfo before ConnectionRequest"));
                };
                if let Some(mut peer_session) = role_sessions.get_mut(&id) {
                    debug!("sending incremental info message for connection with id `{id}`");
                    let out_msg = NtmyMsg::IncrementalInfo { id, extra_info };

                    peer_session.tx.send(encode(&out_msg)).await?;
                } else {
                    // bob isn't here, yet, let's buffer the info
                    let Some(mut session) = self.alice.get_mut(&id) else {
                        warn!("received unexpected incremental info for id `{id}`");
                        return Ok(ControlFlow::Continue(()))
                    };
                    session.incremental_info.push(extra_info);
                }
            }
            NtmyMsg::Done { id } => {
                let Some(role_sessions) = self.sessions(role) else {
                    debug!("received DONE for `{id}` but role was not known");
                    return Err(axum::Error::new("DONE before ConnectionRequest"));
                };
                if let Some(mut _session) = role_sessions.get_mut(&id) {
                    debug!("should be hanging up for `{id}`");
                    // TODO: delete state in stored sessions?
                    // TODO: also terminate other end of connection
                    return Ok(ControlFlow::Break(()));
                } else {
                    warn!("received unexpected DONE for id `{id}`");
                }
            }
        }
        Ok(ControlFlow::Continue(()))
    }

    async fn connect_bob(
        &self,
        id: &String,
        tx: SplitSink<WebSocket, WsMsg>,
        alice_session_info: Vec<u8>,
        alice_extra_info: Vec<Vec<u8>>,
        bob_session_info: Vec<u8>,
    ) -> Result<ControlFlow<()>, axum::Error> {
        // Ensure this is the first Bob arrived.
        let mut conflict = false;
        self.bob.alter(id.clone(), |old| {
            if old.is_none() {
                Some(Session::new(tx, bob_session_info))
            } else {
                conflict = true;
                old
            }
        });
        if conflict {
            debug!("refusing a third peer trying to connect");
            return Ok(ControlFlow::Break(()));
        }

        // Alice is already waiting for Bob on this ID, so we can
        // respond to Bob with the stored information from Alice.
        let out_msg = NtmyMsg::ConnectionRequest {
            id: id.clone(),
            session_info: alice_session_info,
        };

        // Note: Locking here isn't ideal from a performance
        // perspective but sending first and inserting afterwards
        // would be bad from a client perspective, as they might get
        // a positive response when they actually got raced.
        // Refactoring could solve this more cleanly but I can't
        // find a way that doesn't make code more complicated or
        // verbose. Plus, I don't see this as an actual bottleneck.
        debug!("sending offer to waiting peer");
        let mut session = self.bob.get_mut(id).expect("just inserted");
        let tx = &mut session.tx;
        tx.send(encode(&out_msg)).await?;
        for extra_info in alice_extra_info {
            let out_msg = NtmyMsg::IncrementalInfo {
                id: id.clone(),
                extra_info,
            };
            tx.send(encode(&out_msg)).await?;
        }
        Ok(ControlFlow::Continue(()))
    }

    fn sessions(&self, role: &Role) -> Option<&chashmap::CHashMap<String, Session>> {
        match role {
            Role::Alice => Some(&self.alice),
            Role::Bob => Some(&self.bob),
            _other => None,
        }
    }

    fn peer_sessions(&self, role: &Role) -> Option<&chashmap::CHashMap<String, Session>> {
        role.peer().and_then(|r| self.sessions(&r))
    }
}

impl Session {
    fn new(tx: SplitSink<WebSocket, WsMsg>, session_info: Vec<u8>) -> Self {
        Session {
            tx,
            session_info,
            incremental_info: vec![],
        }
    }
}

fn encode(msg: &NtmyMsg) -> WsMsg {
    WsMsg::Binary(bendy::serde::to_bytes(msg).unwrap())
}

impl Role {
    fn peer(&self) -> Option<Self> {
        match self {
            Role::Alice => Some(Role::Bob),
            Role::Bob => Some(Role::Alice),
            _other => None,
        }
    }
}
