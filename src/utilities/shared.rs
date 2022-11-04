pub static HMAC: once_cell::sync::OnceCell<hmac::Hmac<sha2::Sha256>> =
    once_cell::sync::OnceCell::new();
pub static REQUEST_CLIENT: once_cell::sync::OnceCell<reqwest::Client> =
    once_cell::sync::OnceCell::new();
pub static GLOBAL_CONFIG: once_cell::sync::OnceCell<crate::model::Config<'_, '_>> =
    once_cell::sync::OnceCell::new();
pub static HEADER_VALUE_NO_CACHE: actix_web::http::header::HeaderValue =
    actix_web::http::header::HeaderValue::from_static("no-cache");
pub static HEADER_VALUE_CONTENT_HTML: once_cell::sync::Lazy<actix_web::http::header::HeaderValue> =
    once_cell::sync::Lazy::new(|| {
        actix_web::http::header::HeaderValue::from_static(mime::TEXT_HTML_UTF_8.as_ref())
    });

#[cfg(test)]
pub fn test_setup_hmac() {
    use hmac::digest::KeyInit;

    if HMAC
        .set(hmac::Hmac::new_from_slice(b"example").unwrap())
        .is_err()
    {
        // silently ignore this, since it only `Err`s on successive calls
    }
}
