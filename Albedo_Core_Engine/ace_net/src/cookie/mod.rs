//! # Subsistema de Cookies HTTP (RFC 6265bis + CHIPS)

pub mod entry;
pub mod jar;

pub use entry::{is_public_suffix, is_valid_cookie_domain, Cookie, SameSite};
pub use jar::{CookieJar, MAX_COOKIES_PER_DOMAIN};
