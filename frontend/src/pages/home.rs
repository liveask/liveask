use wasm_bindgen::UnwrapThrowExt;
use yew::prelude::*;
use yew_router::prelude::*;

use crate::{fetch, routes::Route, GIT_BRANCH, VERSION_STR};

use super::BASE_API;

#[allow(clippy::empty_structs_with_brackets)]
#[derive(Clone, Debug, Eq, PartialEq, Properties)]
pub struct HomeProps;

pub struct Home {
    api_version: Option<String>,
}
pub enum Msg {
    Example,
    CreateEvent,
    Privacy,
    Admin,
    VersionReceived(Option<String>),
}
impl Component for Home {
    type Message = Msg;
    type Properties = HomeProps;

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
            Msg::Example => {
                ctx.link().navigator().unwrap_throw().push(&Route::Event {
                    id: "eventexample".into(),
                });
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
        html! {
            <div id="home">
                <div class="feature-dark">
                    <h1 id="firstheader">
                        {"Real-Time questions from your audience"}
                    </h1>
                    <p>
                        {"Have you ever organized a meetup, conference, or moderated a panel discussion and wanted an easy way to receive real-time
                        questions from your audience? Welcome to Live-Ask."}
                    </p>
                    <button class="button-red" onclick={ctx.link().callback(|_| Msg::CreateEvent)}>
                        {"Create your Event"}
                    </button>
                    <button class="button-dark" onclick={ctx.link().callback(|_| Msg::Example)}>
                        {"View Example"}
                    </button>
                </div>

                <div class="feature-bright">
                    <h1>
                        {"Incognito"}
                    </h1>
                    <img alt="anonymous" class="img-simple" src="assets/main-incognito.png" />
                    <p>
                        {"No registration necessary - everyone can ask questions and vote. Participant anonymity ensures freedom of speech and a smooth
                        user experience."}
                    </p>
                </div>

                <div class="feature-dark">
                    <h1>
                        {"Effortless"}
                    </h1>
                    <img alt="effortless" class="img-simple" src="assets/main-effortless.png" />
                    <p>
                        {"Set up your event in seconds! Share the link with your audience and let them decide what’s hot."}
                    </p>
                </div>

                <div class="feature-bright">
                    <h1>
                        {"Real-Time"}
                    </h1>
                    <img alt="realtime" class="img-simple" id="img-realtime" src="assets/main-realtime.png" />
                    <p>
                        {" Designed for live events. Questions can be asked and voted on in real time. This way, you can interact with everyone seamlessly."}
                    </p>
                </div>

                <div class="feature-dark">
                    <h1>
                        {"Cross Platform"}
                    </h1>
                    <img alt="simple" class="img-simple" id="img-crossplatform" src="assets/main-crossplatform.png" />
                    <p>
                        {"Use Live-Ask on your mobile phone, tablet, laptop or desktop computer. Go crazy and cast it to your smart TV, too!"}
                    </p>
                </div>

                <div class="feature-bright">
                    <h1>
                        {"Social"}
                    </h1>
                    <img alt="social" class="img-simple" src="assets/main-social.png" />
                    <p>
                        {"We want to make sharing as effortless as possible. Have you organized an awesome event? Live-Ask makes it easy to share it
                        with others. You bring the great content, we’ll help you spread the word."}
                    </p>
                </div>

                {self.view_footer(ctx)}
            </div>
        }
    }
}

impl Home {
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

fn request_version(link: &html::Scope<Home>) {
    link.send_future(async move {
        match fetch::fetch_version(BASE_API).await {
            Err(e) => {
                log::error!("fetch_version error: {e}");
                Msg::VersionReceived(None)
            }
            Ok(res) => Msg::VersionReceived(Some(res)),
        }
    });
}
