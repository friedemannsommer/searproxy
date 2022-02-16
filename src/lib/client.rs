use futures_util::StreamExt;

use crate::lib::{
    rewrite_css::{CssRewrite, RewriteCssError},
    rewrite_html::HtmlRewrite,
};

#[derive(thiserror::Error, Debug)]
pub enum ClientError {
    #[error("HMAC instance uninitialized")]
    HmacInstance,
    #[error("hex decode failed")]
    Hex(#[from] hex::FromHexError),
    #[error("request client is uninitialized")]
    RequestClient,
    #[error("HTTP request failed")]
    Request(#[from] reqwest::Error),
    #[error("HMAC hash is invalid")]
    InvalidHash,
    #[error("HTTP request failed with status code: {0}")]
    UnexpectedStatusCode(u16),
    #[error("String decode failed")]
    StringDecode(#[from] reqwest::header::ToStrError),
    #[error("URL parsing failed")]
    UrlParse(#[from] url::ParseError),
    #[error("MIME parsing failed")]
    MimeParse(#[from] mime::FromStrError),
    #[error("UTF-8 decoding failed")]
    Utf8Decode(#[from] std::str::Utf8Error),
    #[error("HTML rewriting failed")]
    HtmlRewrite(#[from] lol_html::errors::RewritingError),
    #[error("CSS rewriting failed")]
    CssRewrite(#[from] RewriteCssError),
}

pub enum BodyType {
    Complete(bytes::Bytes),
    Stream(
        std::pin::Pin<Box<dyn futures_util::Stream<Item = Result<bytes::Bytes, reqwest::Error>>>>,
    ),
}

pub struct ClientResponse {
    pub body: BodyType,
    pub content_disposition: Option<reqwest::header::HeaderValue>,
    pub content_type: mime::Mime,
}

pub async fn fetch_validate_url(
    method: reqwest::Method,
    url: &str,
    hash: &str,
    acceptable_languages: &str,
) -> Result<ClientResponse, ClientError> {
    let mut hmac = match crate::lib::HMAC.get() {
        Some(instance) => instance.clone(),
        None => return Err(ClientError::HmacInstance),
    };

    let hash_bytes = hex::decode(hash)?;
    let computed_hash = {
        hmac.update(url.as_bytes());
        hmac.finalize()
    };

    if hash_bytes == computed_hash {
        log::debug!("{} '{}'", method, url);
        return fetch_transform_url(method, url, acceptable_languages).await;
    }

    if log::log_enabled!(log::Level::Info) {
        log::info!(
            "rejecting request for: '{}' (invalid hash: {} != {})",
            url,
            hex::encode(hash_bytes),
            hex::encode(computed_hash)
        );
    }

    Err(ClientError::InvalidHash)
}

async fn fetch_transform_url(
    method: reqwest::Method,
    url: &str,
    acceptable_languages: &str,
) -> Result<ClientResponse, ClientError> {
    let request_client = match crate::lib::REQUEST_CLIENT.get() {
        Some(client) => client,
        None => return Err(ClientError::RequestClient),
    };

    let response = request_client
        .request(method.clone(), url)
        .header(reqwest::header::ACCEPT, "*/*")
        .header(reqwest::header::ACCEPT_LANGUAGE, acceptable_languages)
        .send()
        .await?;
    let status_code = response.status().as_u16();

    if status_code == 200 {
        return transform_response(response).await;
    }

    Err(ClientError::UnexpectedStatusCode(status_code))
}

async fn transform_response(response: reqwest::Response) -> Result<ClientResponse, ClientError> {
    let headers = response.headers();
    let content_type: mime::Mime = match headers.get(reqwest::header::CONTENT_TYPE) {
        Some(value) => value.to_str()?.parse()?,
        None => mime::TEXT_PLAIN,
    };

    Ok(
        if content_type == mime::TEXT_HTML || content_type == mime::TEXT_HTML_UTF_8 {
            ClientResponse {
                body: BodyType::Complete(transform_html(response).await?),
                content_disposition: None,
                content_type,
            }
        } else if content_type == mime::TEXT_CSS || content_type == mime::TEXT_CSS_UTF_8 {
            ClientResponse {
                body: BodyType::Complete(transform_css(response).await?),
                content_disposition: None,
                content_type,
            }
        } else {
            ClientResponse {
                content_disposition: headers.get(reqwest::header::CONTENT_DISPOSITION).cloned(),
                body: BodyType::Stream(Box::pin(response.bytes_stream())),
                content_type,
            }
        },
    )
}

async fn transform_html(response: reqwest::Response) -> Result<bytes::Bytes, ClientError> {
    let base_url = response.url().clone();
    let mut rewriter = HtmlRewrite::new(&base_url);
    let mut stream = response.bytes_stream();

    while let Some(chunk_res) = stream.next().await {
        rewriter.write(chunk_res?.as_ref())?;
    }

    Ok(bytes::Bytes::from(rewriter.end()?))
}

async fn transform_css(response: reqwest::Response) -> Result<bytes::Bytes, ClientError> {
    let base_url = response.url().clone();
    let mut rewriter = CssRewrite::new(&base_url);
    let mut stream = response.bytes_stream();

    while let Some(chunk_res) = stream.next().await {
        rewriter.write(chunk_res?.as_ref())?;
    }

    Ok(bytes::Bytes::from(rewriter.end()?))
}
