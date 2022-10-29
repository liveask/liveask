use gloo::timers::callback::Interval;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
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
    #[allow(dead_code)]
    ping_interval: Interval,
    #[allow(dead_code)]
    events: Box<dyn Bridge<EventAgent>>,
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
            ping_interval,
            events: EventAgent::bridge(Callback::noop()),
        }
    }

    fn update(&mut self, msg: Self::Message) {
        match msg {
            Msg::Connected => {
                log::info!("ws connected");

                self.connected = true;
                self.events.send(GlobalEvent::SocketStatus(true));
                self.respond_to_all(&WsResponse::Ready);
            }
            Msg::Disconnected => {
                self.events.send(GlobalEvent::SocketStatus(false));

                self.disconnect();
                self.respond_to_all(&WsResponse::Disconnected);
            }
            Msg::MessageReceived(res) => {
                // log::info!("ws msg: {:?}", res);
                self.respond_to_all(&WsResponse::Message(res));
            }
            Msg::Ping => {
                if self.connected {
                    log::info!("ws send ping");
                    self.ws.as_ref().map(|ws| ws.send_string("p"));
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
                log::info!("ws disconnect");
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
        self.connected = false;
        self.ws = None;
    }

    fn connect(&mut self) {
        log::info!("ws connect: {}", self.url);

        let ws_err_callback = self.link.callback(|_| Msg::Disconnected);
        let ws_close_callback = self.link.callback(|_| Msg::Disconnected);
        let ws_connected_callback = self.link.callback(|_| Msg::Connected);
        let ws_msg_callback = self.link.callback(Msg::MessageReceived);

        let mut client = wasm_sockets::EventClient::new(&self.url).unwrap();

        client.set_on_error(Some(Box::new(move |error| {
            log::info!("ws on_error: {:#?}", error);
            ws_err_callback.emit(());
        })));
        client.set_on_connection(Some(Box::new(
            move |_client: &wasm_sockets::EventClient| {
                ws_connected_callback.emit(());
            },
        )));
        client.set_on_close(Some(Box::new(move || {
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
