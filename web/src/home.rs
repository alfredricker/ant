//! The homepage: what ant is, what's on it, and whether the backend is alive.

use ant_common::models::user::User;
use gloo_net::http::Request;
use yew::prelude::*;

use crate::Health;

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

#[component]
pub fn Home() -> Html {
    html! {
        <>
            <Nav />
            <main>
                <Hero />
                <HowItWorks />
                <Listings />
            </main>
            <Footer />
        </>
    }
}

#[component]
fn Nav() -> Html {
    // None while asking the API, then Some(None) when signed out.
    let user = use_state(|| None::<Option<User>>);

    {
        let user = user.clone();
        use_effect_with((), move |_| {
            wasm_bindgen_futures::spawn_local(async move {
                let me = match Request::get("/api/auth/me").send().await {
                    Ok(response) if response.ok() => response.json::<User>().await.ok(),
                    _ => None,
                };
                user.set(Some(me));
            });
            || ()
        });
    }

    let sign_out = {
        let user = user.clone();
        Callback::from(move |_: MouseEvent| {
            let user = user.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let _ = Request::post("/api/auth/logout").send().await;
                user.set(Some(None));
            });
        })
    };

    let account = match &*user {
        None => html! {},
        Some(Some(user)) => html! {
            <>
                <span class="nav-user">{ user.display_name() }</span>
                <button class="button button-ghost" onclick={sign_out}>{ "Sign out" }</button>
            </>
        },
        // A plain link, not fetch: the browser has to follow the redirects to Google and back.
        Some(None) => html! {
            <a class="button button-ghost" href="/api/auth/google/login">{ "Sign in with Google" }</a>
        },
    };

    html! {
        <header class="nav">
            <a class="brand" href="/">
                <span class="brand-mark">{ "🐜" }</span>
                <span class="brand-name">{ "ant" }</span>
            </a>
            <nav class="nav-links">
                <a href="/browse">{ "Browse" }</a>
                <a href="/post">{ "Post a project" }</a>
                { account }
            </nav>
        </header>
    }
}

#[component]
fn Hero() -> Html {
    html! {
        <section class="hero">
            <p class="eyebrow">{ "Collaborators, not clients" }</p>
            <h1>{ "Find someone to build the thing with." }</h1>
            <p class="lede">
                { "ant is for passion projects, school projects, art, and the odd thing that grew a budget. \
                   Post what you're making, say what you're missing, and get matched with people whose work \
                   actually lines up with yours." }
            </p>
            <div class="hero-actions">
                <a class="button button-primary" href="/post">{ "Post a project" }</a>
                <a class="button button-ghost" href="/browse">{ "Browse projects" }</a>
            </div>
            <BackendStatus />
        </section>
    }
}

#[component]
fn HowItWorks() -> Html {
    let steps = [
        ("Describe it", "A paragraph is enough. What you're building, how far along it is, and what you want a hand with."),
        ("Get matched", "Posts are embedded as vectors, so you surface next to work that's genuinely adjacent — not just sharing a tag."),
        ("Start small", "Trade a message, scope one weekend's worth of work, and see whether you like working together."),
    ];

    html! {
        <section class="how">
            <h2>{ "How it works" }</h2>
            <ol class="steps">
                { for steps.iter().enumerate().map(|(i, (title, body))| html! {
                    <li class="step">
                        <span class="step-number">{ i + 1 }</span>
                        <h3>{ *title }</h3>
                        <p>{ *body }</p>
                    </li>
                }) }
            </ol>
        </section>
    }
}

#[component]
fn Listings() -> Html {
    html! {
        <section class="listings">
            <div class="listings-head">
                <h2>{ "Open right now" }</h2>
                <a href="/browse">{ "See all →" }</a>
            </div>
            <div class="cards">
                { for LISTINGS.iter().map(|listing| html! {
                    <article class="card">
                        <div class="card-top">
                            <span class="kind">{ listing.kind }</span>
                            if listing.funded {
                                <span class="funded">{ "Funded" }</span>
                            }
                        </div>
                        <h3>{ listing.title }</h3>
                        <p>{ listing.blurb }</p>
                        <ul class="tags">
                            { for listing.looking_for.iter().map(|tag| html! {
                                <li class="tag">{ *tag }</li>
                            }) }
                        </ul>
                    </article>
                }) }
            </div>
        </section>
    }
}

/// Small dev affordance: proves the wasm bundle, the axum API and postgres are
/// all actually talking to each other.
#[component]
fn BackendStatus() -> Html {
    let health = use_state(|| None::<Result<Health, String>>);

    {
        let health = health.clone();
        use_effect_with((), move |_| {
            wasm_bindgen_futures::spawn_local(async move {
                let result = match Request::get("/api/health").send().await {
                    Ok(response) => response
                        .json::<Health>()
                        .await
                        .map_err(|err| format!("bad response: {err}")),
                    Err(err) => Err(format!("unreachable: {err}")),
                };
                health.set(Some(result));
            });
            || ()
        });
    }

    let (class, label) = match &*health {
        None => ("status status-pending", "checking backend…".to_string()),
        Some(Ok(h)) if h.status == "ok" => (
            "status status-ok",
            match &h.pgvector {
                Some(version) => format!("api ok · postgres {} · pgvector {version}", h.database),
                None => format!("api ok · postgres {} · pgvector missing", h.database),
            },
        ),
        Some(Ok(h)) => ("status status-down", format!("api {} · postgres {}", h.status, h.database)),
        Some(Err(err)) => ("status status-down", format!("api {err}")),
    };

    html! {
        <p class={class}><span class="dot" />{ label }</p>
    }
}

#[component]
fn Footer() -> Html {
    html! {
        <footer class="footer">
            <span>{ "ant — a place to find collaborators" }</span>
            <a href="/api/health">{ "status" }</a>
        </footer>
    }
}
