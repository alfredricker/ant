use dioxus::prelude::*;

use crate::ui::{Route, pages::FeedQuery};

#[component]
pub fn NotFound(segments: Vec<String>) -> Element {
    let path = segments.join("/");
    rsx! {
        main { class: "not-found",
            h1 { "Nothing here yet." }
            p { class: "feed-note", "/{path} doesn't exist (yet)." }
            Link { class: "button button-primary", to: Route::Feed { query: FeedQuery::default() }, "Back to the feed" }
        }
    }
}
