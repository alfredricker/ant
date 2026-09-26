# Ant
Ant brings together two or more people who share a big goal, so they can go after it together: a long trip, a startup, a research project, a game, an album, anything worth doing with someone else.

It's a social site, not a SaaS product. There's no landing page: you arrive in a feed of what people want to do, with search and filters. Most posts are non-monetary, but posts can say they have funding behind them.

# Start
To start the dev application run the `./build.sh all` script and visit http://localhost:8035

## Database
I'll start with a Postgres database with the pgvector extension to embed posts as vectors for similarity search while maintaining the relational data necessary for user activity.

## Web Code
I will be using Rust for the full stack.
### Routing
For HTTP routing and request handling, I will be using axum https://github.com/tokio-rs/axum
For the frontend I am using Dioxus fullstack https://dioxuslabs.com: server-rendered pages hydrated by wasm, with server functions in place of a hand-written client API.