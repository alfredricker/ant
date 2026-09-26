//! The UI, rendered on the server and hydrated in the browser.

use dioxus::prelude::*;

mod feed;
mod signin;

use feed::{Feed, FeedQuery};
use signin::SignIn;

const STYLES: Asset = asset!("/assets/styles.css");

#[derive(Clone, Debug, PartialEq, Routable)]
enum Route {
    #[route("/?:..query")]
    Feed { query: FeedQuery },
    #[route("/signin?:error")]
    SignIn { error: String },
    #[route("/:..segments")]
    NotFound { segments: Vec<String> },
}

#[component]
pub fn App() -> Element {
    rsx! {
        document::Title { "SandHouse: find people who want the same big thing" }
        document::Meta {
            name: "description",
            content: "SandHouse brings together people with the same ambitions, whether that's a trip, a startup, a research project or something else, so they can go after it together.",
        }
        document::Stylesheet { href: STYLES }
        Router::<Route> {}
    }
}

#[component]
fn NotFound(segments: Vec<String>) -> Element {
    let path = segments.join("/");
    rsx! {
        main { class: "not-found",
            h1 { "Nothing here yet." }
            p { class: "feed-note", "/{path} doesn't exist (yet)." }
            Link { class: "button button-primary", to: Route::Feed { query: FeedQuery::default() }, "Back to the feed" }
        }
    }
}
