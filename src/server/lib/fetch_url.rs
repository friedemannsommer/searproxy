use crate::{lib::PostRequest, server::lib::get_content_security_policy};

pub async fn fetch_url(
    mut response: actix_web::HttpResponse,
    url: &str,
    hash: &str,
    languages: &str,
    request_body: Option<PostRequest>,
) -> actix_web::HttpResponse {
    match crate::lib::fetch_validate_url(url, hash, languages, request_body).await {
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

            if let Some(style_hashes) = client_res.style_hashes {
                headers.insert(
                    actix_web::http::header::CONTENT_SECURITY_POLICY,
                    get_content_security_policy(Some(style_hashes)),
                );
            }

            if let Ok(value) =
                actix_web::http::header::HeaderValue::from_str(client_res.content_type.as_ref())
            {
                headers.insert(actix_web::http::header::CONTENT_TYPE, value);
            }

            response
        }
        Err(err) => {
            log::error!("fetch_validate_url: {:?}", err);
            crate::server::lib::get_error_response(err)
        }
    }
}
