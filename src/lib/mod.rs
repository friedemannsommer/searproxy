mod client;
pub mod macros;
mod rewrite_css;
mod rewrite_html;
mod rewrite_url;
mod shared;

pub use client::{fetch_validate_url, ClientError};
pub use shared::{
    GLOBAL_CONFIG, HEADER_VALUE_CONTENT_HTML, HEADER_VALUE_CONTENT_TEXT, HEADER_VALUE_NO_CACHE,
    HMAC, MINIFY_CONFIG, REQUEST_CLIENT,
};

#[cfg(test)]
pub use shared::test_setup_hmac;
