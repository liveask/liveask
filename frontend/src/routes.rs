use yew_router::prelude::*;

#[derive(Clone, Debug, Eq, PartialEq, Routable)]
pub enum Route {
    #[at("/newevent")]
    NewEvent,
    #[at("/privacy")]
    Privacy,
    #[at("/event/:id")]
    Event { id: String },
    #[at("/eventmod/:id/:secret")]
    EventMod { id: String, secret: String },
    #[at("/")]
    Home,
}
