pub use client::{
    BodyType, ClientError, ClientRedirect, ClientResponse, ClientResponseBody, FetchResult,
    FormRequest, fetch_validate_url,
};
#[cfg(test)]
pub use shared::test_setup_hmac;
pub use shared::{
    BASE64_ENGINE, GLOBAL_CONFIG, HEADER_VALUE_CONTENT_HTML, HEADER_VALUE_NO_CACHE, HMAC,
    HmacInstance, REQUEST_CLIENT,
};

mod client;
pub mod macros;
mod rewrite_css;
mod rewrite_html;
mod rewrite_url;
mod shared;
