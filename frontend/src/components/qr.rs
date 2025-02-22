use wasm_bindgen::UnwrapThrowExt;
use yew::prelude::*;

#[derive(Clone, Debug, Eq, PartialEq, Properties)]
pub struct QrProps {
    pub url: AttrValue,
    pub dimensions: u32,
}

pub struct Qr {
    qr_image: String,
}

impl Component for Qr {
    type Message = ();
    type Properties = QrProps;

    fn create(ctx: &Context<Self>) -> Self {
        use qrcode::{EcLevel, QrCode, Version, render::svg};

        let dim = ctx.props().dimensions;

        let code =
            QrCode::with_version(ctx.props().url.to_string(), Version::Normal(6), EcLevel::M)
                .unwrap_throw();

        let qr_image = code
            .render()
            .min_dimensions(dim, dim)
            .dark_color(svg::Color("#000000"))
            .light_color(svg::Color("#ffffff"))
            .build();

        Self { qr_image }
    }

    fn view(&self, _ctx: &Context<Self>) -> Html {
        let div = gloo_utils::document().create_element("div").unwrap_throw();
        div.set_inner_html(&self.qr_image);
        div.class_list().add_1("qrcode").unwrap_throw();

        let qr_svg = Html::VRef(div.into());

        html! { { qr_svg } }
    }
}
