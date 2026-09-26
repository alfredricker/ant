use chrono::{DateTime, Utc};
use dioxus::prelude::*;

use crate::models::post::Post;

#[component]
pub fn PostCard(post: Post) -> Element {
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
