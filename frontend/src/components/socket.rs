use chrono::Duration;
use gloo::timers::callback::Interval;
use wasm_bindgen::UnwrapThrowExt;
use web_sys::CloseEvent;
use yew::{html::Scope, prelude::*, virtual_dom::AttrValue};

#[derive(Clone, Debug)]
pub enum SocketResponse {
    Connecting,
    Connected,
    Disconnected { reconnect: Option<Duration> },
    Message(String),
}

#[derive(Clone, Debug, PartialEq, Properties)]
pub struct SocketProperties {
    pub reconnect: bool,
    pub url: AttrValue,
    pub msg: Callback<SocketResponse>,
}

pub enum Msg {
    Ping,
    MessageReceived(String),
    Connected,
    Disconnected,
    Reconnect,
}

pub struct EventSocket {
    link: Scope<Self>,
    properties: SocketProperties,
    connected: bool,
    ws: Option<wasm_sockets::EventClient>,
    reconnect_interval: Option<(Duration, Interval)>,
    _ping_interval: Interval,
}
impl Component for EventSocket {
    type Message = Msg;
    type Properties = SocketProperties;

    fn create(ctx: &Context<Self>) -> Self {
        let ping_interval = {
            let link = ctx.link().clone();
            Interval::new(3000, move || link.send_message(Msg::Ping))
        };

        let mut new_self = Self {
            link: ctx.link().clone(),
            properties: ctx.props().clone(),
            connected: false,
            ws: None,
            reconnect_interval: None,
            _ping_interval: ping_interval,
        };

        new_self.connect();

        new_self
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Ping => {
                // log::info!("<EventSocket> update:ping");
                if self.connected {
                    self.ws.as_ref().map(|ws| ws.send_string("p"));
                }
            }
            Msg::MessageReceived(msg) => {
                // log::info!("<EventSocket> update:msg");
                self.emit(SocketResponse::Message(msg));
            }
            Msg::Connected => {
                // log::info!("<EventSocket> update:connected");
                self.connected = true;
                self.reconnect_interval = None;
                self.emit(SocketResponse::Connected);
            }
            Msg::Disconnected => {
                // log::info!("<EventSocket> update:disconnected");
                let do_reconnect = self.connected;

                self.disconnect();

                if do_reconnect {
                    let duration = self.set_reconnect();
                    self.emit(SocketResponse::Disconnected {
                        reconnect: Some(duration),
                    });
                } else {
                    self.emit(SocketResponse::Disconnected { reconnect: None });
                }
            }
            Msg::Reconnect => {
                if self.reconnect_interval.is_some() && !self.connected {
                    self.connect();
                }
            }
        }
        false
    }

    fn changed(&mut self, ctx: &Context<Self>) -> bool {
        self.properties = ctx.props().clone();

        if self.properties.reconnect {
            self.connect();
        }
        true
    }

    fn view(&self, _ctx: &Context<Self>) -> Html {
        html!()
    }

    fn destroy(&mut self, _ctx: &Context<Self>) {
        // log::info!("<EventSocket> destroy");
        self.disconnect();
    }
}

impl EventSocket {
    fn connect(&mut self) {
        if self.ws.is_some() {
            // log::warn!("<EventSocket> already started");
            return;
        }

        let url = self.properties.url.clone();

        // log::info!("<EventSocket> connect: {}", url);

        self.emit(SocketResponse::Connecting);

        let ws_close_callback = self.link.callback(|()| Msg::Disconnected);
        let ws_connected_callback = self.link.callback(|()| Msg::Connected);
        let ws_msg_callback = self.link.callback(Msg::MessageReceived);

        let mut client =
            wasm_sockets::EventClient::new(&url).expect_throw("error creating websocket");

        client.set_on_error(Some(Box::new(move |error| {
            log::error!("<EventSocket> on_error: {:#?}", error);
        })));
        client.set_on_connection(Some(Box::new(
            move |_client: &wasm_sockets::EventClient| {
                ws_connected_callback.emit(());
            },
        )));
        client.set_on_close(Some(Box::new(move |_event: CloseEvent| {
            // log::info!("<EventSocket> on_close: {}", event.reason());
            ws_close_callback.emit(());
        })));
        client.set_on_message(Some(Box::new(
            move |_client: &wasm_sockets::EventClient, message: wasm_sockets::Message| {
                if let wasm_sockets::Message::Text(txt) = message {
                    ws_msg_callback.emit(txt);
                } else {
                    log::error!(
                        "<EventSocket> error: unexpected messages type: {:?}",
                        message
                    );
                }
            },
        )));

        self.ws = Some(client);
    }

    fn disconnect(&mut self) {
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

        // log::info!("<EventSocket> set reconnect timeout: {}", duration);

        self.reconnect_interval = Some((duration, interval));

        duration
    }

    fn emit(&self, msg: SocketResponse) {
        self.properties.msg.emit(msg);
    }
}
