use actix_web::http::header::HeaderValue;

use crate::{
    utilities::{fetch_validate_url, ClientRedirect, ClientResponse, FetchResult, FormRequest},
    server::lib::get_content_security_policy,
};

pub async fn fetch_url(
    response: actix_web::HttpResponse,
    url: &str,
    hash: &str,
    languages: &str,
    request_body: Option<FormRequest>,
) -> actix_web::HttpResponse {
    match fetch_validate_url(url, hash, languages, request_body).await {
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
    mut response: actix_web::HttpResponse,
    client_res: ClientResponse,
) -> actix_web::HttpResponse {
    response = response.set_body(match client_res.body {
        crate::utilities::BodyType::Complete(bytes) => actix_web::body::BoxBody::new(bytes),
        crate::utilities::BodyType::Stream(stream) => {
            actix_web::body::BoxBody::new(actix_web::body::BodyStream::new(stream))
        }
    });

    let headers = response.headers_mut();

    if let Some(value) = client_res.content_disposition {
        headers.insert(actix_web::http::header::CONTENT_DISPOSITION, value);
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
    mut response: actix_web::HttpResponse,
    client_redirect: ClientRedirect,
) -> actix_web::HttpResponse {
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
    response = response.set_body(actix_web::body::BoxBody::new(
        crate::templates::render_template_string(crate::templates::Template::Redirect(
            client_redirect,
        )),
    ));

    response
}
