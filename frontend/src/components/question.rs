use chrono::Utc;
use gloo::timers::callback::Interval;
use gloo::timers::callback::Timeout;
use shared::Item;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use web_sys::Element;
use web_sys::HtmlElement;
use yew::prelude::*;

pub enum QuestionClickType {
    Like,
    Hide,
    Answer,
}

#[derive(Clone, Debug, PartialEq, Properties)]
pub struct Props {
    pub item: Rc<Item>,
    pub index: usize,
    pub mod_view: bool,
    pub is_new: bool,
    pub local_like: bool,
    pub on_click: Callback<(i64, QuestionClickType)>,
}

pub struct Question {
    data: Props,
    node_ref: NodeRef,
    last_pos: Option<i64>,
    timeout: Option<Timeout>,
    animation_time: Option<Timeout>,
    _interval: Interval,
}
pub enum Msg {
    UpdateAge,
    Like,
    ToggleHide,
    ToggleAnswered,
    /// used to start the reorder animation
    StartAnimation,
    EndAnimation,
}
impl Component for Question {
    type Message = Msg;
    type Properties = Props;

    fn create(ctx: &Context<Self>) -> Self {
        let interval = {
            let link = ctx.link().clone();
            Interval::new(1000, move || link.send_message(Msg::UpdateAge))
        };

        Self {
            data: ctx.props().clone(),
            node_ref: NodeRef::default(),
            last_pos: None,
            timeout: None,
            animation_time: None,
            _interval: interval,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Like => {
                if !self.data.item.answered && !self.data.item.hidden {
                    ctx.props()
                        .on_click
                        .emit((self.data.item.id, QuestionClickType::Like));
                    true
                } else {
                    false
                }
            }
            Msg::ToggleHide => {
                ctx.props()
                    .on_click
                    .emit((self.data.item.id, QuestionClickType::Hide));
                true
            }
            Msg::ToggleAnswered => {
                ctx.props()
                    .on_click
                    .emit((self.data.item.id, QuestionClickType::Answer));
                true
            }
            Msg::UpdateAge => self.animation_time.is_none(),
            Msg::EndAnimation => {
                self.animation_time = None;
                false
            }
            Msg::StartAnimation => {
                self.reset_transition();

                let handle = {
                    let link = ctx.link().clone();
                    //note: 500ms needs to be synced with css
                    Timeout::new(500, move || link.send_message(Msg::EndAnimation))
                };
                self.animation_time = Some(handle);

                false
            }
        }
    }

    fn changed(&mut self, ctx: &Context<Self>) -> bool {
        let props = ctx.props().clone();
        if self.data != props {
            // log::info!("changed: {}", props.item.id);
            self.data = props;
            true
        } else {
            false
        }
    }

    fn rendered(&mut self, ctx: &Context<Self>, first_render: bool) {
        let (elem, element_y) = self.get_elem_y();
        let elem: HtmlElement = elem.dyn_into::<HtmlElement>().unwrap();

        if let Some(last_pos) = self.last_pos {
            if self.animation_time.is_none() && self.timeout.is_none() && last_pos != element_y {
                let diff = last_pos - element_y;

                let style = elem.style();

                style.set_property("transition-duration", "0s").unwrap();
                style
                    .set_property("transform", &format!("translate(0px,{}px)", diff))
                    .unwrap();

                let handle = {
                    let link = ctx.link().clone();
                    Timeout::new(0, move || link.send_message(Msg::StartAnimation))
                };

                self.timeout = Some(handle);
            }
        }

        //do not save pos while animating
        if self.animation_time.is_none() {
            self.last_pos = Some(element_y);
        }

        if first_render && self.data.is_new {
            elem.scroll_into_view();
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let liked = ctx.props().local_like;
        let mod_view = ctx.props().mod_view;

        html! {
            <div class="question-host questions-move"
                ref={self.node_ref.clone()}>
                <a class="questionanchor" onclick={ctx.link().callback(|_| Msg::Like)}>

                    <div class="time-since">
                        {self.get_age()}
                    </div>

                    {
                        if liked {
                            Self::get_bubble_liked(self.data.item.likes)
                        }
                        else
                        {
                            Self::get_bubble_not_liked(self.data.item.likes)
                        }
                    }

                    <div class={classes!("text",self.data.item.answered.then_some("answered"))}>
                        {&self.data.item.text}
                    </div>

                    {self.view_like(liked,mod_view)}

                    {self.view_checkmark(mod_view)}
                </a>

                {
                    if mod_view{
                        self.view_mod(ctx)
                    } else {
                        html!{}
                    }
                }

            </div>
        }
    }
}

impl Question {
    fn reset_transition(&mut self) {
        let elem = self.get_element().unwrap();
        let elem: HtmlElement = elem.dyn_into::<HtmlElement>().unwrap();
        let style = elem.style();
        style.remove_property("transition-duration").unwrap();
        style.remove_property("transform").unwrap();
        self.timeout = None;
    }

    fn view_mod(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class="options">
                <button class={classes!("button-hide",self.data.item.hidden.then_some("reverse"))}
                    onclick={ctx.link().callback(|_| Msg::ToggleHide)}
                    hidden={self.data.item.answered}
                    >
                    {
                        if self.data.item.hidden {
                            html!{"unhide"}
                        }else{
                            html!{"hide"}
                        }
                    }
                </button>

                <button class={classes!("button-answered",self.data.item.answered.then_some("reverse"))}
                    onclick={ctx.link().callback(|_| Msg::ToggleAnswered)}
                    hidden={self.data.item.hidden}
                    >
                    {
                        if self.data.item.answered {
                            html!{"not answered"}
                        }else{
                            html!{"answered"}
                        }
                    }
                </button>
            </div>
        }
    }

    fn get_age(&self) -> String {
        use chrono::TimeZone;

        let delta = Utc::now() - Utc.timestamp(self.data.item.create_time_unix, 0);

        if delta.num_minutes() < 1 {
            String::from("just now")
        } else if delta.num_hours() < 1 {
            format!("{} min ago", delta.num_minutes())
        } else if delta.num_days() < 1 {
            format!("{} hours ago", delta.num_hours())
        } else {
            format!("{} days ago", delta.num_days())
        }
    }

    fn get_bubble_liked(likes: i32) -> Html {
        html! {
        <span class="bubble">
            <svg width="29px" height="19px" viewBox="0 0 29 19">
                <g id="Mobile" stroke="none" stroke-width="1" fill-rule="evenodd">
                    <g id="Audience-Page-Questions" transform="translate(-327.000000, -493.000000)">
                        <g id="Frage" transform="translate(10.000000, 482.000000)">
                            <g id="Group-2-Copy-4" transform="translate(317.000000, 11.000000)">
                                <rect id="Rectangle-Copy-10" fill="#FF2C5E" x="4" y="0" width="25" height="15" rx="7.5"></rect>
                                <text id="23" font-size="10" letter-spacing="0.15625" fill="#FFFFFF">
                                    <tspan class="like-count-text" x="16" y="11">
                                        {likes}
                                    </tspan>
                                </text>
                                <circle id="Oval-Copy-50" fill="#FF2C5E" cx="1.5" cy="17.5" r="1.5"></circle>
                                <circle id="Oval-Copy-51" fill="#FF2C5E" cx="5.5" cy="12.5" r="2.5"></circle>
                            </g>
                        </g>
                    </g>
                </g>
            </svg>
        </span>
        }
    }

    fn get_bubble_not_liked(likes: i32) -> Html {
        html! {
        <span class="bubble">
            <svg width="29px" height="19px" viewBox="0 0 29 19">
                <g id="Mobile" stroke="none" stroke-width="1" fill-rule="evenodd">
                    <g id="Audience-Page-Questions" transform="translate(-327.000000, -493.000000)">
                        <g id="Frage" transform="translate(10.000000, 482.000000)">
                            <g id="Group-2-Copy-4" transform="translate(317.000000, 11.000000)">
                                <rect id="Rectangle-Copy-10" fill="#D4D4D4" x="4" y="0" width="25" height="15" rx="7.5"></rect>
                                <text id="23" font-size="10" letter-spacing="0.15625" fill="#FFFFFF">
                                    <tspan class="like-count-text" x="16" y="11">
                                        {likes}
                                    </tspan>
                                </text>
                                <circle id="Oval-Copy-50" fill="#D4D4D4" cx="1.5" cy="17.5" r="1.5"></circle>
                                <circle id="Oval-Copy-51" fill="#D4D4D4" cx="5.5" cy="12.5" r="2.5"></circle>
                            </g>
                        </g>
                    </g>
                </g>
            </svg>
        </span>
        }
    }

    fn view_checkmark(&self, mod_view: bool) -> Html {
        if !mod_view && self.data.item.answered {
            return html! {
            <div class="checkmark">
                <svg width="12px" height="8px" viewBox="0 0 12 8">
                    <g id="Symbols" stroke="none" stroke-width="1" fill="none" fill-rule="evenodd">
                        <g id="tick" stroke-width="2" stroke="#8CC63F">
                            <polyline id="Path-2" points="0 3.6977267 4.02613665 7.72386335 11.75 -1.13686838e-13"></polyline>
                        </g>
                    </g>
                </svg>
            </div>
            };
        }

        html! {}
    }

    fn view_like(&self, liked: bool, mod_view: bool) -> Html {
        if !self.data.item.answered && !mod_view {
            return html! {
                <div class="like-action">
                    {
                        if liked {
                            "Unlike!"
                        }else{
                            "I like!"
                        }
                    }
                </div>
            };
        }

        html! {}
    }
}

impl Question {
    fn get_elem_y(&self) -> (Element, i64) {
        let elem = self.get_element().unwrap();
        let scroll_y = gloo_utils::window().scroll_y().unwrap() as i64;
        let r = elem.get_bounding_client_rect();
        let element_y = r.y() as i64 + scroll_y;

        (elem, element_y)
    }

    fn get_element(&self) -> Option<Element> {
        self.node_ref.cast::<Element>()
    }
}
