use wasm_bindgen::UnwrapThrowExt;
use web_sys::HtmlInputElement;
use yew::prelude::*;

pub enum Msg {
    EnablePasswordInput,
    EditPassword,
    DisablePassword,
    InputChange(InputEvent),
    InputExit,
}

enum State {
    Disabled,
    InputUnconfirmed(String),
    Confirmed(String),
}

pub struct ModPassword {
    state: State,
    input: NodeRef,
}
impl Component for ModPassword {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            state: State::Disabled,
            input: NodeRef::default(),
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::EnablePasswordInput => {
                self.state = State::InputUnconfirmed(String::new());
                true
            }
            Msg::EditPassword => {
                self.state = State::InputUnconfirmed(self.current_value().to_string());
                true
            }
            Msg::DisablePassword => {
                self.state = State::Disabled;
                true
            }
            Msg::InputChange(e) => {
                let target: HtmlInputElement = e.target_dyn_into().unwrap_throw();
                self.state = State::InputUnconfirmed(target.value());
                true
            }
            Msg::InputExit => {
                self.state = State::Confirmed(self.current_value().to_string());
                true
            }
        }
    }

    fn rendered(&mut self, _ctx: &Context<Self>, _first_render: bool) {
        if let State::InputUnconfirmed(_) = self.state {
            self.input
                .cast::<HtmlInputElement>()
                .unwrap_throw()
                .focus()
                .unwrap_throw();
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let content = match &self.state {
            State::Disabled => Self::view_disabled(ctx),
            State::InputUnconfirmed(value) => self.view_input_unconfirmed(value, ctx),
            State::Confirmed(_) => Self::view_confirmed(ctx),
        };

        html! {
            <div class="password">
                {content}
            </div>
        }
    }
}

impl ModPassword {
    fn view_disabled(ctx: &Context<Self>) -> Html {
        html! {
            <button class="button-white" onclick={ctx.link().callback(|_|Msg::EnablePasswordInput)} >
                {"Password"}
            </button>
        }
    }

    fn view_input_unconfirmed(&self, current: &str, ctx: &Context<Self>) -> Html {
        let value = current.to_string();
        html! {
            <>
                <input
                    ref={self.input.clone()}
                    type="text"
                    placeholder="password"
                    {value}
                    oninput={ctx.link().callback(Msg::InputChange)}
                    onblur={ctx.link().callback(|_|Msg::InputExit)} />
                <img id="edit" src="/assets/pwd/pwd-edit.svg"/>
            </>
        }
    }

    fn view_confirmed(ctx: &Context<Self>) -> Html {
        html! {
            <>
                <div class="confirmed" onclick={ctx.link().callback(|_|Msg::EditPassword)}>
                    {"*****"}
                </div>
                <img id="delete" src="/assets/pwd/pwd-remove.svg" onmousedown={ctx.link().callback(|_|Msg::DisablePassword)} />
            </>
        }
    }

    fn current_value(&self) -> &str {
        match &self.state {
            State::InputUnconfirmed(value) | State::Confirmed(value) => value,
            State::Disabled => "",
        }
    }
}
