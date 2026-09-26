//! The feed's search text and filters live in the URL
//! (`/?q=…&kind=…&funded=1`), so results are rendered on the server, links to
//! a search work, and the search form and filter chips work before the wasm
//! has loaded.

use std::fmt;

use crate::models::post::PostKind;

/// What the feed shows, as read from and written to the query string.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FeedQuery {
    pub text: String,
    pub kind: Option<PostKind>,
    pub funded: bool,
}

impl FeedQuery {
    pub(super) fn with_kind(&self, kind: Option<PostKind>) -> Self {
        Self { kind, ..self.clone() }
    }

    pub(super) fn with_funded(&self, funded: bool) -> Self {
        Self { funded, ..self.clone() }
    }

    pub(super) fn is_filtered(&self) -> bool {
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
