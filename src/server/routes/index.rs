use crate::{
    lib::{ClientError, FormRequest},
    server::lib::{accepted_languages, fetch_url},
};
use actix_web::HttpMessage;

#[actix_web::get("/")]
pub async fn handle_get_request(
    query: actix_web::web::Query<crate::model::IndexHttpArgs>,
    http_request: actix_web::web::HttpRequest,
) -> actix_web::HttpResponse {
    let response = get_base_response();

    match query.url.as_deref() {
        None => render_index(response),
        Some(url) => {
            if let Some(hash) = query.hash.as_deref() {
                fetch_url(response, url, hash, accepted_languages(&http_request), None).await
            } else {
                render_index(response)
            }
        }
    }
}

#[actix_web::post("/")]
pub async fn handle_post_request(
    query: actix_web::web::Query<crate::model::IndexHttpArgs>,
    http_request: actix_web::web::HttpRequest,
    mut body: actix_web::web::Form<std::collections::HashMap<String, String>>,
) -> actix_web::HttpResponse {
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
                accepted_languages(&http_request),
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

fn get_base_response() -> actix_web::HttpResponse {
    let mut response = actix_web::HttpResponse::new(actix_web::http::StatusCode::OK);

    response.headers_mut().insert(
        actix_web::http::header::CACHE_CONTROL,
        crate::lib::HEADER_VALUE_NO_CACHE.clone(),
    );

    response
}

fn render_index(response: actix_web::HttpResponse) -> actix_web::HttpResponse {
    let mut response_body = response.set_body(actix_web::body::BoxBody::new(
        crate::templates::render_template_string(crate::templates::Template::Index),
    ));

    response_body.headers_mut().insert(
        actix_web::http::header::CONTENT_TYPE,
        actix_web::http::header::HeaderValue::from_static("text/html"),
    );

    response_body
}
