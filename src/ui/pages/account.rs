//! `/account`: your account details, and where you pick a username. New
//! accounts land here after signing up, since they start without one.

use dioxus::prelude::*;

use crate::{
    api,
    models::user::User,
    ui::{Route, components::Brand, pages::FeedQuery},
};

#[component]
pub fn Account() -> Element {
    // Resolved during server rendering, like the nav, so a signed-out visit
    // gets the sign-in link straight away.
    let user = use_server_future(api::current_user)?;

    let body = match &*user.read() {
        None => rsx! { p { class: "feed-note", "Loading…" } },
        Some(Ok(Some(user))) => rsx! { UsernameForm { user: user.clone() } },
        Some(Ok(None)) => rsx! {
            p { class: "feed-note", "You're signed out." }
            Link { class: "button button-primary", to: Route::SignIn { error: String::new() }, "Sign in" }
        },
        Some(Err(err)) => rsx! { p { class: "feed-note feed-error", "Couldn't load your account: {err}" } },
    };

    rsx! {
        document::Title { "Your account · SandHouse" }
        main { class: "auth",
            Brand {}
            {body}
        }
    }
}

#[component]
fn UsernameForm(user: User) -> Element {
    let email = user.email.clone();
    let mut saved = use_signal(|| user.username.clone());
    let mut draft = use_signal(|| user.username.clone().unwrap_or_default());
    let mut error = use_signal(|| None::<String>);
    let mut just_saved = use_signal(|| false);
    let mut saving = use_signal(|| false);

    let unchanged = draft.read().trim() == saved.read().as_deref().unwrap_or("");

    let submit = move |evt: FormEvent| {
        evt.prevent_default();
        // Checked here too so typos get an answer without a round trip; the
        // server has the final say (and knows what's taken).
        let name = draft.read().trim().to_owned();
        if let Err(err) = User::validate_username(&name) {
            error.set(Some(err));
            return;
        }
        saving.set(true);
        spawn(async move {
            match api::account::set_username(name).await {
                Ok(user) => {
                    draft.set(user.username.clone().unwrap_or_default());
                    saved.set(user.username);
                    error.set(None);
                    just_saved.set(true);
                }
                Err(err) => error.set(Some(err.message.unwrap_or_else(|| err.status.to_string()))),
            }
            saving.set(false);
        });
    };

    rsx! {
        section { class: "auth-card",
            h2 {
                if saved.read().is_some() { "Your username" } else { "Pick a username" }
            }
            p { class: "form-hint account-email", "Signed in as {email}" }
            form { class: "form", onsubmit: submit,
                label { r#for: "username", "Username" }
                input {
                    id: "username",
                    name: "username",
                    autocomplete: "username",
                    minlength: "{User::MIN_USERNAME}",
                    maxlength: "{User::MAX_USERNAME}",
                    pattern: "[A-Za-z0-9_\\-]+",
                    "aria-describedby": "username-hint",
                    required: true,
                    value: "{draft}",
                    oninput: move |evt| {
                        draft.set(evt.value());
                        just_saved.set(false);
                    },
                }
                p { id: "username-hint", class: "form-hint",
                    "What others see on your posts. {User::MIN_USERNAME} to {User::MAX_USERNAME} letters, numbers, _ or -."
                }
                if let Some(err) = error() {
                    p { class: "form-error", role: "alert", "{err}" }
                }
                if just_saved() {
                    p { class: "form-saved", role: "status", "Saved." }
                }
                button {
                    class: "button button-primary",
                    r#type: "submit",
                    disabled: saving() || unchanged,
                    if saving() { "Saving…" } else { "Save" }
                }
            }
        }
        Link { class: "button button-ghost auth-link", to: Route::Feed { query: FeedQuery::default() }, "Back to the feed" }
    }
}
