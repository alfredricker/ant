//! The front page: a feed of open posts with search and filters. No landing
//! page; you arrive in the middle of what people want to do.

use chrono::{DateTime, Utc};
use dioxus::prelude::*;

mod query;

pub use query::FeedQuery;

use crate::{
    api::{self, posts::PAGE_SIZE},
    models::post::{Post, PostKind},
    ui::{
        Route,
        components::{Footer, PostCard, TopBar},
    },
};

#[component]
pub fn Feed(query: FeedQuery) -> Element {
    rsx! {
        TopBar { query: query.clone() }
        main { class: "feed",
            Filters { query: query.clone() }
            // Keyed on the query, so a new search starts from a fresh first
            // page instead of appending to the old one.
            PostList { key: "{query}", query }
        }
        Footer {}
    }
}

#[component]
fn Filters(query: FeedQuery) -> Element {
    rsx! {
        nav { class: "filters", "aria-label": "Filter posts",
            ul { class: "chips",
                li {
                    Chip {
                        to: query.with_kind(None),
                        active: query.kind.is_none(),
                        label: "Everything",
                    }
                }
                for kind in PostKind::ALL {
                    li { key: "{kind.slug()}",
                        Chip {
                            to: query.with_kind(Some(kind)),
                            active: query.kind == Some(kind),
                            label: kind.label(),
                        }
                    }
                }
            }
            Chip {
                to: query.with_funded(!query.funded),
                active: query.funded,
                label: "Funded only",
            }
        }
    }
}

#[component]
fn Chip(to: FeedQuery, active: bool, label: &'static str) -> Element {
    rsx! {
        Link {
            class: if active { "chip chip-active" } else { "chip" },
            to: Route::Feed { query: to },
            "{label}"
        }
    }
}

#[component]
fn PostList(query: FeedQuery) -> Element {
    let first = {
        let query = query.clone();
        use_server_future(move || fetch_page(query.clone(), None))?
    };
    // Pages after the first, fetched in the browser by "Show more".
    let mut more = use_signal(Vec::<Post>::new);
    let mut exhausted = use_signal(|| false);
    let mut loading = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);

    let first = match &*first.read() {
        None => return rsx! { p { class: "feed-note", "Loading…" } },
        Some(Err(err)) => return rsx! { p { class: "feed-note feed-error", "Couldn't load posts: {err}" } },
        Some(Ok(posts)) => posts.clone(),
    };

    if first.is_empty() {
        return rsx! {
            div { class: "feed-empty",
                p { "Nobody's posted that yet." }
                if query.is_filtered() {
                    Link { to: Route::Feed { query: FeedQuery::default() }, "Clear search and filters" }
                }
            }
        };
    }

    let has_more = first.len() as i64 == PAGE_SIZE && !exhausted();
    let first_cursor = first.last().map(|post| post.created_at);
    let show_more = move |_| {
        let cursor = more.read().last().map(|post| post.created_at).or(first_cursor);
        let query = query.clone();
        loading.set(true);
        spawn(async move {
            match fetch_page(query, cursor).await {
                Ok(page) => {
                    if (page.len() as i64) < PAGE_SIZE {
                        exhausted.set(true);
                    }
                    more.write().extend(page);
                    error.set(None);
                }
                Err(err) => error.set(Some(err)),
            }
            loading.set(false);
        });
    };

    rsx! {
        div { class: "posts",
            for post in first.iter().chain(more.read().iter()) {
                PostCard { key: "{post.id}", post: post.clone() }
            }
        }
        if let Some(err) = error() {
            p { class: "feed-note feed-error", "Couldn't load more: {err}" }
        }
        if has_more {
            button {
                class: "button button-ghost feed-more",
                disabled: loading(),
                onclick: show_more,
                if loading() { "Loading…" } else { "Show more" }
            }
        }
    }
}

async fn fetch_page(query: FeedQuery, before: Option<DateTime<Utc>>) -> Result<Vec<Post>, String> {
    let text = Some(query.text.trim().to_owned()).filter(|t| !t.is_empty());
    let funded = query.funded.then_some(true);
    api::posts::list_posts(query.kind, funded, text, None, before)
        .await
        .map_err(|err| err.to_string())
}
