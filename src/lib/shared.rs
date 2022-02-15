pub static HMAC: once_cell::sync::OnceCell<hmac_sha256::HMAC> = once_cell::sync::OnceCell::new();
pub static REQUEST_CLIENT: once_cell::sync::OnceCell<reqwest::Client> =
    once_cell::sync::OnceCell::new();
pub static GLOBAL_CONFIG: once_cell::sync::OnceCell<crate::model::Config> =
    once_cell::sync::OnceCell::new();
pub static MINIFY_CONFIG: once_cell::sync::Lazy<minify_html::Cfg> =
    once_cell::sync::Lazy::new(minify_html::Cfg::spec_compliant);
pub static HEADER_VALUE_NO_CACHE: actix_web::http::header::HeaderValue =
    actix_web::http::header::HeaderValue::from_static("no-cache");
pub static HEADER_VALUE_CONTENT_HTML: once_cell::sync::Lazy<actix_web::http::header::HeaderValue> =
    once_cell::sync::Lazy::new(|| {
        actix_web::http::header::HeaderValue::from_static(mime::TEXT_HTML_UTF_8.as_ref())
    });
pub static HEADER_VALUE_CONTENT_TEXT: once_cell::sync::Lazy<actix_web::http::header::HeaderValue> =
    once_cell::sync::Lazy::new(|| {
        actix_web::http::header::HeaderValue::from_static(mime::TEXT_PLAIN_UTF_8.as_ref())
    });
