//! `/signin`: continue with Google, or sign in / sign up with a password.
//!
//! Plain forms posting to `/api/auth/*`, like sign-out: they work before the
//! wasm loads, and the server's redirect (home, or back here with
//! `?error=<slug>`) refreshes everything that depends on who's signed in.

use dioxus::prelude::*;

use crate::{
    models::auth::{AuthError, EMAIL_MAX_CHARS, PASSWORD_MAX_CHARS, PASSWORD_MIN_CHARS},
    ui::components::Brand,
};

#[component]
pub fn SignIn(error: String) -> Element {
    // Unknown slugs show nothing: the page never echoes the query string.
    let error = AuthError::from_slug(&error);

    rsx! {
        document::Title { "Sign in · SandHouse" }
        main { class: "auth",
            Brand {}
            if let Some(error) = error {
                p { class: "form-error", role: "alert", "{error.message()}" }
            }

            // A plain link, not the router: the browser has to follow the
            // redirects to Google and back.
            a { class: "button button-ghost auth-link", href: "/api/auth/google/login", "Continue with Google" }

            section { class: "auth-card",
                h2 { "Sign in" }
                form { class: "form", method: "post", action: "/api/auth/login",
                    EmailField { id: "signin-email" }
                    label { r#for: "signin-password", "Password" }
                    input {
                        id: "signin-password",
                        r#type: "password",
                        name: "password",
                        autocomplete: "current-password",
                        maxlength: "{PASSWORD_MAX_CHARS}",
                        required: true,
                    }
                    button { class: "button button-primary", r#type: "submit", "Sign in" }
                }
            }

            section { class: "auth-card",
                h2 { "New here?" }
                form { class: "form", method: "post", action: "/api/auth/register",
                    EmailField { id: "signup-email" }
                    label { r#for: "signup-password", "Password" }
                    input {
                        id: "signup-password",
                        r#type: "password",
                        name: "password",
                        autocomplete: "new-password",
                        minlength: "{PASSWORD_MIN_CHARS}",
                        maxlength: "{PASSWORD_MAX_CHARS}",
                        "aria-describedby": "signup-password-hint",
                        required: true,
                    }
                    p { id: "signup-password-hint", class: "form-hint",
                        "At least {PASSWORD_MIN_CHARS} characters. A few random words work well."
                    }
                    button { class: "button button-primary", r#type: "submit", "Create account" }
                }
            }
        }
    }
}

#[component]
fn EmailField(id: &'static str) -> Element {
    rsx! {
        label { r#for: id, "Email" }
        input {
            id,
            r#type: "email",
            name: "email",
            autocomplete: "email",
            maxlength: "{EMAIL_MAX_CHARS}",
            required: true,
        }
    }
}
