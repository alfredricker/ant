//! The UI, rendered on the server and hydrated in the browser.

use dioxus::prelude::*;

mod home;

use home::Home;

const STYLES: Asset = asset!("/assets/styles.css");

#[derive(Clone, Debug, PartialEq, Routable)]
enum Route {
    #[route("/")]
    Home {},
    #[route("/:..segments")]
    NotFound { segments: Vec<String> },
}

#[component]
pub fn App() -> Element {
    rsx! {
        document::Title { "ant — find people to build with" }
        document::Meta {
            name: "description",
            content: "ant pairs people up on passion projects, school projects and anything else worth building together.",
        }
        document::Stylesheet { href: STYLES }
        Router::<Route> {}
    }
}

#[component]
fn NotFound(segments: Vec<String>) -> Element {
    let path = segments.join("/");
    rsx! {
        main {
            section { class: "hero",
                h1 { "Nothing here yet." }
                p { class: "lede", "/{path} doesn't exist (yet)." }
                div { class: "hero-actions",
                    Link { class: "button button-primary", to: Route::Home {}, "Back home" }
                }
            }
        }
    }
}
