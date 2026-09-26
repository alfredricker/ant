//! The sticky bar on top of the feed: brand, search, and who's signed in.

use dioxus::prelude::*;

use crate::{
    api,
    ui::{
        Route,
        components::{Avatar, Brand},
        pages::FeedQuery,
    },
};

#[component]
pub fn TopBar(query: FeedQuery) -> Element {
    rsx! {
        header { class: "topbar",
            Brand {}
            SearchBox { query }
            AccountMenu {}
        }
    }
}

#[component]
fn SearchBox(query: FeedQuery) -> Element {
    let mut text = use_signal(|| query.text.clone());

    rsx! {
        // A plain GET form: works before the wasm loads. Once it has, submit
        // goes through the router instead of reloading the page.
        form {
            class: "search",
            role: "search",
            method: "get",
            action: "/",
            onsubmit: move |evt| {
                evt.prevent_default();
                navigator().push(Route::Feed {
                    query: FeedQuery { text: text(), ..query.clone() },
                });
            },
            input {
                r#type: "search",
                name: "q",
                placeholder: "Search ambitions, skills, places…",
                "aria-label": "Search posts",
                value: "{text}",
                oninput: move |evt| text.set(evt.value()),
            }
            if let Some(kind) = query.kind {
                input { r#type: "hidden", name: "kind", value: kind.slug() }
            }
            if query.funded {
                input { r#type: "hidden", name: "funded", value: "1" }
            }
        }
    }
}

#[component]
fn AccountMenu() -> Element {
    // Resolved during server rendering (the session cookie comes along), so
    // the page arrives already showing the right account state.
    let user = use_server_future(api::current_user)?;

    let account = match &*user.read() {
        None => rsx! {},
        Some(Ok(Some(user))) => rsx! {
            Link {
                class: "nav-avatar",
                to: Route::Account { notice: String::new() },
                title: "Your account",
                "aria-label": "Your account ({user.display_name()})",
                Avatar { user_id: user.id, url: user.avatar_url.clone(), size: 36 }
            }
            // A form, not a fetch: works before the wasm loads, and the
            // server's redirect home refreshes everything that depends on it.
            form { class: "nav-form", method: "post", action: "/api/auth/logout",
                button { class: "button button-ghost", r#type: "submit", "Sign out" }
            }
        },
        Some(_) => rsx! {
            Link { class: "button button-ghost", to: Route::SignIn { error: String::new() }, "Sign in" }
        },
    };

    rsx! {
        nav { class: "nav-account",
            Link { class: "button button-primary", to: "/post", "Post" }
            {account}
        }
    }
}
