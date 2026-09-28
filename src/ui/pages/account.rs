//! `/account`: your photo, username, email, password, theme and devices.
//! New accounts land here after signing up, since they start without a
//! username.
//!
//! The photo form is a plain multipart post (file bytes don't fit server
//! functions), which comes back as `/account?notice=<slug>`. Everything else
//! goes through server functions and updates in place.

use dioxus::prelude::*;

use crate::{
    api,
    models::{
        account::{AVATAR_MAX_BYTES, AccountDetails, AvatarNotice, Theme},
        auth::{EMAIL_MAX_CHARS, PASSWORD_MAX_CHARS, PASSWORD_MIN_CHARS},
        user::User,
    },
    ui::{
        Route,
        components::{Avatar, Brand},
        error_text,
        pages::FeedQuery,
        use_theme,
    },
};

#[component]
pub fn Account(notice: String) -> Element {
    // Resolved during server rendering, like the nav, so a signed-out visit
    // gets the sign-in link straight away.
    let details = use_server_future(|| async {
        api::account::account_details().await.map_err(|err| error_text(&err))
    })?;

    let body = match &*details.read() {
        None => rsx! { p { class: "feed-note", "Loading…" } },
        Some(Ok(Some(details))) => rsx! {
            Settings { details: details.clone(), notice: AvatarNotice::from_slug(&notice) }
        },
        Some(Ok(None)) => rsx! {
            div { class: "card account-signed-out",
                p { "You're signed out." }
                Link { class: "button button-primary", to: Route::SignIn { error: String::new() }, "Sign in" }
            }
        },
        Some(Err(err)) => rsx! { p { class: "feed-note feed-error", "Couldn't load your account: {err}" } },
    };

    rsx! {
        document::Title { "Your account · SandHouse" }
        main { class: "account-page",
            Brand {}
            h1 { "Your account" }
            {body}
            Link { class: "button button-ghost account-back", to: Route::Feed { query: FeedQuery::default() }, "Back to the feed" }
        }
    }
}

#[component]
fn Settings(details: AccountDetails, notice: Option<AvatarNotice>) -> Element {
    // Shared by the sections, so a change in one (say, the email) shows up
    // everywhere it's displayed.
    let user = use_signal(|| details.user.clone());
    let has_password = use_signal(|| details.has_password);

    rsx! {
        ProfileSection { user, notice }
        SignInSection { user, has_password, google_linked: details.google_linked }
        AppearanceSection {}
        DevicesSection {}
    }
}

// ---------------------------------------------------------------------------
// Profile: photo and username
// ---------------------------------------------------------------------------

#[component]
fn ProfileSection(user: Signal<User>, notice: Option<AvatarNotice>) -> Element {
    let current = user.read().clone();
    let max_mb = AVATAR_MAX_BYTES / (1024 * 1024);

    rsx! {
        section { class: "card", "aria-labelledby": "profile-heading",
            h2 { id: "profile-heading", "Profile" }
            div { class: "account-photo",
                Avatar { user_id: current.id, url: current.avatar_url.clone(), size: 88 }
                div { class: "account-photo-actions",
                    form {
                        class: "account-photo-form",
                        method: "post",
                        action: "/api/account/avatar",
                        enctype: "multipart/form-data",
                        label { r#for: "avatar", "Photo" }
                        input {
                            id: "avatar",
                            r#type: "file",
                            name: "avatar",
                            accept: "image/jpeg,image/png,image/webp,image/gif",
                            "aria-describedby": "avatar-hint",
                            required: true,
                        }
                        p { id: "avatar-hint", class: "form-hint",
                            "JPEG, PNG, WebP or GIF, up to {max_mb} MB. We crop it to a square."
                        }
                        div { class: "account-actions",
                            button { class: "button button-primary", r#type: "submit", "Upload photo" }
                            if current.avatar_url.is_some() {
                                button {
                                    class: "button button-ghost",
                                    r#type: "submit",
                                    // Same form, other endpoint: no file needed.
                                    formaction: "/api/account/avatar/remove",
                                    formnovalidate: true,
                                    "Remove photo"
                                }
                            }
                        }
                    }
                }
            }
            if let Some(notice) = notice {
                p {
                    class: if notice.is_error() { "form-error" } else { "form-saved" },
                    role: if notice.is_error() { "alert" } else { "status" },
                    "{notice.message()}"
                }
            }
            hr { class: "account-divider" }
            UsernameForm { user }
        }
    }
}

#[component]
fn UsernameForm(user: Signal<User>) -> Element {
    let mut draft = use_signal(|| user.read().username.clone().unwrap_or_default());
    let mut status = use_signal(|| FormStatus::Idle);

    let saved = user.read().username.clone();
    let unchanged = draft.read().trim() == saved.as_deref().unwrap_or("");

    let submit = move |evt: FormEvent| {
        evt.prevent_default();
        // Checked here too so typos get an answer without a round trip; the
        // server has the final say (and knows what's taken).
        let name = draft.read().trim().to_owned();
        if let Err(err) = User::validate_username(&name) {
            status.set(FormStatus::Failed(err));
            return;
        }
        status.set(FormStatus::Saving);
        spawn(async move {
            match api::account::set_username(name).await {
                Ok(updated) => {
                    draft.set(updated.username.clone().unwrap_or_default());
                    user.set(updated);
                    status.set(FormStatus::Saved("Saved."));
                }
                Err(err) => status.set(FormStatus::Failed(error_text(&err))),
            }
        });
    };

    rsx! {
        form { class: "form", onsubmit: submit,
            label { r#for: "username",
                if saved.is_some() { "Username" } else { "Pick a username" }
            }
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
                    status.set(FormStatus::Idle);
                },
            }
            p { id: "username-hint", class: "form-hint",
                "What others see on your posts. {User::MIN_USERNAME} to {User::MAX_USERNAME} letters, numbers, _ or -."
            }
            StatusLine { status }
            SubmitButton { status, disabled: unchanged, label: "Save username" }
        }
    }
}

// ---------------------------------------------------------------------------
// Signing in: email and password
// ---------------------------------------------------------------------------

#[component]
fn SignInSection(user: Signal<User>, has_password: Signal<bool>, google_linked: bool) -> Element {
    let email_note = match (user.read().email_verified, google_linked) {
        (true, _) => "Verified by Google.",
        (false, true) => "Not verified. Signing in with Google under this address will verify it.",
        (false, false) => "Not verified yet.",
    };

    rsx! {
        section { class: "card", "aria-labelledby": "signin-heading",
            h2 { id: "signin-heading", "Signing in" }
            p { class: "form-hint account-methods",
                match (google_linked, has_password()) {
                    (true, true) => "You can sign in with Google or with your email and password.",
                    (true, false) => "You sign in with Google. Set a password below to also sign in with your email.",
                    (false, _) => "You sign in with your email and password.",
                }
            }
            EmailForm { user, has_password, note: email_note }
            hr { class: "account-divider" }
            PasswordForm { has_password }
        }
    }
}

#[component]
fn EmailForm(user: Signal<User>, has_password: Signal<bool>, note: &'static str) -> Element {
    let mut email = use_signal(|| user.read().email.clone());
    let mut current_password = use_signal(String::new);
    let mut status = use_signal(|| FormStatus::Idle);

    let unchanged = email.read().trim() == user.read().email;

    let submit = move |evt: FormEvent| {
        evt.prevent_default();
        let new_email = email.read().trim().to_owned();
        let password = has_password().then(|| current_password.read().clone());
        status.set(FormStatus::Saving);
        spawn(async move {
            match api::account::change_email(new_email, password).await {
                Ok(updated) => {
                    email.set(updated.email.clone());
                    user.set(updated);
                    current_password.set(String::new());
                    status.set(FormStatus::Saved("Email changed."));
                }
                Err(err) => status.set(FormStatus::Failed(error_text(&err))),
            }
        });
    };

    rsx! {
        form { class: "form", onsubmit: submit,
            label { r#for: "email", "Email" }
            input {
                id: "email",
                r#type: "email",
                name: "email",
                autocomplete: "email",
                maxlength: "{EMAIL_MAX_CHARS}",
                "aria-describedby": "email-hint",
                required: true,
                value: "{email}",
                oninput: move |evt| {
                    email.set(evt.value());
                    status.set(FormStatus::Idle);
                },
            }
            p { id: "email-hint", class: "form-hint", "{note}" }
            // Only when it's about to change, to keep the form short.
            if has_password() && !unchanged {
                label { r#for: "email-password", "Current password" }
                input {
                    id: "email-password",
                    r#type: "password",
                    autocomplete: "current-password",
                    maxlength: "{PASSWORD_MAX_CHARS}",
                    required: true,
                    value: "{current_password}",
                    oninput: move |evt| current_password.set(evt.value()),
                }
            }
            StatusLine { status }
            SubmitButton { status, disabled: unchanged, label: "Change email" }
        }
    }
}

#[component]
fn PasswordForm(has_password: Signal<bool>) -> Element {
    let mut current_password = use_signal(String::new);
    let mut new_password = use_signal(String::new);
    let mut status = use_signal(|| FormStatus::Idle);

    let submit = move |evt: FormEvent| {
        evt.prevent_default();
        let current = has_password().then(|| current_password.read().clone());
        let new = new_password();
        status.set(FormStatus::Saving);
        spawn(async move {
            match api::account::change_password(current, new).await {
                Ok(()) => {
                    current_password.set(String::new());
                    new_password.set(String::new());
                    status.set(FormStatus::Saved(if has_password() {
                        "Password changed. Your other devices were signed out."
                    } else {
                        "Password set. Your other devices were signed out."
                    }));
                    has_password.set(true);
                }
                Err(err) => status.set(FormStatus::Failed(error_text(&err))),
            }
        });
    };

    let submit_label = if has_password() { "Change password" } else { "Set password" };

    rsx! {
        form { class: "form", onsubmit: submit,
            if has_password() {
                label { r#for: "current-password", "Current password" }
                input {
                    id: "current-password",
                    r#type: "password",
                    autocomplete: "current-password",
                    maxlength: "{PASSWORD_MAX_CHARS}",
                    required: true,
                    value: "{current_password}",
                    oninput: move |evt| current_password.set(evt.value()),
                }
            }
            label { r#for: "new-password",
                if has_password() { "New password" } else { "Password" }
            }
            input {
                id: "new-password",
                r#type: "password",
                autocomplete: "new-password",
                minlength: "{PASSWORD_MIN_CHARS}",
                maxlength: "{PASSWORD_MAX_CHARS}",
                "aria-describedby": "new-password-hint",
                required: true,
                value: "{new_password}",
                oninput: move |evt| {
                    new_password.set(evt.value());
                    status.set(FormStatus::Idle);
                },
            }
            p { id: "new-password-hint", class: "form-hint",
                "At least {PASSWORD_MIN_CHARS} characters. A few random words work well."
            }
            StatusLine { status }
            SubmitButton { status, disabled: false, label: submit_label }
        }
    }
}

// ---------------------------------------------------------------------------
// Appearance
// ---------------------------------------------------------------------------

#[component]
fn AppearanceSection() -> Element {
    let mut theme = use_theme();
    let mut error = use_signal(|| None::<String>);

    rsx! {
        section { class: "card", "aria-labelledby": "appearance-heading",
            h2 { id: "appearance-heading", "Appearance" }
            fieldset { class: "theme-options",
                legend { class: "form-hint", "Theme" }
                for option in Theme::ALL {
                    label { key: "{option.slug()}", class: "theme-option",
                        input {
                            r#type: "radio",
                            name: "theme",
                            value: option.slug(),
                            checked: theme() == option,
                            onchange: move |_| {
                                // Restyle now; saving can catch up.
                                let before = theme();
                                theme.set(option);
                                spawn(async move {
                                    match api::account::set_theme(option).await {
                                        Ok(()) => error.set(None),
                                        Err(err) => {
                                            theme.set(before);
                                            error.set(Some(error_text(&err)));
                                        }
                                    }
                                });
                            },
                        }
                        ThemeSwatch { theme: option }
                        span { "{option.label()}" }
                    }
                }
            }
            if let Some(err) = error() {
                p { class: "form-error", role: "alert", "Couldn't save your theme: {err}" }
            }
        }
    }
}

/// A tiny page in the theme's colors. "Match system" shows half of each.
#[component]
fn ThemeSwatch(theme: Theme) -> Element {
    let schemes: &[&str] = match theme {
        Theme::System => &["light", "dark"],
        Theme::Light => &["light"],
        Theme::Dark => &["dark"],
    };
    rsx! {
        span { class: "theme-swatch", "aria-hidden": "true",
            for scheme in schemes {
                span { key: "{scheme}", class: "theme-swatch-half", "data-scheme": *scheme,
                    span { class: "theme-swatch-bar" }
                    span { class: "theme-swatch-dot" }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Devices
// ---------------------------------------------------------------------------

#[component]
fn DevicesSection() -> Element {
    let mut status = use_signal(|| FormStatus::Idle);

    let sign_out_others = move |_| {
        status.set(FormStatus::Saving);
        spawn(async move {
            match api::account::sign_out_other_devices().await {
                Ok(0) => status.set(FormStatus::Saved("You weren't signed in anywhere else.")),
                Ok(_) => status.set(FormStatus::Saved("Signed out everywhere else.")),
                Err(err) => status.set(FormStatus::Failed(error_text(&err))),
            }
        });
    };

    rsx! {
        section { class: "card", "aria-labelledby": "devices-heading",
            h2 { id: "devices-heading", "Devices" }
            p { class: "form-hint account-methods",
                "Signed in on a computer you don't use anymore? Sign it out from here."
            }
            StatusLine { status }
            div { class: "account-actions",
                button {
                    class: "button button-ghost",
                    r#type: "button",
                    disabled: status() == FormStatus::Saving,
                    onclick: sign_out_others,
                    "Sign out other devices"
                }
                // A form, like the nav's: works before the wasm loads.
                form { class: "nav-form", method: "post", action: "/api/auth/logout",
                    button { class: "button button-ghost", r#type: "submit", "Sign out" }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Form plumbing
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
enum FormStatus {
    Idle,
    Saving,
    Saved(&'static str),
    Failed(String),
}

#[component]
fn StatusLine(status: Signal<FormStatus>) -> Element {
    match status() {
        FormStatus::Saved(message) => rsx! { p { class: "form-saved", role: "status", "{message}" } },
        FormStatus::Failed(message) => rsx! { p { class: "form-error", role: "alert", "{message}" } },
        FormStatus::Idle | FormStatus::Saving => rsx! {},
    }
}

#[component]
fn SubmitButton(status: Signal<FormStatus>, disabled: bool, label: &'static str) -> Element {
    let saving = status() == FormStatus::Saving;
    rsx! {
        button {
            class: "button button-primary",
            r#type: "submit",
            disabled: saving || disabled,
            if saving { "Saving…" } else { "{label}" }
        }
    }
}
