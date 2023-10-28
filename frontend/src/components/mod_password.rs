use yew::prelude::*;

pub enum Msg {
    EnablePasswordInput,
}

pub struct ModPassword;
impl Component for ModPassword {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {}
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::EnablePasswordInput => {
                log::info!("EnablePassword");
                false
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <button class="button-white" onclick={ctx.link().callback(|_|Msg::EnablePasswordInput)} >
                {"Password"}
            </button>
        }
    }
}
