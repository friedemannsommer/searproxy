use tracing::error;

pub fn get_error_response() -> actix_web::HttpResponse {
    let mut response =
        actix_web::HttpResponse::new(actix_web::http::StatusCode::INTERNAL_SERVER_ERROR);

    response.headers_mut().insert(
        actix_web::http::header::CACHE_CONTROL,
        crate::lib::HEADER_VALUE_NO_CACHE.clone(),
    );

    match crate::templates::render_minified(crate::templates::Template::Error) {
        Ok(html) => {
            let mut response_body = response.set_body(actix_web::body::BoxBody::new(html));

            response_body.headers_mut().insert(
                actix_web::http::header::CONTENT_TYPE,
                crate::lib::HEADER_VALUE_CONTENT_HTML.clone(),
            );

            response_body
        }
        Err(err) => {
            error!("{:?}", err);

            response.headers_mut().insert(
                actix_web::http::header::CONTENT_TYPE,
                crate::lib::HEADER_VALUE_CONTENT_TEXT.clone(),
            );

            response
        }
    }
}
