//! # Subsistema de Cookies HTTP (RFC 6265bis + CHIPS)

pub mod entry;
pub mod jar;

pub use entry::{Cookie, SameSite};
pub use jar::CookieJar;
