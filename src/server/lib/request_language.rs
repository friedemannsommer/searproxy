static DEFAULT_LANGUAGE: actix_web::http::header::HeaderValue =
    actix_web::http::header::HeaderValue::from_static("en");

pub fn accepted_languages(http_request: &actix_web::HttpRequest) -> &'_ str {
    match http_request
        .headers()
        .get(actix_web::http::header::ACCEPT_LANGUAGE)
        .unwrap_or(&DEFAULT_LANGUAGE)
        .to_str()
    {
        Ok(languages) => languages,
        Err(_) => "en",
    }
}
