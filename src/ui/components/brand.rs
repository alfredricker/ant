use dioxus::prelude::*;

use crate::ui::{Route, pages::FeedQuery};

/// The 🐜 SandHouse mark, linking home.
#[component]
pub fn Brand() -> Element {
    rsx! {
        Link { class: "brand", to: Route::Feed { query: FeedQuery::default() },
            span { class: "brand-mark", "🐜" }
            span { class: "brand-name", "SandHouse" }
        }
    }
}
