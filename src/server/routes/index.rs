use crate::{
    server::lib::fetch_url,
    utilities::{ClientError, ClientResponseBody, FormRequest},
};

#[actix_web::get("/")]
pub async fn handle_get_request(
    query: actix_web::web::Query<crate::model::IndexHttpArgs>,
    http_request: actix_web::HttpRequest,
) -> actix_web::HttpResponse<ClientResponseBody> {
    let response = get_base_response();

    match query.url.as_deref() {
        None => render_index(response),
        Some(url) => {
            if let Some(hash) = query.hash.as_deref() {
                fetch_url(response, url, hash, http_request.headers(), None).await
            } else {
                render_index(response)
            }
        }
    }
}

#[actix_web::post("/")]
pub async fn handle_post_request(
    query: actix_web::web::Query<crate::model::IndexHttpArgs>,
    http_request: actix_web::HttpRequest,
    mut body: actix_web::web::Form<std::collections::HashMap<String, String>>,
) -> actix_web::HttpResponse<ClientResponseBody> {
    let response = get_base_response();

    if let Some(url) = query.url.as_deref() {
        if let Some(hash) = query.hash.as_deref() {
            let origin_method = body
                .remove("_searproxy_origin_method")
                .map(|m| m.to_ascii_uppercase());
            let method = origin_method.as_deref().unwrap_or("GET");

            return fetch_url(
                response,
                url,
                hash,
                http_request.headers(),
                Some(FormRequest {
                    body: body.into_inner(),
                    method: if method.trim() == "GET" {
                        reqwest::Method::GET
                    } else {
                        reqwest::Method::POST
                    },
                }),
            )
            .await;
        }
    }

    crate::server::lib::get_error_response(ClientError::BadRequest)
}

fn get_base_response() -> actix_web::HttpResponse<ClientResponseBody> {
    let mut response = actix_web::HttpResponse::<ClientResponseBody>::with_body(
        actix_web::http::StatusCode::OK,
        actix_web::body::EitherBody::Right {
            body: bytes::Bytes::default(),
        },
    );

    response.headers_mut().insert(
        actix_web::http::header::CACHE_CONTROL,
        crate::utilities::HEADER_VALUE_NO_CACHE.clone(),
    );

    response
}

fn render_index(
    response: actix_web::HttpResponse<ClientResponseBody>,
) -> actix_web::HttpResponse<ClientResponseBody> {
    let mut response_body = response.set_body(actix_web::body::EitherBody::Right {
        body: bytes::Bytes::from(crate::templates::render_template_string(
            crate::templates::Template::Index,
        )),
    });

    response_body.headers_mut().insert(
        actix_web::http::header::CONTENT_TYPE,
        actix_web::http::header::HeaderValue::from_static(mime::TEXT_HTML_UTF_8.as_ref()),
    );

    response_body
}
