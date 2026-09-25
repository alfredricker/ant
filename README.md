# Ant
Ant is an application where users find other users to collaborate on projects.
The default is non-monetary projects, such as passion projects, school projects, artistic projects, etc. But the app should support listings with financial backing.

# Start
To start the dev application run the `./build.sh all` script and visit http://localhost:8035

## Database
I'll start with a Postgres database with the pgvector extension to embed posts as vectors for similarity search while maintaining the relational data necessary for user activity.

## Web Code
I will be using Rust for the full stack.
### Routing
For HTTP routing and request handling, I will be using axum https://github.com/tokio-rs/axum
For the frontend I am using Dioxus fullstack https://dioxuslabs.com: server-rendered pages hydrated by wasm, with server functions in place of a hand-written client API.