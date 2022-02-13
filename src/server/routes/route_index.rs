use tracing::{error, span};

static DEFAULT_LANGUAGE: actix_web::http::header::HeaderValue =
    actix_web::http::header::HeaderValue::from_static("en");

pub async fn handle_request(
    query: actix_web::web::Query<crate::model::IndexHttpArgs>,
    http_request: actix_web::web::HttpRequest,
) -> actix_web::HttpResponse {
    let _index_span = span!(tracing::Level::TRACE, "http_index").entered();
    let mut response = actix_web::HttpResponse::new(actix_web::http::StatusCode::OK);

    response.headers_mut().insert(
        actix_web::http::header::CACHE_CONTROL,
        crate::lib::HEADER_VALUE_NO_CACHE.clone(),
    );

    match query.url.as_deref() {
        None => render_index(response),
        Some(url) => {
            if let Some(hash) = query.hash.as_deref() {
                match crate::lib::fetch_validate_url(
                    reqwest::Method::GET,
                    url,
                    hash,
                    match http_request
                        .headers()
                        .get(actix_web::http::header::ACCEPT_LANGUAGE)
                        .unwrap_or(&DEFAULT_LANGUAGE)
                        .to_str()
                    {
                        Ok(languages) => languages,
                        Err(_) => "en",
                    },
                )
                .await
                {
                    Ok(client_res) => {
                        let mut next_response =
                            response.set_body(actix_web::body::BoxBody::new(client_res.body));
                        let headers = next_response.headers_mut();

                        if let Ok(value) = actix_web::http::header::HeaderValue::from_str(
                            client_res.content_type.as_ref(),
                        ) {
                            headers.insert(actix_web::http::header::CONTENT_TYPE, value);
                        }

                        next_response
                    }
                    Err(err) => {
                        error!("{:?}", err);
                        // todo: this should include at least some information about why it isn't working
                        crate::server::lib::get_error_response()
                    }
                }
            } else {
                render_index(response)
            }
        }
    }
}

fn render_index(response: actix_web::HttpResponse) -> actix_web::HttpResponse {
    let _render_span = span!(tracing::Level::TRACE, "render_index_html").entered();

    match crate::templates::render_minified(crate::templates::Template::Index) {
        Ok(html) => {
            let mut response_body = response.set_body(actix_web::body::BoxBody::new(html));

            response_body.headers_mut().insert(
                actix_web::http::header::CONTENT_TYPE,
                actix_web::http::header::HeaderValue::from_static("text/html"),
            );

            response_body
        }
        Err(err) => {
            error!("{:?}", err);
            crate::server::lib::get_error_response()
        }
    }
}
