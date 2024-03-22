use actix_web::http::header::HeaderValue;

use crate::{
    server::lib::get_content_security_policy,
    utilities::{
        fetch_validate_url, ClientRedirect, ClientResponse, ClientResponseBody, FetchResult,
        FormRequest,
    },
};

pub async fn fetch_url(
    response: actix_web::HttpResponse<ClientResponseBody>,
    url: &str,
    hash: &str,
    headers: &actix_web::http::header::HeaderMap,
    request_body: Option<FormRequest>,
) -> actix_web::HttpResponse<ClientResponseBody> {
    match fetch_validate_url(url, hash, headers, request_body).await {
        Ok(fetch_result) => match fetch_result {
            FetchResult::Response(client_res) => handle_client_response(response, client_res),
            FetchResult::Redirect(client_redirect) => {
                handle_client_redirect(response, client_redirect)
            }
        },
        Err(err) => {
            log::error!("fetch_validate_url: {:?}", err);
            crate::server::lib::get_error_response(err)
        }
    }
}

fn handle_client_response(
    mut response: actix_web::HttpResponse<ClientResponseBody>,
    client_res: ClientResponse,
) -> actix_web::HttpResponse<ClientResponseBody> {
    response = response.set_body(match client_res.body {
        crate::utilities::BodyType::Complete(body) => actix_web::body::EitherBody::Right { body },
        crate::utilities::BodyType::Stream(stream) => actix_web::body::EitherBody::Left {
            body: if let Some(body_size) = client_res.content_length {
                actix_web::body::EitherBody::Left {
                    body: actix_web::body::SizedStream::new(body_size, stream),
                }
            } else {
                actix_web::body::EitherBody::Right {
                    body: actix_web::body::BodyStream::new(stream),
                }
            },
        },
    });

    let headers = response.headers_mut();

    if let Some(value) = client_res.content_disposition {
        if let Ok(header_value) = HeaderValue::from_bytes(value.as_ref()) {
            headers.insert(actix_web::http::header::CONTENT_DISPOSITION, header_value);
        }
    }

    if let Some(style_hashes) = client_res.style_hashes {
        headers.insert(
            actix_web::http::header::CONTENT_SECURITY_POLICY,
            get_content_security_policy(Some(style_hashes)),
        );
    }

    if let Ok(value) = HeaderValue::from_str(client_res.content_type.as_ref()) {
        headers.insert(actix_web::http::header::CONTENT_TYPE, value);
    }

    response
}

fn handle_client_redirect(
    mut response: actix_web::HttpResponse<ClientResponseBody>,
    client_redirect: ClientRedirect,
) -> actix_web::HttpResponse<ClientResponseBody> {
    let header_value_res = HeaderValue::try_from(client_redirect.internal_url.as_str());

    if let Ok(header_value) = header_value_res {
        if let Some(config) = crate::utilities::GLOBAL_CONFIG.get() {
            if config.follow_redirects {
                response
                    .headers_mut()
                    .insert(actix_web::http::header::LOCATION, header_value);
                *response.status_mut() = actix_web::http::StatusCode::TEMPORARY_REDIRECT;
                return response;
            }
        }
    }

    response.headers_mut().insert(
        actix_web::http::header::CONTENT_TYPE,
        HeaderValue::from_static(mime::TEXT_HTML_UTF_8.as_ref()),
    );

    response = response.set_body(actix_web::body::EitherBody::Right {
        body: bytes::Bytes::from(crate::templates::render_template_string(
            crate::templates::Template::Redirect(client_redirect),
        )),
    });

    response
}
