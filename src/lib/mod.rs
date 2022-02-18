pub use client::{BodyType, ClientError, fetch_validate_url};
pub use shared::{
    GLOBAL_CONFIG, HEADER_VALUE_CONTENT_HTML, HEADER_VALUE_NO_CACHE, HMAC, REQUEST_CLIENT,
};
#[cfg(test)]
pub use shared::test_setup_hmac;

mod client;
pub mod macros;
mod rewrite_css;
mod rewrite_html;
mod rewrite_url;
mod shared;

