//! The UI, rendered on the server and hydrated in the browser.

use dioxus::prelude::*;

mod components;
mod pages;

use pages::{Account, Feed, FeedQuery, NotFound, SignIn};

/// Every stylesheet, loaded once for the whole app rather than by the page
/// that uses it, so client-side navigation never shows a page unstyled.
/// Order matters only for `base.css`, which defines the color tokens.
const STYLESHEETS: &[Asset] = &[
    asset!("/assets/styles/base.css"),
    asset!("/assets/styles/buttons.css"),
    asset!("/assets/styles/forms.css"),
    asset!("/assets/styles/brand.css"),
    asset!("/assets/styles/top_bar.css"),
    asset!("/assets/styles/footer.css"),
    asset!("/assets/styles/post_card.css"),
    asset!("/assets/styles/feed.css"),
    asset!("/assets/styles/auth.css"),
    asset!("/assets/styles/not_found.css"),
];

#[derive(Clone, Debug, PartialEq, Routable)]
enum Route {
    #[route("/?:..query")]
    Feed { query: FeedQuery },
    #[route("/account")]
    Account {},
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
        for href in STYLESHEETS {
            document::Stylesheet { href: *href }
        }
        Router::<Route> {}
    }
}
