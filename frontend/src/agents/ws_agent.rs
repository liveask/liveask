use chrono::Duration;
use gloo::timers::callback::Interval;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use wasm_bindgen::UnwrapThrowExt;
use yew::Callback;
use yew_agent::{Agent, AgentLink, Bridge, Bridged, HandlerId};

use crate::agents::GlobalEvent;

use super::EventAgent;

#[derive(Serialize, Deserialize, Debug)]
pub enum SocketInput {
    Connect(String),
    Disconnect,
    Reconnect,
}

#[derive(Debug)]
pub enum Msg {
    MessageReceived(String),
    Connected,
    Disconnected,
    Ping,
    Reconnect,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum WsResponse {
    Ready,
    Disconnected,
    Message(String),
}

pub struct WebSocketAgent {
    url: String,
    link: AgentLink<Self>,
    ws: Option<wasm_sockets::EventClient>,
    subscribers: HashSet<HandlerId>,
    connected: bool,
    _ping_interval: Interval,
    events: Box<dyn Bridge<EventAgent>>,
    reconnect_interval: Option<(Duration, Interval)>,
}

impl Agent for WebSocketAgent {
    type Reach = yew_agent::Context<Self>;
    type Message = Msg;
    type Input = SocketInput;
    type Output = WsResponse;

    fn create(link: AgentLink<Self>) -> Self {
        let ping_interval = {
            let link = link.clone();
            Interval::new(3000, move || link.send_message(Msg::Ping))
        };

        Self {
            url: String::new(),
            link,
            ws: None,
            subscribers: HashSet::new(),
            connected: false,
            _ping_interval: ping_interval,
            events: EventAgent::bridge(Callback::noop()),
            reconnect_interval: None,
        }
    }

    fn update(&mut self, msg: Self::Message) {
        match msg {
            Msg::Connected => {
                self.connected = true;
                self.reconnect_interval = None;
                self.events.send(GlobalEvent::SocketStatus {
                    connected: true,
                    timeout_secs: None,
                });
                self.respond_to_all(&WsResponse::Ready);
            }
            Msg::Disconnected => {
                let do_reconnect = self.connected;

                self.disconnect();
                self.respond_to_all(&WsResponse::Disconnected);

                if do_reconnect {
                    let duration = self.set_reconnect();

                    self.events.send(GlobalEvent::SocketStatus {
                        connected: false,
                        timeout_secs: Some(duration.num_seconds()),
                    });
                }
            }
            Msg::MessageReceived(res) => {
                self.respond_to_all(&WsResponse::Message(res));
            }
            Msg::Ping => {
                if self.connected {
                    self.ws.as_ref().map(|ws| ws.send_string("p"));
                }
            }
            Msg::Reconnect => {
                if self.reconnect_interval.is_some() && !self.connected {
                    self.connect();
                }
            }
        }
    }

    fn handle_input(&mut self, msg: Self::Input, _id: HandlerId) {
        match msg {
            SocketInput::Connect(url) => {
                self.url = url;
                self.connect();
            }
            SocketInput::Reconnect => {
                if self.connected {
                    log::warn!("still connected, wont reconnect");
                } else {
                    self.connect();
                }
            }
            SocketInput::Disconnect => {
                self.disconnect();
            }
        }
    }

    fn connected(&mut self, id: HandlerId) {
        self.subscribers.insert(id);

        if self.connected {
            self.respond(id, WsResponse::Ready);
        }
    }

    fn disconnected(&mut self, id: HandlerId) {
        self.subscribers.remove(&id);
    }

    fn destroy(&mut self) {
        log::info!("ws agent destroyed");
    }
}

impl WebSocketAgent {
    fn respond(&self, sub: HandlerId, response: WsResponse) {
        self.link.respond(sub, response);
    }

    fn respond_to_all(&self, response: &WsResponse) {
        for sub in &self.subscribers {
            self.respond(*sub, response.clone());
        }
    }

    fn disconnect(&mut self) {
        // log::info!("ws disconnect");

        self.connected = false;
        if let Some(client) = &mut self.ws {
            client.set_on_error(None);
            client.set_on_connection(None);
            //Note: doing this will lead to borrow panics
            // client.set_on_close(None);
            client.set_on_message(None);

            client.close().unwrap_throw();
        }
        self.ws = None;
    }

    fn set_reconnect(&mut self) -> Duration {
        //TODO: base duration on previous one
        let duration = Duration::seconds(4);

        let interval = {
            let link = self.link.clone();
            Interval::new(
                duration
                    .num_milliseconds()
                    .try_into()
                    .expect_throw("duration millis should always fit into u32"),
                move || {
                    link.send_message(Msg::Reconnect);
                },
            )
        };

        log::info!("ws set reconnect timeout: {}", duration);

        self.reconnect_interval = Some((duration, interval));

        duration
    }

    fn connect(&mut self) {
        log::info!("ws connect: {}", self.url);

        let ws_close_callback = self.link.callback(|_| Msg::Disconnected);
        let ws_connected_callback = self.link.callback(|_| Msg::Connected);
        let ws_msg_callback = self.link.callback(Msg::MessageReceived);

        let mut client =
            wasm_sockets::EventClient::new(&self.url).expect_throw("error creating websocket");

        client.set_on_error(Some(Box::new(move |_error| {
            // log::info!("ws on_error: {:#?}", error);
            log::info!("ws on_error");
        })));
        client.set_on_connection(Some(Box::new(
            move |_client: &wasm_sockets::EventClient| {
                ws_connected_callback.emit(());
            },
        )));
        client.set_on_close(Some(Box::new(move |_event| {
            log::info!("ws on_close");
            ws_close_callback.emit(());
        })));
        client.set_on_message(Some(Box::new(
            move |_client: &wasm_sockets::EventClient, message: wasm_sockets::Message| {
                if let wasm_sockets::Message::Text(txt) = message {
                    ws_msg_callback.emit(txt);
                } else {
                    log::error!("ws error: unexpected messages type: {:?}", message);
                }
            },
        )));

        self.ws = Some(client);
    }
}
