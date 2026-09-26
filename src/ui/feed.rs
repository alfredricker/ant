//! The front page: a feed of open posts with search and filters. No landing
//! page; you arrive in the middle of what people want to do.
//!
//! The search text and filters live in the URL (`/?q=…&kind=…&funded=1`), so
//! results are rendered on the server, links to a search work, and the search
//! form and filter chips work before the wasm has loaded.

use std::fmt;

use chrono::{DateTime, Utc};
use dioxus::prelude::*;

use super::Route;
use crate::{
    api::{self, posts::PAGE_SIZE},
    models::post::{Post, PostKind},
};

/// What the feed shows, as read from and written to the query string.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FeedQuery {
    pub text: String,
    pub kind: Option<PostKind>,
    pub funded: bool,
}

impl FeedQuery {
    fn with_kind(&self, kind: Option<PostKind>) -> Self {
        Self { kind, ..self.clone() }
    }

    fn with_funded(&self, funded: bool) -> Self {
        Self { funded, ..self.clone() }
    }

    fn is_filtered(&self) -> bool {
        *self != Self::default()
    }
}

impl From<&str> for FeedQuery {
    fn from(query: &str) -> Self {
        let mut feed = FeedQuery::default();
        let mut after_text = false;
        for pair in query.split('&') {
            match pair.split_once('=') {
                // Browsers send spaces in form fields as `+`.
                Some(("q", text)) => {
                    feed.text = text.replace('+', " ");
                    after_text = true;
                    continue;
                }
                Some(("kind", kind)) => feed.kind = PostKind::from_slug(kind),
                Some(("funded", funded)) => feed.funded = matches!(funded, "1" | "true" | "on"),
                // The router percent-decodes the whole query before we see
                // it, so an `&` typed into the search box splits the text;
                // glue the pieces back on.
                _ if after_text => {
                    feed.text.push('&');
                    feed.text.push_str(&pair.replace('+', " "));
                    continue;
                }
                _ => {}
            }
            after_text = false;
        }
        feed
    }
}

impl fmt::Display for FeedQuery {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut pairs = Vec::new();
        if let Some(kind) = self.kind {
            pairs.push(format!("kind={}", kind.slug()));
        }
        if self.funded {
            pairs.push("funded=1".to_owned());
        }
        // Last, so an `&` in it can only swallow the end of the query.
        let text = self.text.trim();
        if !text.is_empty() {
            pairs.push(format!("q={text}"));
        }
        f.write_str(&pairs.join("&"))
    }
}

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
fn TopBar(query: FeedQuery) -> Element {
    rsx! {
        header { class: "topbar",
            Link { class: "brand", to: Route::Feed { query: FeedQuery::default() },
                span { class: "brand-mark", "🐜" }
                span { class: "brand-name", "ant" }
            }
            SearchBox { query }
            Account {}
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
fn Account() -> Element {
    // Resolved during server rendering (the session cookie comes along), so
    // the page arrives already showing the right account state.
    let user = use_server_future(api::current_user)?;

    let account = match &*user.read() {
        None => rsx! {},
        Some(Ok(Some(user))) => rsx! {
            span { class: "nav-user", "{user.display_name()}" }
            // A form, not a fetch: works before the wasm loads, and the
            // server's redirect home refreshes everything that depends on it.
            form { class: "nav-form", method: "post", action: "/api/auth/logout",
                button { class: "button button-ghost", r#type: "submit", "Sign out" }
            }
        },
        // A plain link, not the router: the browser has to follow the
        // redirects to Google and back.
        Some(_) => rsx! {
            a { class: "button button-ghost", href: "/api/auth/google/login", "Sign in" }
        },
    };

    rsx! {
        nav { class: "account",
            Link { class: "button button-primary", to: "/post", "Post" }
            {account}
        }
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

#[component]
fn PostCard(post: Post) -> Element {
    let author = post.author.display_name().to_owned();
    let initial = author.chars().next().unwrap_or('?').to_uppercase().to_string();

    rsx! {
        article { class: "post",
            header { class: "post-head",
                span { class: "avatar", "aria-hidden": "true", "{initial}" }
                div { class: "post-meta",
                    span { class: "post-author", "{author}" }
                    span { class: "post-sub",
                        "{post.kind.label()} · "
                        time { datetime: "{post.created_at.to_rfc3339()}", "{ago(post.created_at)}" }
                    }
                }
                if post.funded {
                    span { class: "funded", "Funded" }
                }
            }
            h2 { class: "post-title", "{post.title}" }
            p { class: "post-body", "{post.body}" }
            if let Some(url) = &post.image_url {
                img { class: "post-image", src: "{url}", alt: "", loading: "lazy" }
            }
            if !post.looking_for.is_empty() {
                div { class: "post-wants",
                    span { class: "post-wants-label", "Looking for" }
                    ul { class: "tags",
                        for tag in &post.looking_for {
                            li { class: "tag", key: "{tag}", "{tag}" }
                        }
                    }
                }
            }
            footer { class: "post-foot",
                span { class: if post.liked { "count count-liked" } else { "count" },
                    "♥ {post.like_count}"
                }
                span { class: "count", "💬 {post.comment_count}" }
            }
        }
    }
}

/// "5m", "3h", "2d", then the date.
fn ago(at: DateTime<Utc>) -> String {
    let secs = (Utc::now() - at).num_seconds().max(0);
    match secs {
        0..60 => "just now".to_owned(),
        60..3_600 => format!("{}m", secs / 60),
        3_600..86_400 => format!("{}h", secs / 3_600),
        86_400..604_800 => format!("{}d", secs / 86_400),
        _ => at.format("%b %-d").to_string(),
    }
}

/// Small dev affordance: proves the server, the wasm bundle and postgres are
/// all actually talking to each other.
#[component]
fn BackendStatus() -> Element {
    let health = use_server_future(api::status)?;

    let (class, label) = match &*health.read() {
        None => ("status status-pending", "checking backend…".to_string()),
        Some(Ok(h)) if h.is_ok() => (
            "status status-ok",
            match &h.pgvector {
                Some(version) => format!("api ok · postgres {} · pgvector {version}", h.database),
                None => format!("api ok · postgres {} · pgvector missing", h.database),
            },
        ),
        Some(Ok(h)) => ("status status-down", format!("api {} · postgres {}", h.status, h.database)),
        Some(Err(err)) => ("status status-down", format!("api {err}")),
    };

    rsx! {
        span { class, span { class: "dot" } "{label}" }
    }
}

#[component]
fn Footer() -> Element {
    rsx! {
        footer { class: "footer",
            span { "ant: find people who want the same big thing" }
            // Plain link: a JSON endpoint, not a page the router knows.
            a { href: "/api/health", BackendStatus {} }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_round_trips() {
        let query = FeedQuery {
            text: "rust audio".into(),
            kind: Some(PostKind::Travel),
            funded: true,
        };
        assert_eq!(query.to_string(), "kind=travel&funded=1&q=rust audio");
        assert_eq!(FeedQuery::from(query.to_string().as_str()), query);
        assert_eq!(FeedQuery::from(""), FeedQuery::default());
    }

    #[test]
    fn query_reads_form_submissions() {
        let query = FeedQuery::from("q=rock+climbing&kind=nope&funded=on");
        assert_eq!(query.text, "rock climbing");
        assert_eq!(query.kind, None);
        assert!(query.funded);
    }

    #[test]
    fn query_keeps_ampersands_in_search_text() {
        assert_eq!(FeedQuery::from("kind=art&q=R&D lab").text, "R&D lab");
    }
}
