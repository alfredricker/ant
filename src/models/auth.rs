//! Email + password sign-in rules, shared so the sign-in page can show the
//! same limits the server enforces.

/// NIST 800-63B: at least 8, no composition rules, and room for passphrases.
pub const PASSWORD_MIN_CHARS: usize = 8;
/// Bounds the work a single sign-in attempt can ask Argon2 for.
pub const PASSWORD_MAX_CHARS: usize = 128;
/// The longest address SMTP allows.
pub const EMAIL_MAX_CHARS: usize = 254;

/// Why a sign-in or sign-up form was turned away. The server redirects to
/// `/signin?error=<slug>`, so only these fixed messages ever reach the page,
/// never text from the query string.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthError {
    /// Wrong email or password. Deliberately doesn't say which.
    Invalid,
    EmailTaken,
    BadEmail,
    BadPassword,
}

impl AuthError {
    pub fn slug(self) -> &'static str {
        match self {
            Self::Invalid => "invalid",
            Self::EmailTaken => "taken",
            Self::BadEmail => "email",
            Self::BadPassword => "password",
        }
    }

    pub fn from_slug(slug: &str) -> Option<Self> {
        [Self::Invalid, Self::EmailTaken, Self::BadEmail, Self::BadPassword]
            .into_iter()
            .find(|err| err.slug() == slug)
    }

    pub fn message(self) -> &'static str {
        match self {
            Self::Invalid => "Wrong email or password. If you signed up with Google, continue with Google.",
            Self::EmailTaken => "There's already an account with that email. Sign in instead.",
            Self::BadEmail => "That doesn't look like an email address.",
            Self::BadPassword => "Passwords need 8 to 128 characters.",
        }
    }
}

/// A plausible address: something@something, no spaces. Whether it's real is
/// for a verification email to find out.
pub fn check_email(email: &str) -> Result<(), AuthError> {
    let plausible = email.chars().count() <= EMAIL_MAX_CHARS
        && !email.chars().any(char::is_whitespace)
        && email
            .rsplit_once('@')
            .is_some_and(|(local, domain)| !local.is_empty() && !domain.is_empty());
    if plausible { Ok(()) } else { Err(AuthError::BadEmail) }
}

pub fn check_password(password: &str) -> Result<(), AuthError> {
    let chars = password.chars().count();
    if (PASSWORD_MIN_CHARS..=PASSWORD_MAX_CHARS).contains(&chars) {
        Ok(())
    } else {
        Err(AuthError::BadPassword)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs_round_trip() {
        for err in [AuthError::Invalid, AuthError::EmailTaken, AuthError::BadEmail, AuthError::BadPassword] {
            assert_eq!(AuthError::from_slug(err.slug()), Some(err));
        }
        assert_eq!(AuthError::from_slug("<script>"), None);
    }

    #[test]
    fn emails() {
        assert!(check_email("a@b.c").is_ok());
        assert!(check_email("a@b").is_ok());
        for bad in ["", "ab", "@b.c", "a@", "a b@c.d", &format!("{}@b.c", "a".repeat(254))] {
            assert_eq!(check_email(bad), Err(AuthError::BadEmail), "{bad:?}");
        }
    }

    #[test]
    fn passwords() {
        assert!(check_password("12345678").is_ok());
        // Counted in characters, not bytes.
        assert!(check_password(&"é".repeat(128)).is_ok());
        assert!(check_password("1234567").is_err());
        assert!(check_password(&"a".repeat(129)).is_err());
    }
}
