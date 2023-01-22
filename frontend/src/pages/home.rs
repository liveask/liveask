use wasm_bindgen::UnwrapThrowExt;
use yew::prelude::*;
use yew_router::prelude::*;

use crate::{fetch, routes::Route, VERSION_STR};

use super::BASE_API;

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
                ctx.link().history().unwrap_throw().push(Route::NewEvent);
                false
            }
            Msg::Example => {
                ctx.link().history().unwrap_throw().push(Route::Event {
                    id: "eventexample".into(),
                });
                false
            }
            Msg::Privacy => {
                ctx.link().history().unwrap_throw().push(Route::Privacy);
                false
            }
            Msg::Admin => {
                ctx.link().history().unwrap_throw().push(Route::Login);
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
                    <img class="img-simple" src="assets/main-incognito.png" />
                    <p>
                        {"No registration necessary - everyone can ask questions and vote. Participant anonymity ensures freedom of speech and a smooth
                        user experience."}
                    </p>
                </div>

                <div class="feature-dark">
                    <h1>
                        {"Effortless"}
                    </h1>
                    <img class="img-simple" src="assets/main-effortless.png" />
                    <p>
                        {"Set up your event in seconds! Share the link with your audience and let them decide what’s hot."}
                    </p>
                </div>

                <div class="feature-bright">
                    <h1>
                        {"Real-Time"}
                    </h1>
                    <img class="img-simple" id="img-realtime" src="assets/main-realtime.png" />
                    <p>
                        {" Designed for live events. Questions can be asked and voted on in real time. This way, you can interact with everyone seamlessly."}
                    </p>
                </div>

                <div class="feature-dark">
                    <h1>
                        {"Cross Platform"}
                    </h1>
                    <img class="img-simple" id="img-crossplatform" src="assets/main-crossplatform.png" />
                    <p>
                        {"Use Live-Ask on your mobile phone, tablet, laptop or desktop computer. Go crazy and cast it to your smart TV, too!"}
                    </p>
                </div>

                <div class="feature-bright">
                    <h1>
                        {"Social"}
                    </h1>
                    <img class="img-simple" src="assets/main-social.png" />
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
    fn view_footer(&self, ctx: &Context<Self>) -> Html {
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

        let branch = if env!("VERGEN_GIT_BRANCH") == "main" {
            String::new()
        } else {
            format!("({})", env!("VERGEN_GIT_BRANCH"))
        };

        let git_sha = env!("VERGEN_GIT_SHA_SHORT");

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

                <a href="https://twitter.com/liveask1">
                    {twitter_svg}
                </a>
                <a href="https://github.com/liveask/liveask">
                    {github_svg}
                </a>

                <a class="about" onclick={ctx.link().callback(|_| Msg::Privacy)}>
                    {"Privacy Policy"}
                </a>

                <a class="about" href="https://github.com/liveask/liveask">
                    {"About"}
                </a>

                <div class="version">
                    { format!("v.{VERSION_STR}-{git_sha} {branch} {api_version}") }
                </div>

                <div id="admin">
                    <div class="inner" onclick={ctx.link().callback(|_| Msg::Admin)}>
                        <img src="/assets/admin.svg" />
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
