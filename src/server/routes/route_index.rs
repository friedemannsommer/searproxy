use crate::server::lib::ErrorDetail;

static DEFAULT_LANGUAGE: actix_web::http::header::HeaderValue =
    actix_web::http::header::HeaderValue::from_static("en");

pub async fn handle_request(
    query: actix_web::web::Query<crate::model::IndexHttpArgs>,
    http_request: actix_web::web::HttpRequest,
) -> actix_web::HttpResponse {
    let mut response = actix_web::HttpResponse::new(actix_web::http::StatusCode::OK);

    response.headers_mut().insert(
        actix_web::http::header::CACHE_CONTROL,
        crate::lib::HEADER_VALUE_NO_CACHE.clone(),
    );

    match query.url.as_deref() {
        None => render_index(response),
        Some(url) => {
            if let Some(hash) = query.hash.as_deref() {
                fetch_url(
                    response,
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
            } else {
                render_index(response)
            }
        }
    }
}

fn render_index(response: actix_web::HttpResponse) -> actix_web::HttpResponse {
    match crate::templates::render_template(crate::templates::Template::Index, None) {
        Ok(html) => {
            let mut response_body = response.set_body(actix_web::body::BoxBody::new(html));

            response_body.headers_mut().insert(
                actix_web::http::header::CONTENT_TYPE,
                actix_web::http::header::HeaderValue::from_static("text/html"),
            );

            response_body
        }
        Err(err) => {
            log::error!("{:?}", err);
            crate::server::lib::get_error_response(ErrorDetail::Template(err))
        }
    }
}

async fn fetch_url(
    mut response: actix_web::HttpResponse,
    url: &str,
    hash: &str,
    languages: &str,
) -> actix_web::HttpResponse {
    match crate::lib::fetch_validate_url(reqwest::Method::GET, url, hash, languages).await {
        Ok(client_res) => {
            response = response.set_body(match client_res.body {
                crate::lib::BodyType::Complete(bytes) => actix_web::body::BoxBody::new(bytes),
                crate::lib::BodyType::Stream(stream) => {
                    actix_web::body::BoxBody::new(actix_web::body::BodyStream::new(stream))
                }
            });

            let headers = response.headers_mut();

            if let Some(value) = client_res.content_disposition {
                headers.insert(actix_web::http::header::CONTENT_DISPOSITION, value);
            }

            if let Ok(value) =
                actix_web::http::header::HeaderValue::from_str(client_res.content_type.as_ref())
            {
                headers.insert(actix_web::http::header::CONTENT_TYPE, value);
            }

            response
        }
        Err(err) => {
            log::error!("{:?}", err);
            crate::server::lib::get_error_response(ErrorDetail::Client(err))
        }
    }
}
