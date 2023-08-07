use std::borrow::Cow;

use crate::utilities::{ClientError, ClientResponseBody};

#[derive(serde::Serialize)]
pub struct ErrorMessage<'name, 'description> {
    pub name: Cow<'name, str>,
    pub description: Cow<'description, str>,
}

pub fn get_error_response(
    error_detail: ClientError,
) -> actix_web::HttpResponse<ClientResponseBody> {
    let mut response = actix_web::HttpResponse::with_body(
        match error_detail {
            ClientError::InvalidHash => actix_web::http::StatusCode::UNAUTHORIZED,
            ClientError::Hex(_)
            | ClientError::BadRequest
            | ClientError::IpRangeDenied(_)
            | ClientError::ResolveHostname(_) => actix_web::http::StatusCode::BAD_REQUEST,
            _ => actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
        },
        actix_web::body::EitherBody::Right {
            body: bytes::Bytes::from(crate::templates::render_template_string(
                crate::templates::Template::Error(get_error_message(error_detail)),
            )),
        },
    );
    let headers = response.headers_mut();

    headers.insert(
        actix_web::http::header::CACHE_CONTROL,
        crate::utilities::HEADER_VALUE_NO_CACHE.clone(),
    );

    headers.insert(
        actix_web::http::header::CONTENT_TYPE,
        crate::utilities::HEADER_VALUE_CONTENT_HTML.clone(),
    );

    response
}

fn get_error_message(error_detail: ClientError) -> Option<ErrorMessage<'static, 'static>> {
    match error_detail {
        ClientError::InvalidHash => Some(ErrorMessage {
            name: Cow::Borrowed("Invalid hash"),
            description: Cow::Borrowed("The given URL and hash combination doesn't match."),
        }),
        ClientError::UnexpectedStatusCode(status_code) => Some(ErrorMessage {
            name: Cow::Borrowed("Unexpected status code"),
            description: Cow::Owned(format!("Origin returned status code: '{status_code}'")),
        }),
        ClientError::MimeParse(_) => Some(ErrorMessage {
            name: Cow::Borrowed("Invalid media type"),
            description: Cow::Borrowed("Origin returned invalid media type"),
        }),
        ClientError::Hex(_) => Some(ErrorMessage {
            name: Cow::Borrowed("Invalid hash"),
            description: Cow::Borrowed("The given hash must be valid hexadecimal."),
        }),
        ClientError::BadRequest => Some(ErrorMessage {
            name: Cow::Borrowed("Bad request"),
            description: Cow::Owned(error_detail.to_string()),
        }),
        ClientError::RedirectWithoutLocation => Some(ErrorMessage {
            name: Cow::Borrowed("Invalid redirect"),
            description: Cow::Owned(error_detail.to_string()),
        }),
        ClientError::IpRangeDenied(host) => Some(ErrorMessage {
            name: Cow::Borrowed("Request denied"),
            description: Cow::Owned(format!(
                "The requested host \"{host}\" is not permitted by the service provider."
            )),
        }),
        ClientError::ResolveHostname(host) => Some(ErrorMessage {
            name: Cow::Borrowed("Unknown host"),
            description: Cow::Owned(format!(
                "The requested host \"{host}\" couldn't be resolved."
            )),
        }),
        _ => None,
    }
}
