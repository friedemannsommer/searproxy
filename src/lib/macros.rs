#[macro_export]
macro_rules! static_asset_route {
    ($path: literal, $name: path, $mediaType: literal) => {{
        async fn handle_static_asset() -> ::actix_web::HttpResponse {
            let _span = ::tracing::span!(::tracing::Level::TRACE, "static_asset", $path).entered();
            let mut response = actix_web::HttpResponse::build(actix_web::http::StatusCode::OK)
                .body(actix_web::web::Bytes::from_static($name));

            response.headers_mut().insert(
                actix_web::http::header::CONTENT_TYPE,
                actix_web::http::header::HeaderValue::from_static($mediaType),
            );

            response
        }

        ::actix_web::web::resource($path).route(::actix_web::web::get().to(handle_static_asset))
    }};
}
