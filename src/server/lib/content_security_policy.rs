use actix_web::http::header::HeaderValue;

use crate::assets::{HEADER_STYLESHEET_HASH, MAIN_STYLESHEET_HASH};

pub fn get_content_security_policy(style_hashes_opt: Option<Vec<String>>) -> HeaderValue {
    let style_src_hashes = if let Some(mut style_hashes) = style_hashes_opt {
        style_hashes.push(String::from(HEADER_STYLESHEET_HASH));
        style_hashes.join(" ")
    } else {
        String::from(MAIN_STYLESHEET_HASH)
    };

    HeaderValue::from_str(
        format!(
            "default-src 'none'; block-all-mixed-content; img-src data: 'self'; style-src 'self' {}; prefetch-src 'self'; media-src 'self'; frame-src 'self'; font-src 'self'; frame-ancestors 'self'",
            style_src_hashes
        ).as_str()
    ).expect("unexpected non ASCII chars in header value")
}
