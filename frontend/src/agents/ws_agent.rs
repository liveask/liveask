use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use yew_agent::{Agent, AgentLink, HandlerId};

#[derive(Serialize, Deserialize, Debug)]
pub enum SocketInput {
    Connect(String),
}

#[derive(Debug)]
pub enum Msg {
    MessageReceived(String),
    Connected,
    Disconnected,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum WsResponse {
    Ready,
    Disconnected,
    Message(String),
}

pub struct WebSocketAgent {
    link: AgentLink<Self>,
    ws: Option<wasm_sockets::EventClient>,
    subscribers: HashSet<HandlerId>,
    connected: bool,
}

impl Agent for WebSocketAgent {
    type Reach = yew_agent::Context<Self>;
    type Message = Msg;
    type Input = SocketInput;
    type Output = WsResponse;

    fn create(link: AgentLink<Self>) -> Self {
        Self {
            link,
            ws: None,
            subscribers: HashSet::new(),
            connected: false,
        }
    }

    fn update(&mut self, msg: Self::Message) {
        match msg {
            Msg::Connected => {
                self.connected = true;
                self.respond_to_all(&WsResponse::Ready);
            }
            Msg::Disconnected => {
                self.connected = false;
                self.ws = None;

                self.respond_to_all(&WsResponse::Disconnected);
            }
            Msg::MessageReceived(res) => {
                // log::info!("ws msg: {:?}", res);
                self.respond_to_all(&WsResponse::Message(res));
            }
        }
    }

    fn handle_input(&mut self, msg: Self::Input, _id: HandlerId) {
        match msg {
            SocketInput::Connect(url) => {
                self.connect(&url);
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

    fn connect(&mut self, url: &str) {
        log::info!("ws connect: {}", url);

        let ws_err_callback = self.link.callback(|_| Msg::Disconnected);
        let ws_close_callback = self.link.callback(|_| Msg::Disconnected);
        let ws_connected_callback = self.link.callback(|_| Msg::Connected);
        let ws_msg_callback = self.link.callback(Msg::MessageReceived);

        let mut client = wasm_sockets::EventClient::new(url).unwrap();
        client.set_on_error(Some(Box::new(move |error| {
            log::error!("ws error: {:#?}", error);
            ws_err_callback.emit(());
        })));
        client.set_on_connection(Some(Box::new(
            move |_client: &wasm_sockets::EventClient| {
                // log::info!("{:#?}", client.status);
                ws_connected_callback.emit(());
            },
        )));
        client.set_on_close(Some(Box::new(move || {
            // log::info!("Connection closed");
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
