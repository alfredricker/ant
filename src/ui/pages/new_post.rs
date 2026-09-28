//! `/post`: write a post. What kind it is, a title, the description, tags
//! for who you're looking for, and optionally a picture; "Post" sends it and
//! goes back to the feed, where it's now on top.
//!
//! The picture uploads as soon as it's picked, so the preview is what the
//! server made of it and posting doesn't wait on it. Unlike the account
//! page's photo form, this needs the wasm; the Post button stays disabled
//! until it has loaded.

use dioxus::prelude::*;

use crate::{
    api,
    models::{
        post::{POST_IMAGE_MAX_BYTES, PostInput, PostKind},
        user::User,
    },
    ui::{Route, components::Brand, error_text, pages::FeedQuery},
};

#[component]
pub fn NewPost() -> Element {
    let user = use_server_future(api::current_user)?;

    let body = match &*user.read() {
        None => rsx! { p { class: "feed-note", "Loading…" } },
        Some(Ok(Some(user))) => rsx! { Composer { user: user.clone() } },
        Some(Ok(None)) => rsx! {
            div { class: "card account-signed-out",
                p { "Sign in to post." }
                Link { class: "button button-primary", to: Route::SignIn { error: String::new() }, "Sign in" }
            }
        },
        Some(Err(err)) => rsx! { p { class: "feed-note feed-error", "Couldn't check who's signed in: {err}" } },
    };

    rsx! {
        document::Title { "New post · SandHouse" }
        main { class: "compose-page",
            Brand {}
            h1 { "New post" }
            {body}
            Link { class: "button button-ghost compose-back", to: Route::Feed { query: FeedQuery::default() }, "Back to the feed" }
        }
    }
}

#[component]
fn Composer(user: User) -> Element {
    let mut kind = use_signal(|| None::<PostKind>);
    let mut title = use_signal(String::new);
    let mut body = use_signal(String::new);
    let mut funded = use_signal(|| false);
    let tags = use_signal(Vec::<String>::new);
    // A tag still being typed; `submit` counts it too.
    let tag_draft = use_signal(String::new);
    let image_url = use_signal(|| None::<String>);
    let uploading = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let mut posting = use_signal(|| false);
    // Only true in the browser once the wasm runs; before that the form
    // can't upload or post.
    let mut hydrated = use_signal(|| false);
    use_effect(move || hydrated.set(true));

    let submit = move |evt: FormEvent| {
        evt.prevent_default();
        if uploading() || posting() {
            return;
        }
        if let Err(err) = finish_draft(tags, tag_draft) {
            error.set(Some(err));
            return;
        }
        let Some(kind) = kind() else {
            error.set(Some("pick what kind of post this is".into()));
            return;
        };
        let mut input = PostInput {
            kind,
            title: title(),
            body: body(),
            looking_for: tags(),
            funded: funded(),
            image_url: image_url(),
        };
        // Same checks as the server, to answer without a round trip.
        input.normalize();
        if let Err(err) = input.validate() {
            error.set(Some(err));
            return;
        }

        error.set(None);
        posting.set(true);
        spawn(async move {
            match api::posts::create_post(input).await {
                Ok(_) => {
                    navigator().push(Route::Feed { query: FeedQuery::default() });
                }
                Err(err) => {
                    error.set(Some(error_text(&err)));
                    posting.set(false);
                }
            }
        });
    };

    rsx! {
        form { class: "card form compose", onsubmit: submit,
            if user.username.is_none() {
                p { class: "compose-note",
                    "You haven't picked a username, so this will show as \"someone\". "
                    Link { to: Route::Account { notice: String::new() }, "Pick one" }
                }
            }

            label { r#for: "post-kind", "Kind" }
            select {
                id: "post-kind",
                name: "kind",
                required: true,
                onchange: move |evt| kind.set(PostKind::from_slug(&evt.value())),
                option { value: "", disabled: true, selected: kind().is_none(), "Pick one" }
                for option in PostKind::ALL {
                    option {
                        key: "{option.slug()}",
                        value: option.slug(),
                        selected: kind() == Some(option),
                        "{option.label()}"
                    }
                }
            }

            label { r#for: "post-title", "Title" }
            input {
                id: "post-title",
                name: "title",
                placeholder: "Walk the Camino de Santiago next spring",
                maxlength: "{PostInput::MAX_TITLE}",
                required: true,
                value: "{title}",
                oninput: move |evt| title.set(evt.value()),
            }

            label { r#for: "post-body", "Description" }
            textarea {
                id: "post-body",
                name: "body",
                rows: "8",
                placeholder: "What you want to do, where it's at, and what you'd like from whoever joins.",
                maxlength: "{PostInput::MAX_BODY}",
                required: true,
                value: "{body}",
                oninput: move |evt| body.set(evt.value()),
            }

            TagField { tags, draft: tag_draft }

            ImageField { image_url, uploading, enabled: hydrated() }

            label { class: "compose-check",
                input {
                    r#type: "checkbox",
                    name: "funded",
                    checked: funded(),
                    onchange: move |evt| funded.set(evt.checked()),
                }
                "There's money behind it"
            }
            p { class: "form-hint", "A budget, a grant, or pay for whoever joins. Funded posts get a badge." }

            if let Some(err) = error() {
                p { class: "form-error", role: "alert", "{err}" }
            }
            button {
                class: "button button-primary compose-submit",
                r#type: "submit",
                disabled: !hydrated() || uploading() || posting(),
                if posting() { "Posting…" } else if uploading() { "Uploading picture…" } else { "Post" }
            }
        }
    }
}

/// Tags as chips, plus a box for the next one: Enter or a comma adds it,
/// Backspace in the empty box takes the last one back.
#[component]
fn TagField(tags: Signal<Vec<String>>, draft: Signal<String>) -> Element {
    let mut error = use_signal(|| None::<String>);
    let mut draft = draft;
    let mut tags = tags;

    rsx! {
        label { r#for: "post-tags", "Looking for" }
        div { class: "tag-field",
            ul { class: "tags",
                for (i, tag) in tags.read().iter().enumerate() {
                    li { class: "tag", key: "{tag}",
                        "{tag}"
                        button {
                            class: "tag-remove",
                            r#type: "button",
                            "aria-label": "Remove {tag}",
                            onclick: move |_| {
                                tags.write().remove(i);
                                error.set(None);
                            },
                            "×"
                        }
                    }
                }
            }
            input {
                id: "post-tags",
                placeholder: if tags.read().is_empty() { "Rust, Pixel art, a drummer…" } else { "" },
                "aria-describedby": "post-tags-hint",
                value: "{draft}",
                oninput: move |evt| {
                    // Typed or pasted commas finish the tags before them.
                    let value = evt.value();
                    let Some((done, rest)) = value.rsplit_once(',') else {
                        draft.set(value);
                        error.set(None);
                        return;
                    };
                    match add_tags(&mut tags.write(), done) {
                        Ok(()) => {
                            draft.set(rest.to_owned());
                            error.set(None);
                        }
                        Err((err, left)) => {
                            draft.set(format!("{left},{rest}"));
                            error.set(Some(err));
                        }
                    }
                },
                onkeydown: move |evt| match evt.key() {
                    Key::Enter => {
                        // Not a submit: finish the tag instead.
                        evt.prevent_default();
                        error.set(finish_draft(tags, draft).err());
                    }
                    Key::Backspace if draft.read().is_empty() => {
                        tags.write().pop();
                        error.set(None);
                    }
                    _ => {}
                },
            }
        }
        p { id: "post-tags-hint", class: "form-hint",
            "Skills or people you need, up to {PostInput::MAX_TAGS}. Press Enter or type a comma after each."
        }
        if let Some(err) = error() {
            p { class: "form-error", role: "alert", "{err}" }
        }
    }
}

/// Turns what's in the tag box into tags. Whatever doesn't fit stays in the
/// box.
fn finish_draft(mut tags: Signal<Vec<String>>, mut draft: Signal<String>) -> Result<(), String> {
    let text = draft.peek().clone();
    match add_tags(&mut tags.write(), &text) {
        Ok(()) => {
            draft.set(String::new());
            Ok(())
        }
        Err((err, left)) => {
            draft.set(left);
            Err(err)
        }
    }
}

/// Adds each comma-separated tag in `text`. Stops at the first that doesn't
/// fit, and returns why along with that tag and the rest, to put back in the
/// box so nothing typed is lost.
fn add_tags(tags: &mut Vec<String>, text: &str) -> Result<(), (String, String)> {
    let mut parts = text.split(',');
    while let Some(part) = parts.next() {
        if let Err(err) = add_tag(tags, part) {
            let left: Vec<&str> = std::iter::once(part).chain(parts).collect();
            return Err((err, left.join(",").trim_start().to_owned()));
        }
    }
    Ok(())
}

/// `PostInput::normalize` would drop blanks and duplicates too; doing it
/// here keeps the chips honest.
fn add_tag(tags: &mut Vec<String>, tag: &str) -> Result<(), String> {
    let tag = tag.trim();
    if tag.is_empty() || tags.iter().any(|t| t.eq_ignore_ascii_case(tag)) {
        return Ok(());
    }
    if tag.chars().count() > PostInput::MAX_TAG {
        return Err(format!("tags are at most {} characters", PostInput::MAX_TAG));
    }
    if tags.len() >= PostInput::MAX_TAGS {
        return Err(format!("at most {} tags", PostInput::MAX_TAGS));
    }
    tags.push(tag.to_owned());
    Ok(())
}

/// Pick a picture and it uploads straight away; then shows the preview and
/// a button to drop it.
#[component]
fn ImageField(image_url: Signal<Option<String>>, uploading: Signal<bool>, enabled: bool) -> Element {
    let mut image_url = image_url;
    let mut uploading = uploading;
    let mut error = use_signal(|| None::<String>);
    // Bumped after every pick, so the file input comes back empty and
    // picking the same file again still counts as a change.
    let mut picks = use_signal(|| 0u32);
    let max_mb = POST_IMAGE_MAX_BYTES / (1024 * 1024);

    let pick = move |evt: FormEvent| {
        let Some(file) = evt.files().into_iter().next() else {
            return;
        };
        if file.size() > POST_IMAGE_MAX_BYTES as u64 {
            error.set(Some(format!("that file is too big; pictures can be up to {max_mb} MB")));
            picks += 1;
            return;
        }
        error.set(None);
        uploading.set(true);
        spawn(async move {
            match api::posts::upload_post_image(file.into()).await {
                Ok(url) => image_url.set(Some(url)),
                Err(err) => error.set(Some(error_text(&err))),
            }
            uploading.set(false);
            picks += 1;
        });
    };

    rsx! {
        span { class: "compose-label", id: "post-image-label", "Picture (optional)" }
        if let Some(url) = image_url() {
            div { class: "compose-image",
                img { src: "{url}", alt: "The picture you picked" }
                button {
                    class: "button button-ghost",
                    r#type: "button",
                    onclick: move |_| image_url.set(None),
                    "Remove picture"
                }
            }
        } else {
            input {
                key: "{picks}",
                class: "compose-file",
                r#type: "file",
                accept: "image/jpeg,image/png,image/webp,image/gif",
                "aria-labelledby": "post-image-label",
                "aria-describedby": "post-image-hint",
                disabled: !enabled || uploading(),
                onchange: pick,
            }
            p { id: "post-image-hint", class: "form-hint",
                if uploading() { "Uploading…" } else { "JPEG, PNG, WebP or GIF, up to {max_mb} MB." }
            }
        }
        if let Some(err) = error() {
            p { class: "form-error", role: "alert", "{err}" }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_tag_skips_blanks_and_duplicates() {
        let mut tags = vec!["Rust".to_owned()];
        add_tag(&mut tags, "  ").unwrap();
        add_tag(&mut tags, " rust ").unwrap();
        add_tag(&mut tags, " Pixel art ").unwrap();
        assert_eq!(tags, ["Rust", "Pixel art"]);
    }

    #[test]
    fn add_tags_keeps_what_does_not_fit() {
        let mut tags: Vec<String> = (1..PostInput::MAX_TAGS).map(|i| format!("tag {i}")).collect();
        let (_, left) = add_tags(&mut tags, "Rust, Audio, Maps").unwrap_err();
        assert_eq!(tags.last().unwrap(), "Rust");
        assert_eq!(left, "Audio, Maps");
    }

    #[test]
    fn add_tag_enforces_the_limits() {
        let mut tags = Vec::new();
        assert!(add_tag(&mut tags, &"x".repeat(PostInput::MAX_TAG + 1)).is_err());
        for i in 0..PostInput::MAX_TAGS {
            add_tag(&mut tags, &format!("tag {i}")).unwrap();
        }
        assert!(add_tag(&mut tags, "one more").is_err());
        assert_eq!(tags.len(), PostInput::MAX_TAGS);
    }
}
