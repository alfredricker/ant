use dioxus::prelude::*;
use uuid::Uuid;

/// For users without a photo; each user always gets the same one.
const INSECTS: [Asset; 4] = [
    asset!("/assets/avatars/ant.svg"),
    asset!("/assets/avatars/ladybug.svg"),
    asset!("/assets/avatars/bee.svg"),
    asset!("/assets/avatars/beetle.svg"),
];

fn insect(user_id: Uuid) -> Asset {
    INSECTS[(user_id.as_u128() % INSECTS.len() as u128) as usize]
}

/// A round user photo: `url` (their Google photo or an upload), or their
/// insect when there's none or it fails to load.
#[component]
pub fn Avatar(user_id: Uuid, url: Option<String>, size: u32) -> Element {
    let mut failed = use_signal(|| false);
    // A new URL (say, just uploaded) deserves a fresh try.
    use_effect(use_reactive!(|url| {
        let _ = url;
        failed.set(false);
    }));

    let src = match &url {
        Some(url) if !failed() => url.clone(),
        _ => insect(user_id).to_string(),
    };

    rsx! {
        img {
            class: "avatar-img",
            src,
            alt: "",
            width: "{size}",
            height: "{size}",
            // Google refuses some photo requests that carry a referrer.
            referrerpolicy: "no-referrer",
            onerror: move |_| failed.set(true),
        }
    }
}
