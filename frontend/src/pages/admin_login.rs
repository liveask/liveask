use wasm_bindgen::UnwrapThrowExt;
use web_sys::HtmlInputElement;
use yew::prelude::*;

use crate::{fetch, pwd::pwd_hash};

use super::BASE_API;

pub struct AdminLogin {
    name: String,
    pwd: String,
    logged_in: bool,
}

#[derive(Debug)]
pub enum Input {
    Name,
    Pwd,
}

#[derive(Clone, Debug, Eq, PartialEq, Properties)]
pub struct AdminLoginProps;

pub enum Msg {
    Login,
    LoginResult(bool),
    InputChange(Input, InputEvent),
}
impl Component for AdminLogin {
    type Message = Msg;
    type Properties = AdminLoginProps;

    fn create(_: &Context<Self>) -> Self {
        Self {
            name: String::new(),
            pwd: String::new(),
            logged_in: false,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::InputChange(input, c) => {
                match input {
                    Input::Name => {
                        let target: HtmlInputElement = c.target_dyn_into().unwrap_throw();
                        self.name = target.value();
                    }
                    Input::Pwd => {
                        let target: HtmlInputElement = c.target_dyn_into().unwrap_throw();
                        self.pwd = target.value();
                    }
                }

                true
            }
            Msg::Login => {
                let name = self.name.clone();
                let pwd = self.pwd.clone();

                ctx.link().send_future(async move {
                    let res = fetch::admin_login(BASE_API, name, pwd_hash(pwd)).await;

                    match res {
                        Ok(_) => {
                            log::info!("login ok");
                            Msg::LoginResult(true)
                        }
                        Err(e) => {
                            log::error!("login error: {e}");
                            Msg::LoginResult(false)
                        }
                    }
                });

                false
            }
            Msg::LoginResult(result) => {
                self.name.clear();
                self.pwd.clear();
                self.logged_in = result;
                true
            }
        }
    }

    #[allow(clippy::if_not_else)]
    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class="newevent-bg">
                <div class="title">
                    {"Admin Login"}
                </div>
                <div class="form" hidden={self.logged_in}>
                    <div class="newevent">
                        <div class="input-box">
                            <input
                                type="text"
                                // name="eventname"
                                placeholder="user name"
                                value={self.name.clone()}
                                maxlength="30"
                                // autocomplete="off"
                                required=true
                                oninput={ctx.link().callback(|input| Msg::InputChange(Input::Name,input))}/>
                        </div>

                        <div class="input-box">
                            <input
                                type="password"
                                name="pwd"
                                placeholder="password"
                                value={self.pwd.clone()}
                                oninput={ctx.link().callback(|input| Msg::InputChange(Input::Pwd,input))}/>
                        </div>
                    </div>
                    <button
                        class="button-finish"
                        // disabled={!self.can_create()}
                        onclick={ctx.link().callback(|_| Msg::Login)}>
                        {"login"}
                    </button>
                </div>
                <div class="form" hidden={!self.logged_in}>
                    {"Logged in"}
                </div>
            </div>
        }
    }
}
