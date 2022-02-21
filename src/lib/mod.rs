pub use client::{fetch_validate_url, BodyType, ClientError, PostRequest};
#[cfg(test)]
pub use shared::test_setup_hmac;
pub use shared::{
    GLOBAL_CONFIG, HEADER_VALUE_CONTENT_HTML, HEADER_VALUE_NO_CACHE, HMAC, REQUEST_CLIENT,
};

mod client;
pub mod macros;
mod rewrite_css;
mod rewrite_html;
mod rewrite_url;
mod shared;
