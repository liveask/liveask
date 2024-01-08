use wasm_bindgen::UnwrapThrowExt;
use yew::prelude::*;
use yew_router::prelude::*;

use crate::{components::Footer, routes::Route};

#[allow(clippy::empty_structs_with_brackets)]
#[derive(Clone, Debug, Eq, PartialEq, Properties)]
pub struct HomeProps;

pub struct Home;
pub enum Msg {
    Example,
    CreateEvent,
}
impl Component for Home {
    type Message = Msg;
    type Properties = HomeProps;

    fn create(_ctx: &Context<Self>) -> Self {
        Self {}
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

                <Footer></Footer>
            </div>
        }
    }
}
