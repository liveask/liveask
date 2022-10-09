use yew::prelude::*;

#[derive(Clone, Debug, Eq, PartialEq, Properties)]
pub struct PrivacyProps;

pub struct Privacy {}
impl Component for Privacy {
    type Message = ();
    type Properties = PrivacyProps;

    fn create(_: &Context<Self>) -> Self {
        Self {}
    }

    fn view(&self, _ctx: &Context<Self>) -> Html {
        let twitter_svg = include_str!("../../inline-assets/privacy.html");
        let div = gloo_utils::document().create_element("div").unwrap();
        div.set_inner_html(twitter_svg);
        let node = Html::VRef(div.into());

        html! {
            {node}
        }
    }
}
