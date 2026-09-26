use dioxus::prelude::*;

use crate::api;

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
pub fn Footer() -> Element {
    rsx! {
        footer { class: "footer",
            span { "SandHouse: find people who want the same big thing" }
            // Plain link: a JSON endpoint, not a page the router knows.
            a { href: "/api/health", BackendStatus {} }
        }
    }
}
