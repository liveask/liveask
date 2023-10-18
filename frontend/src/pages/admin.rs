use shared::{GetUserInfo, UserInfo};
use wasm_bindgen::UnwrapThrowExt;
use web_sys::HtmlInputElement;
use worker2::pwd_hash;
use yew::prelude::*;

use crate::fetch;

use super::BASE_API;

enum AdminState {
    RequestingInfo,
    LoggedIn(UserInfo),
    NotLoggedIn,
}

pub struct AdminLogin {
    name: String,
    pwd: String,
    state: AdminState,
}

#[derive(Debug)]
pub enum Input {
    Name,
    Pwd,
}

#[allow(clippy::empty_structs_with_brackets)]
#[derive(Clone, Debug, Eq, PartialEq, Properties)]
pub struct AdminLoginProps;

pub enum Msg {
    Login,
    LogOut,
    UserInfoResult(GetUserInfo),
    LoginResult(bool),
    InputChange(Input, InputEvent),
}
impl Component for AdminLogin {
    type Message = Msg;
    type Properties = AdminLoginProps;

    fn create(ctx: &Context<Self>) -> Self {
        request_user_info(ctx.link());

        Self {
            name: String::new(),
            pwd: String::new(),
            state: AdminState::RequestingInfo,
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
                    let res = fetch::admin_login(BASE_API, name, pwd_hash(&pwd)).await;

                    match res {
                        Ok(()) => {
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
            Msg::LogOut => {
                log::info!("log out");
                self.state = AdminState::RequestingInfo;
                request_logout(ctx.link());
                true
            }
            Msg::LoginResult(_) => {
                self.name.clear();
                self.pwd.clear();

                self.state = AdminState::RequestingInfo;
                request_user_info(ctx.link());

                true
            }
            Msg::UserInfoResult(res) => {
                if let Some(user) = res.user {
                    self.state = AdminState::LoggedIn(user);
                } else {
                    self.state = AdminState::NotLoggedIn;
                }

                true
            }
        }
    }

    #[allow(clippy::if_not_else)]
    fn view(&self, ctx: &Context<Self>) -> Html {
        match &self.state {
            AdminState::NotLoggedIn => self.view_login(ctx),
            AdminState::RequestingInfo => Self::view_waiting(),
            AdminState::LoggedIn(user) => Self::view_logged_in(ctx, user),
        }
    }
}

impl AdminLogin {
    #[allow(clippy::if_not_else)]
    fn view_login(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class="newevent-bg">
                <div class="title">
                    {"Admin Login"}
                </div>
                <div class="form">
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

            </div>
        }
    }

    fn view_logged_in(ctx: &Context<Self>, user: &UserInfo) -> Html {
        html! {
            <div class="newevent-bg">
                <div class="title">
                    {"Admin Login"}
                </div>

                <div class="form">
                    <p>{format!("Logged in as: '{}'",user.name)}</p>
                    <p>{format!("expires: {} min",user.expires.as_secs().saturating_div(60))}</p>

                    <button
                        class="button-finish"
                        onclick={ctx.link().callback(|_| Msg::LogOut)}>
                        {"logout"}
                    </button>
                </div>
            </div>
        }
    }

    fn view_waiting() -> Html {
        html! {
            <div class="newevent-bg">
                <div class="title">
                    {"Admin Login"}
                </div>
                <div class="form">
                    {"Waiting..."}
                </div>
            </div>
        }
    }
}

fn request_user_info(link: &html::Scope<AdminLogin>) {
    link.send_future(async move {
        match fetch::fetch_user(BASE_API).await {
            Err(res) => {
                log::error!("fetch_user error: {:?}", res);
                Msg::UserInfoResult(GetUserInfo { user: None })
            }
            Ok(res) => Msg::UserInfoResult(res),
        }
    });
}

fn request_logout(link: &html::Scope<AdminLogin>) {
    link.send_future(async move {
        if let Err(res) = fetch::admin_logout(BASE_API).await {
            log::error!("admin_logout error: {:?}", res);
        }

        Msg::UserInfoResult(GetUserInfo { user: None })
    });
}
