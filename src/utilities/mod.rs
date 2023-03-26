pub use client::{
    fetch_validate_url, BodyType, ClientError, ClientRedirect, ClientResponse, FetchResult,
    FormRequest,
};
#[cfg(test)]
pub use shared::test_setup_hmac;
pub use shared::{
    HmacInstance, BASE64_ENGINE, GLOBAL_CONFIG, HEADER_VALUE_CONTENT_HTML, HEADER_VALUE_NO_CACHE,
    HMAC, REQUEST_CLIENT,
};

mod client;
pub mod macros;
mod rewrite_css;
mod rewrite_html;
mod rewrite_url;
mod shared;
