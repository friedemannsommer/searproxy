use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::Error;
use tracing::Span;

pub struct RootSpan {}

impl tracing_actix_web::RootSpanBuilder for RootSpan {
    fn on_request_start(request: &ServiceRequest) -> Span {
        tracing::span!(
            tracing::Level::TRACE,
            "HttpRequest",
            http.method = request.method().as_str(),
            http.uri = request
                .uri()
                .path_and_query()
                .map(|p| p.as_str())
                .unwrap_or("/"),
            http.status_code = tracing::field::Empty
        )
    }

    fn on_request_end<B>(span: Span, outcome: &Result<ServiceResponse<B>, Error>) {
        match outcome {
            Ok(response) => {
                span.record("http.status_code", &response.status().as_u16());
            }
            Err(_) => {
                span.record("http.status_code", &0);
            }
        }
    }
}
