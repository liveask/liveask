use wasm_bindgen::UnwrapThrowExt;
use yew::prelude::*;
use yew_router::prelude::*;

use crate::{fetch, routes::Route, GIT_BRANCH, VERSION_STR};

pub struct Footer {
    api_version: Option<String>,
}
pub enum Msg {
    CreateEvent,
    Privacy,
    Admin,
    VersionReceived(Option<String>),
}
impl Component for Footer {
    type Message = Msg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        request_version(ctx.link());

        Self { api_version: None }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::CreateEvent => {
                ctx.link().navigator().unwrap_throw().push(&Route::NewEvent);
                false
            }
            Msg::Privacy => {
                ctx.link().navigator().unwrap_throw().push(&Route::Privacy);
                false
            }
            Msg::Admin => {
                ctx.link().navigator().unwrap_throw().push(&Route::Login);
                false
            }
            Msg::VersionReceived(api_version) => {
                self.api_version = api_version;
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        self.view_footer(ctx)
    }
}

impl Footer {
    fn view_social() -> Html {
        let twitter_svg = {
            let svg = include_str!("../../inline-assets/twitter.svg");
            let div = gloo_utils::document().create_element("div").unwrap_throw();
            div.set_inner_html(svg);
            div.set_id("twitter");
            div.set_class_name("social");
            Html::VRef(div.into())
        };

        let github_svg = {
            let svg = include_str!("../../inline-assets/github.svg");
            let div = gloo_utils::document().create_element("div").unwrap_throw();
            div.set_inner_html(svg);
            div.set_id("github");
            div.set_class_name("social");
            Html::VRef(div.into())
        };

        let linkedin_svg = {
            let svg = include_str!("../../inline-assets/linkedin.svg");
            let div = gloo_utils::document().create_element("div").unwrap_throw();
            div.set_inner_html(svg);
            div.set_id("linkedin");
            div.set_class_name("social");
            Html::VRef(div.into())
        };

        let producthunt_svg = {
            let svg = include_str!("../../inline-assets/ph.svg");
            let div = gloo_utils::document().create_element("div").unwrap_throw();
            div.set_inner_html(svg);
            div.set_id("producthunt");
            div.set_class_name("social");
            Html::VRef(div.into())
        };
        let insta_svg = {
            let svg = include_str!("../../inline-assets/insta.svg");
            let div = gloo_utils::document().create_element("div").unwrap_throw();
            div.set_inner_html(svg);
            div.set_id("insta");
            div.set_class_name("social");
            Html::VRef(div.into())
        };
        let mastodon_svg = {
            let svg = include_str!("../../inline-assets/mastodon.svg");
            let div = gloo_utils::document().create_element("div").unwrap_throw();
            div.set_inner_html(svg);
            div.set_id("mastodon");
            div.set_class_name("social");
            Html::VRef(div.into())
        };

        html! {
            <>
            <a href="https://github.com/liveask/liveask" target="_blank">
                {github_svg}
            </a>
            <a href="https://twitter.com/liveaskapp" target="_blank">
                {twitter_svg}
            </a>
            <a href="https://www.instagram.com/liveaskapp/" target="_blank">
                {insta_svg}
            </a>
            <a href="https://mastodon.social/@liveask" target="_blank">
                {mastodon_svg}
            </a>
            <a href="https://www.linkedin.com/company/live-ask" target="_blank">
                {linkedin_svg}
            </a>
            <a href="https://www.producthunt.com/products/live-ask" target="_blank">
                {producthunt_svg}
            </a>
            </>
        }
    }

    fn view_footer(&self, ctx: &Context<Self>) -> Html {
        let branch = if GIT_BRANCH == "main" {
            String::new()
        } else {
            format!("({GIT_BRANCH})",)
        };

        let git_sha = env!("VERGEN_GIT_SHA");

        let api_version = self
            .api_version
            .as_ref()
            .filter(|version| version != &git_sha)
            .map_or_else(String::new, |api_version| format!("[api:{api_version}]"));

        html! {
            <div class="feature-dark">
                <h1>
                    {"Try it now for free!"}
                </h1>
                <button class="button-red" onclick={ctx.link().callback(|_| Msg::CreateEvent)}>
                    {"Create your Event"}
                </button>

                <div class="copyright">
                    {"© 2023 Live-Ask. All right reserved"}
                </div>

                {Self::view_social()}

                <a class="about" href="https://github.com/liveask/liveask" target="_blank">
                    {"About"}
                </a>

                <div class="link about" onclick={ctx.link().callback(|_| Msg::Privacy)}>
                    {"Privacy Policy"}
                </div>

                <a class="status" href="https://liveask.instatus.com" target="_blank">
                    {"Status"}
                </a>

                <a class="version" href="https://github.com/liveask/liveask/blob/main/CHANGELOG.md" target="_blank">
                    { format!("v{VERSION_STR}-{git_sha} {branch} {api_version}") }
                </a>

                <div id="admin">
                    <div class="inner" onclick={ctx.link().callback(|_| Msg::Admin)}>
                        <img alt="admin-button" src="/assets/admin.svg" />
                    </div>
                </div>
            </div>
        }
    }
}

fn request_version(link: &html::Scope<Footer>) {
    link.send_future(async move {
        match fetch::fetch_version(crate::pages::BASE_API).await {
            Err(e) => {
                log::error!("fetch_version error: {e}");
                Msg::VersionReceived(None)
            }
            Ok(res) => Msg::VersionReceived(Some(res)),
        }
    });
}
