//! The UI, rendered on the server and hydrated in the browser.

use dioxus::prelude::*;

mod components;
mod pages;

use crate::{api, models::account::Theme};
use pages::{Account, Feed, FeedQuery, NotFound, SignIn};

/// Every stylesheet, loaded once for the whole app rather than by the page
/// that uses it, so client-side navigation never shows a page unstyled.
/// Order matters only for `base.css`, which defines the color tokens.
const STYLESHEETS: &[Asset] = &[
    asset!("/assets/styles/base.css"),
    asset!("/assets/styles/buttons.css"),
    asset!("/assets/styles/forms.css"),
    asset!("/assets/styles/avatar.css"),
    asset!("/assets/styles/brand.css"),
    asset!("/assets/styles/top_bar.css"),
    asset!("/assets/styles/footer.css"),
    asset!("/assets/styles/post_card.css"),
    asset!("/assets/styles/feed.css"),
    asset!("/assets/styles/auth.css"),
    asset!("/assets/styles/account.css"),
    asset!("/assets/styles/not_found.css"),
];

#[derive(Clone, Debug, PartialEq, Routable)]
enum Route {
    #[route("/?:..query")]
    Feed { query: FeedQuery },
    #[route("/account?:notice")]
    Account { notice: String },
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
        Themed { Router::<Route> {} }
    }
}

/// Wraps the app in the signed-in user's theme (see `assets/styles/base.css`).
/// Resolved during server rendering, so the first paint is already right.
/// The theme is shared as a `Signal<Theme>` context, so the account page's
/// picker restyles everything the moment it's clicked.
#[component]
fn Themed(children: Element) -> Element {
    // If it can't be loaded, the default (follow the OS) is a fine fallback.
    let saved = use_server_future(|| async { api::account::theme().await.unwrap_or_default() })?;
    let initial = saved.read().unwrap_or_default();
    let theme = use_context_provider(|| Signal::new(initial));

    rsx! {
        div { class: "app", "data-theme": theme.read().slug(), {children} }
    }
}

/// The theme picked on the account page; set it to restyle the whole app.
pub fn use_theme() -> Signal<Theme> {
    use_context()
}
