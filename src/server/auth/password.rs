//! Password hashing: Argon2id with the crate's defaults (19 MiB, 2 passes,
//! 1 lane), OWASP's recommended minimum. Hashes are PHC strings that carry
//! their own salt and parameters, so the cost can be raised later without
//! invalidating existing ones.

use std::{sync::LazyLock, thread};

use argon2::{
    Argon2,
    password_hash::{PasswordHasher, PasswordVerifier, phc::PasswordHash},
};
use tokio::sync::Semaphore;

use crate::server::error::ApiError;

/// Each hash holds ~19 MiB and a core for tens of milliseconds; capping how
/// many run at once keeps a burst of sign-ins from exhausting memory or the
/// blocking thread pool. Extra requests queue.
static HASHING: LazyLock<Semaphore> = LazyLock::new(|| {
    Semaphore::new(thread::available_parallelism().map_or(2, |n| n.get()))
});

/// Checked against when an email has no password, so an unknown email takes
/// as long as a wrong password and timing doesn't reveal who has an account.
static DUMMY_HASH: LazyLock<String> =
    LazyLock::new(|| hash_now("not anyone's password").expect("hashing a constant"));

/// Builds the dummy hash up front, so the first unknown-email sign-in isn't
/// the slow one.
pub fn warm_up() {
    LazyLock::force(&DUMMY_HASH);
}

fn hash_now(password: &str) -> Result<String, argon2::password_hash::Error> {
    Ok(Argon2::default().hash_password(password.as_bytes())?.to_string())
}

fn verify_now(password: &str, hash: &str) -> bool {
    match PasswordHash::new(hash) {
        // The parameters come from the stored hash, not `Argon2::default()`.
        Ok(parsed) => Argon2::default().verify_password(password.as_bytes(), &parsed).is_ok(),
        Err(err) => {
            tracing::error!("unparseable password hash in the database: {err}");
            false
        }
    }
}

/// Runs Argon2 off the async threads, a few at a time.
async fn run<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> Result<T, ApiError> {
    let _permit = HASHING.acquire().await.expect("HASHING is never closed");
    tokio::task::spawn_blocking(work).await.map_err(|err| {
        tracing::error!("password hashing task failed: {err}");
        ApiError::internal()
    })
}

/// A PHC string for storing, with a fresh random salt.
pub async fn hash(password: String) -> Result<String, ApiError> {
    run(move || hash_now(&password)).await?.map_err(|err| {
        tracing::error!("password hashing failed: {err}");
        ApiError::internal()
    })
}

/// Whether `password` matches `hash`. Pass `None` when there's no stored
/// hash; it still does the work of a check, then answers no.
pub async fn verify(password: String, hash: Option<String>) -> Result<bool, ApiError> {
    run(move || match hash {
        Some(hash) => verify_now(&password, &hash),
        None => {
            verify_now(&password, &DUMMY_HASH);
            false
        }
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let hash = hash_now("correct horse battery staple").unwrap();
        assert!(hash.starts_with("$argon2id$v=19$m=19456,t=2,p=1$"), "{hash}");
        assert!(verify_now("correct horse battery staple", &hash));
        assert!(!verify_now("correct horse battery stapler", &hash));
    }

    #[test]
    fn salted() {
        assert_ne!(hash_now("same").unwrap(), hash_now("same").unwrap());
    }

    #[test]
    fn garbage_hash_never_matches() {
        assert!(!verify_now("", "not a phc string"));
    }
}
