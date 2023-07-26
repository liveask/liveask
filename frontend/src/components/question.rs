use chrono::Utc;
use gloo::timers::callback::Interval;
use gloo::timers::callback::Timeout;
use shared::QuestionItem;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use wasm_bindgen::UnwrapThrowExt;
use web_sys::Element;
use web_sys::HtmlElement;
use web_sys::ScrollBehavior;
use web_sys::ScrollIntoViewOptions;
use web_sys::ScrollLogicalPosition;
use yew::prelude::*;

pub enum QuestionClickType {
    Like,
    Hide,
    Answer,
    Approve,
}

//TODO: use bitflag to rid us of this warning
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Debug, PartialEq, Properties)]
pub struct Props {
    pub item: Rc<QuestionItem>,
    pub index: usize,
    pub mod_view: bool,
    pub is_new: bool,
    pub local_like: bool,
    pub can_vote: bool,
    pub blurr: bool,
    pub on_click: Callback<(i64, QuestionClickType)>,
}

pub struct Question {
    data: Props,
    age_text: String,
    node_ref: NodeRef,
    last_pos: Option<i64>,
    timeout: Option<Timeout>,
    reorder_animation_timeout: Option<Timeout>,
    highlight_animation_timeout: Option<Timeout>,
    _interval: Interval,
    highlighted: bool,
}

pub enum AnimationState {
    Start,
    End,
}

pub enum Msg {
    UpdateAge,
    QuestionClick(QuestionClickType),
    ReorderAnimation(AnimationState),
    HighlightEnd,
}
impl Component for Question {
    type Message = Msg;
    type Properties = Props;

    fn create(ctx: &Context<Self>) -> Self {
        let interval = {
            let link = ctx.link().clone();
            Interval::new(1000, move || link.send_message(Msg::UpdateAge))
        };

        let mut res = Self {
            data: ctx.props().clone(),
            age_text: String::new(),
            node_ref: NodeRef::default(),
            last_pos: None,
            timeout: None,
            reorder_animation_timeout: None,
            highlight_animation_timeout: None,
            _interval: interval,
            highlighted: false,
        };

        if res.data.is_new {
            res.highlighted = true;
            log::info!("highlight on");
            let link = ctx.link().clone();
            res.highlight_animation_timeout = Some(Timeout::new(800, move || {
                log::info!("highlight off");
                link.send_message(Msg::HighlightEnd);
            }));
        }

        res
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::QuestionClick(click_type) => {
                if matches!(click_type, QuestionClickType::Like) {
                    if ctx.props().can_vote
                        && !self.data.item.screening
                        && !self.data.item.answered
                        && !self.data.item.hidden
                    {
                        ctx.props()
                            .on_click
                            .emit((self.data.item.id, QuestionClickType::Like));
                        return true;
                    }
                    return false;
                }
                ctx.props().on_click.emit((self.data.item.id, click_type));
                true
            }
            Msg::UpdateAge => {
                let age = self.get_age();
                if age != self.age_text {
                    self.age_text = age;
                    return true;
                }
                false
            }
            Msg::ReorderAnimation(state) => {
                match state {
                    AnimationState::End => {
                        self.reorder_animation_timeout = None;
                        false
                    }
                    AnimationState::Start => {
                        self.reset_transition();

                        let handle = {
                            let link = ctx.link().clone();
                            //note: 500ms needs to be synced with css
                            Timeout::new(500, move || {
                                link.send_message(Msg::ReorderAnimation(AnimationState::End));
                            })
                        };
                        self.reorder_animation_timeout = Some(handle);

                        false
                    }
                }
            }

            Msg::HighlightEnd => {
                self.highlighted = false;
                self.highlight_animation_timeout = None;
                true
            }
        }
    }

    fn changed(&mut self, ctx: &Context<Self>) -> bool {
        let props = ctx.props().clone();
        if self.data == props {
            false
        } else {
            // log::info!("changed: {}", props.item.id);

            let likes_changed = self.data.item.likes != props.item.likes;
            if likes_changed {
                log::info!(
                    "q: {} likes changed (old: {})",
                    self.data.item.id,
                    self.data.item.likes
                );

                //TODO: animate bubble
            }
            self.data = props;
            true
        }
    }

    fn rendered(&mut self, ctx: &Context<Self>, first_render: bool) {
        let (elem, element_y) = self.get_elem_y();
        let elem: HtmlElement = elem.dyn_into::<HtmlElement>().unwrap_throw();

        if let Some(last_pos) = self.last_pos {
            if self.reorder_animation_timeout.is_none()
                && self.timeout.is_none()
                && last_pos != element_y
            {
                let diff = last_pos - element_y;

                let style = elem.style();

                style
                    .set_property("transition-duration", "0s")
                    .unwrap_throw();
                style
                    .set_property("transform", &format!("translate(0px,{diff}px)"))
                    .unwrap_throw();

                let handle = {
                    let link = ctx.link().clone();
                    Timeout::new(0, move || {
                        link.send_message(Msg::ReorderAnimation(AnimationState::Start));
                    })
                };

                self.timeout = Some(handle);
            }
        }

        //do not save pos while animating
        if self.reorder_animation_timeout.is_none() {
            self.last_pos = Some(element_y);
        }

        if first_render && self.data.is_new {
            elem.scroll_into_view_with_scroll_into_view_options(
                ScrollIntoViewOptions::new()
                    .block(ScrollLogicalPosition::Center)
                    .behavior(ScrollBehavior::Smooth),
            );
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let liked = ctx.props().local_like;
        let mod_view = ctx.props().mod_view;
        let blurred = ctx.props().blurr;
        let can_vote = ctx.props().can_vote && !self.data.item.screening;
        let screened = !self.data.item.screening;
        let main_classes = classes!(
            "question-host",
            "questions-move",
            self.data.item.screening.then_some("unscreened-question"),
        );

        html! {
            <div class={main_classes}
                ref={self.node_ref.clone()}>
                <a class={classes!("questionanchor",self.highlighted.then_some("highlighted"),)}
                    onclick={ctx.link().callback(|_| Msg::QuestionClick(QuestionClickType::Like))}>

                    <div class="time-since">
                        {self.get_age()}
                    </div>

                    {
                        if screened {
                            if liked {
                                Self::get_bubble_liked(self.data.item.likes)
                            }
                            else
                            {
                                Self::get_bubble_not_liked(self.data.item.likes)
                            }
                        } else { html!() }
                    }

                    <div class={classes!("text",self.data.item.answered.then_some("answered"),blurred.then_some("blurr"))}>
                        {&self.data.item.text}
                    </div>

                    {self.view_like(can_vote,liked,mod_view)}

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
        let elem = self.get_element().expect_throw("reset_transition error");
        let elem: HtmlElement = elem
            .dyn_into::<HtmlElement>()
            .expect_throw("reset_transition error 2");
        let style = elem.style();
        style
            .remove_property("transition-duration")
            .expect_throw("reset_transition error 3");
        style
            .remove_property("transform")
            .expect_throw("reset_transition error 4");
        self.timeout = None;
    }

    fn view_mod(&self, ctx: &Context<Self>) -> Html {
        if ctx.props().blurr {
            return html! {};
        }

        let hidden = self.data.item.hidden;
        let answered = self.data.item.answered;
        let screened = !self.data.item.screening;

        if screened {
            html! {
                <div class="options">
                    <button class={classes!("button-hide",hidden.then_some("reverse"))}
                        onclick={ctx.link().callback(|_| Msg::QuestionClick(QuestionClickType::Hide))}
                        hidden={answered}
                        >
                        {
                            if hidden {
                                html!{"unhide"}
                            }else{
                                html!{"hide"}
                            }
                        }
                    </button>

                    <button class={classes!("button-answered",answered.then_some("reverse"))}
                        onclick={ctx.link().callback(|_| Msg::QuestionClick(QuestionClickType::Answer))}
                        hidden={hidden}
                        >
                        {
                            if answered {
                                html!{"not answered"}
                            }else{
                                html!{"answered"}
                            }
                        }
                    </button>
                </div>
            }
        } else {
            html! {
                <div class="options">
                    <button class={classes!("button-hide",hidden.then_some("reverse"))}
                        onclick={ctx.link().callback(|_| Msg::QuestionClick(QuestionClickType::Hide))}
                        >
                        {"hide"}
                    </button>

                    <button class="button-answered"
                        onclick={ctx.link().callback(|_| Msg::QuestionClick(QuestionClickType::Approve))}
                        >
                        {"approve"}
                    </button>
                </div>
            }
        }
    }

    fn get_age(&self) -> String {
        use chrono::TimeZone;

        let delta = Utc::now()
            - Utc
                .timestamp_opt(self.data.item.create_time_unix, 0)
                .latest()
                .unwrap_throw();

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

    fn view_like(&self, can_like: bool, liked: bool, mod_view: bool) -> Html {
        if can_like && !self.data.item.answered && !mod_view {
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
        use easy_cast::ConvFloat;

        let elem = self.get_element().expect_throw("get_elem_y error 1");
        let scroll_y = i64::conv_nearest(
            gloo_utils::window()
                .scroll_y()
                .expect_throw("get_elem_y error 2"),
        );
        let r = elem.get_bounding_client_rect();
        let element_y = i64::conv_nearest(r.y()) + scroll_y;

        (elem, element_y)
    }

    fn get_element(&self) -> Option<Element> {
        self.node_ref.cast::<Element>()
    }
}
