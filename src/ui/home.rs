//! The homepage: what ant is, what's on it, and whether the backend is alive.

use dioxus::prelude::*;

use crate::api;

/// Placeholder listings until the API serves real ones.
struct Listing {
    title: &'static str,
    kind: &'static str,
    blurb: &'static str,
    looking_for: &'static [&'static str],
    funded: bool,
}

const LISTINGS: &[Listing] = &[
    Listing {
        title: "Field recording archive",
        kind: "Passion project",
        blurb: "Mapping a year of city soundscapes. Have the recordings and the map, need someone who enjoys audio tooling.",
        looking_for: &["Rust", "Audio", "Maps"],
        funded: false,
    },
    Listing {
        title: "Co-op puzzle game for two",
        kind: "Game",
        blurb: "Small asymmetric puzzler — one player sees the room, the other sees the rules. Prototype plays well.",
        looking_for: &["Pixel art", "Game design"],
        funded: false,
    },
    Listing {
        title: "Senior capstone: transit delay model",
        kind: "School project",
        blurb: "Predicting bus bunching from open GTFS feeds. Data is cleaned; looking for a second pair of hands on modelling.",
        looking_for: &["Python", "ML", "Data viz"],
        funded: false,
    },
    Listing {
        title: "Local-first invoicing for freelancers",
        kind: "Startup",
        blurb: "Bootstrapped, small revenue already. Budget set aside for a design-minded collaborator.",
        looking_for: &["Design", "Frontend"],
        funded: true,
    },
];

const STEPS: &[(&str, &str)] = &[
    ("Describe it", "A paragraph is enough. What you're building, how far along it is, and what you want a hand with."),
    ("Get matched", "Posts are embedded as vectors, so you surface next to work that's genuinely adjacent — not just sharing a tag."),
    ("Start small", "Trade a message, scope one weekend's worth of work, and see whether you like working together."),
];

#[component]
pub fn Home() -> Element {
    rsx! {
        Nav {}
        main {
            Hero {}
            HowItWorks {}
            Listings {}
        }
        Footer {}
    }
}

#[component]
fn Nav() -> Element {
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
            a { class: "button button-ghost", href: "/api/auth/google/login", "Sign in with Google" }
        },
    };

    rsx! {
        header { class: "nav",
            Link { class: "brand", to: "/",
                span { class: "brand-mark", "🐜" }
                span { class: "brand-name", "ant" }
            }
            nav { class: "nav-links",
                Link { to: "/browse", "Browse" }
                Link { to: "/post", "Post a project" }
                {account}
            }
        }
    }
}

#[component]
fn Hero() -> Element {
    rsx! {
        section { class: "hero",
            p { class: "eyebrow", "Collaborators, not clients" }
            h1 { "Find someone to build the thing with." }
            p { class: "lede",
                "ant is for passion projects, school projects, art, and the odd thing that grew a budget. \
                 Post what you're making, say what you're missing, and get matched with people whose work \
                 actually lines up with yours."
            }
            div { class: "hero-actions",
                Link { class: "button button-primary", to: "/post", "Post a project" }
                Link { class: "button button-ghost", to: "/browse", "Browse projects" }
            }
            BackendStatus {}
        }
    }
}

#[component]
fn HowItWorks() -> Element {
    rsx! {
        section { class: "how",
            h2 { "How it works" }
            ol { class: "steps",
                for (i, (title, body)) in STEPS.iter().enumerate() {
                    li { class: "step", key: "{title}",
                        span { class: "step-number", {(i + 1).to_string()} }
                        h3 { "{title}" }
                        p { "{body}" }
                    }
                }
            }
        }
    }
}

#[component]
fn Listings() -> Element {
    rsx! {
        section { class: "listings",
            div { class: "listings-head",
                h2 { "Open right now" }
                Link { to: "/browse", "See all →" }
            }
            div { class: "cards",
                for listing in LISTINGS {
                    article { class: "card", key: "{listing.title}",
                        div { class: "card-top",
                            span { class: "kind", "{listing.kind}" }
                            if listing.funded {
                                span { class: "funded", "Funded" }
                            }
                        }
                        h3 { "{listing.title}" }
                        p { "{listing.blurb}" }
                        ul { class: "tags",
                            for tag in listing.looking_for {
                                li { class: "tag", "{tag}" }
                            }
                        }
                    }
                }
            }
        }
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
        p { class, span { class: "dot" } "{label}" }
    }
}

#[component]
fn Footer() -> Element {
    rsx! {
        footer { class: "footer",
            span { "ant — a place to find collaborators" }
            // Plain link: a JSON endpoint, not a page the router knows.
            a { href: "/api/health", "status" }
        }
    }
}
