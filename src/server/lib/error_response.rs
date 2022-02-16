use std::borrow::Cow;
use tracing::error;

use crate::lib::ClientError;

#[derive(thiserror::Error, Debug)]
pub enum ErrorDetail {
    #[error(transparent)]
    Client(#[from] ClientError),
    #[error("Template render failed")]
    Template(#[from] tera::Error),
}

#[derive(serde::Serialize)]
pub struct ErrorMessage<'name, 'description> {
    name: Cow<'name, str>,
    description: Cow<'description, str>,
}

pub fn get_error_response(error_detail: ErrorDetail) -> actix_web::HttpResponse {
    let mut response = actix_web::HttpResponse::new(match error_detail {
        ErrorDetail::Client(ClientError::InvalidHash) => actix_web::http::StatusCode::UNAUTHORIZED,
        _ => actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
    });
    let mut context = tera::Context::new();

    if let Some(error_message) = get_error_message(error_detail) {
        context.insert("error_detail", &error_message);
    }

    response.headers_mut().insert(
        actix_web::http::header::CACHE_CONTROL,
        crate::lib::HEADER_VALUE_NO_CACHE.clone(),
    );

    match crate::templates::render_minified(crate::templates::Template::Error, Some(context)) {
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

fn get_error_message(error_detail: ErrorDetail) -> Option<ErrorMessage<'static, 'static>> {
    match error_detail {
        ErrorDetail::Client(ClientError::InvalidHash) => Some(ErrorMessage {
            name: Cow::Borrowed("Invalid hash"),
            description: Cow::Borrowed("The given URL and hash combination doesn't match."),
        }),
        ErrorDetail::Client(ClientError::UnexpectedStatusCode(status_code)) => Some(ErrorMessage {
            name: Cow::Borrowed("Unexpected status code"),
            description: Cow::Owned(format!("Origin returned status code: '{}'", status_code)),
        }),
        _ => None,
    }
}
