use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, ToSocketAddrs};

use futures_util::StreamExt;

use crate::{
    model::PermittedIpRange,
    utilities::{
        rewrite_css::{CssRewrite, RewriteCssError},
        rewrite_html::HtmlRewrite,
        rewrite_html::HtmlRewriteResult,
        rewrite_url::rewrite_url,
        GLOBAL_CONFIG,
    },
};

#[derive(thiserror::Error, Debug)]
pub enum ClientError {
    #[error("HMAC instance uninitialized")]
    HmacInstance,
    #[error("Hex decode failed")]
    Hex(#[from] hex::FromHexError),
    #[error("Request client is uninitialized")]
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
    #[error("URL rewriting failed")]
    UrlRewrite(#[from] crate::utilities::rewrite_url::RewriteUrlError),
    #[error("HTML rewriting failed")]
    HtmlRewrite(#[from] lol_html::errors::RewritingError),
    #[error("CSS rewriting failed")]
    CssRewrite(#[from] RewriteCssError),
    #[error("Server will not process the request due to a client error")]
    BadRequest,
    #[error("Server returned 3XX status code without a 'Location' header")]
    RedirectWithoutLocation,
    #[error("The IP `{0}` is not within the permitted range(s)")]
    IpRangeDenied(String),
    #[error("Can't resolve hostname `{0}`")]
    ResolveHostname(String),
}

pub enum FetchResult {
    Response(ClientResponse),
    Redirect(ClientRedirect),
}

pub enum BodyType {
    Complete(bytes::Bytes),
    Stream(
        std::pin::Pin<Box<dyn futures_util::Stream<Item = Result<bytes::Bytes, reqwest::Error>>>>,
    ),
}

pub struct FormRequest {
    pub body: std::collections::HashMap<String, String>,
    pub method: reqwest::Method,
}

pub struct ClientResponse {
    pub body: BodyType,
    pub content_disposition: Option<reqwest::header::HeaderValue>,
    pub content_type: mime::Mime,
    pub style_hashes: Option<Vec<String>>,
}

pub struct ClientRedirect {
    pub external_url: String,
    pub internal_url: String,
    pub status_code: reqwest::StatusCode,
}

pub async fn fetch_validate_url(
    url: &str,
    hash: &str,
    acceptable_languages: &str,
    request_body_opt: Option<FormRequest>,
) -> Result<FetchResult, ClientError> {
    use hmac::Mac;
    use std::str::FromStr;

    let mut hmac = match crate::utilities::HMAC.get() {
        Some(instance) => instance.clone(),
        None => return Err(ClientError::HmacInstance),
    };
    let mut next_url = url::Url::from_str(url)?;

    validate_request_host(&next_url)?;

    let (method, request_body) = match request_body_opt {
        Some(payload) => {
            if payload.method == reqwest::Method::POST {
                (payload.method, Some(payload.body))
            } else {
                append_form_params(&mut next_url, payload.body);
                (payload.method, None)
            }
        }
        None => (reqwest::Method::GET, None),
    };
    let hash_bytes = hex::decode(hash)?;

    hmac.update(url.as_bytes());

    if hmac.verify_slice(&hash_bytes).is_ok() {
        log::debug!("{} '{}'", method, next_url.as_str());
        return fetch_transform_url(method, next_url, acceptable_languages, request_body).await;
    }

    if log::log_enabled!(log::Level::Info) {
        log::info!(
            "rejecting request for: '{}' (invalid hash: {})",
            url,
            hex::encode(hash_bytes)
        );
    }

    Err(ClientError::InvalidHash)
}

async fn fetch_transform_url(
    method: reqwest::Method,
    url: url::Url,
    acceptable_languages: &str,
    request_body: Option<std::collections::HashMap<String, String>>,
) -> Result<FetchResult, ClientError> {
    let request_client = match crate::utilities::REQUEST_CLIENT.get() {
        Some(client) => client,
        None => return Err(ClientError::RequestClient),
    };
    let mut request = request_client
        .request(method, url.clone())
        .header(reqwest::header::ACCEPT, "*/*")
        .header(reqwest::header::ACCEPT_LANGUAGE, acceptable_languages);

    if let Some(payload) = request_body {
        request = request.form(&payload);
    }

    let response = request.send().await?;
    let status_code = response.status();

    if status_code == reqwest::StatusCode::OK {
        return Ok(FetchResult::Response(transform_response(response).await?));
    }

    if status_code.is_redirection() {
        return if let Some(location) = response.headers().get(reqwest::header::LOCATION) {
            let redirect_url = location.to_str()?;

            Ok(FetchResult::Redirect(ClientRedirect {
                external_url: url.join(redirect_url)?.to_string(),
                internal_url: String::from(rewrite_url(&url, redirect_url)?),
                status_code,
            }))
        } else {
            Err(ClientError::RedirectWithoutLocation)
        };
    }

    Err(ClientError::UnexpectedStatusCode(status_code.as_u16()))
}

fn append_form_params(url: &mut url::Url, params: std::collections::HashMap<String, String>) {
    let mut query_pairs = url.query_pairs_mut();

    for pair in params.iter() {
        query_pairs.append_pair(pair.0, pair.1);
    }
}

async fn transform_response(response: reqwest::Response) -> Result<ClientResponse, ClientError> {
    let headers = response.headers();
    let content_type: mime::Mime = match headers.get(reqwest::header::CONTENT_TYPE) {
        Some(value) => value.to_str()?.parse()?,
        None => mime::TEXT_PLAIN,
    };

    Ok(
        if content_type == mime::TEXT_HTML || content_type == mime::TEXT_HTML_UTF_8 {
            let rewritten_html = transform_html(response).await?;

            ClientResponse {
                body: BodyType::Complete(bytes::Bytes::from(rewritten_html.html)),
                content_disposition: None,
                content_type,
                style_hashes: Some(rewritten_html.style_hashes),
            }
        } else if content_type == mime::TEXT_CSS || content_type == mime::TEXT_CSS_UTF_8 {
            ClientResponse {
                body: BodyType::Complete(transform_css(response).await?),
                content_disposition: None,
                content_type,
                style_hashes: None,
            }
        } else {
            ClientResponse {
                content_disposition: headers.get(reqwest::header::CONTENT_DISPOSITION).cloned(),
                body: BodyType::Stream(Box::pin(response.bytes_stream())),
                content_type,
                style_hashes: None,
            }
        },
    )
}

async fn transform_html(response: reqwest::Response) -> Result<HtmlRewriteResult, ClientError> {
    let mut rewriter = HtmlRewrite::new(std::rc::Rc::new(response.url().clone()));
    let mut stream = response.bytes_stream();

    while let Some(chunk_res) = stream.next().await {
        rewriter.write(chunk_res?.as_ref())?;
    }

    Ok(rewriter.end()?)
}

async fn transform_css(response: reqwest::Response) -> Result<bytes::Bytes, ClientError> {
    let mut rewriter = CssRewrite::new(std::rc::Rc::new(response.url().clone()));
    let mut stream = response.bytes_stream();

    while let Some(chunk_res) = stream.next().await {
        rewriter.write(chunk_res?.as_ref())?;
    }

    Ok(bytes::Bytes::from(rewriter.end()?))
}

fn validate_request_host(url: &url::Url) -> Result<(), ClientError> {
    if let Some(config) = GLOBAL_CONFIG.get() {
        if let Some(host) = url.host() {
            return match host {
                url::Host::Ipv4(ip_v4) => verify_ip_v4_range(config.permitted_ip_range, ip_v4),
                url::Host::Ipv6(ip_v6) => verify_ip_v6_range(config.permitted_ip_range, ip_v6),
                url::Host::Domain(hostname) => verify_hostname(config.permitted_ip_range, hostname),
            };
        }
    }

    Ok(())
}

fn verify_ip_v4_range(
    permitted_ip_range: PermittedIpRange,
    ip: Ipv4Addr,
) -> Result<(), ClientError> {
    match permitted_ip_range {
        PermittedIpRange::None => Err(ClientError::IpRangeDenied(ip.to_string())),
        PermittedIpRange::Global => {
            if ip.is_link_local()
                || ip.is_broadcast()
                || ip.is_loopback()
                || ip.is_private()
                || ip.is_unspecified()
            {
                Err(ClientError::IpRangeDenied(ip.to_string()))
            } else {
                Ok(())
            }
        }
        PermittedIpRange::Private => {
            if ip.is_link_local() || ip.is_broadcast() || ip.is_loopback() || ip.is_unspecified() {
                Err(ClientError::IpRangeDenied(ip.to_string()))
            } else {
                Ok(())
            }
        }
        PermittedIpRange::Local => {
            if ip.is_link_local() || ip.is_broadcast() {
                Err(ClientError::IpRangeDenied(ip.to_string()))
            } else {
                Ok(())
            }
        }
    }
}

fn verify_ip_v6_range(
    permitted_ip_range: PermittedIpRange,
    ip: Ipv6Addr,
) -> Result<(), ClientError> {
    match permitted_ip_range {
        PermittedIpRange::None => Err(ClientError::IpRangeDenied(ip.to_string())),
        PermittedIpRange::Global | PermittedIpRange::Private => {
            if ip.is_loopback() || ip.is_unspecified() {
                Err(ClientError::IpRangeDenied(ip.to_string()))
            } else {
                Ok(())
            }
        }
        PermittedIpRange::Local => Ok(()),
    }
}

fn verify_hostname(
    permitted_ip_range: PermittedIpRange,
    hostname: &str,
) -> Result<(), ClientError> {
    let ip_addr = if let Some(socket_addr) = (hostname, 80)
        .to_socket_addrs()
        .map_err(|err| {
            log::warn!("Couldn't resolve domain name `{hostname}`, with reason: {err}");
            ClientError::ResolveHostname(hostname.to_string())
        })?
        .next()
    {
        socket_addr.ip()
    } else {
        return Err(ClientError::ResolveHostname(hostname.to_string()));
    };

    match ip_addr {
        IpAddr::V4(ip_v4) => verify_ip_v4_range(permitted_ip_range, ip_v4),
        IpAddr::V6(ip_v6) => verify_ip_v6_range(permitted_ip_range, ip_v6),
    }
}

#[cfg(test)]
mod tests {
    use std::net::{Ipv4Addr, Ipv6Addr};

    use crate::model::PermittedIpRange;

    use super::{verify_ip_v4_range, verify_ip_v6_range};

    const IP_V4_DENIED_IP_LIST: [Ipv4Addr; 2] = [
        Ipv4Addr::new(169, 254, 0, 0),
        Ipv4Addr::new(255, 255, 255, 255),
    ];

    const IP_V4_GLOBAL_IP_LIST: [Ipv4Addr; 5] = [
        Ipv4Addr::new(1, 1, 1, 1),
        Ipv4Addr::new(1, 0, 0, 1),
        Ipv4Addr::new(8, 8, 8, 8),
        Ipv4Addr::new(8, 8, 4, 4),
        Ipv4Addr::new(9, 9, 9, 9),
    ];

    const IP_V4_PRIVATE_IP_LIST: [Ipv4Addr; 7] = [
        Ipv4Addr::new(10, 0, 0, 1),
        Ipv4Addr::new(10, 1, 0, 1),
        Ipv4Addr::new(10, 10, 0, 1),
        Ipv4Addr::new(172, 16, 0, 1),
        Ipv4Addr::new(172, 20, 0, 1),
        Ipv4Addr::new(192, 168, 0, 1),
        Ipv4Addr::new(192, 168, 1, 1),
    ];

    const IP_V4_LOCAL_IP_LIST: [Ipv4Addr; 7] = [
        Ipv4Addr::new(127, 0, 0, 1),
        Ipv4Addr::new(127, 1, 0, 1),
        Ipv4Addr::new(127, 10, 0, 1),
        Ipv4Addr::new(127, 16, 0, 1),
        Ipv4Addr::new(127, 20, 0, 1),
        Ipv4Addr::new(127, 168, 0, 1),
        Ipv4Addr::new(127, 200, 1, 1),
    ];

    #[test]
    fn ip_v4_range_none() {
        assert!(verify_ip_v4_range(PermittedIpRange::None, Ipv4Addr::UNSPECIFIED).is_err());

        for ip in IP_V4_DENIED_IP_LIST {
            assert!(verify_ip_v4_range(PermittedIpRange::None, ip).is_err());
        }

        for ip in IP_V4_GLOBAL_IP_LIST {
            assert!(verify_ip_v4_range(PermittedIpRange::None, ip).is_err());
        }

        for ip in IP_V4_PRIVATE_IP_LIST {
            assert!(verify_ip_v4_range(PermittedIpRange::None, ip).is_err());
        }

        for ip in IP_V4_LOCAL_IP_LIST {
            assert!(verify_ip_v4_range(PermittedIpRange::None, ip).is_err());
        }
    }

    #[test]
    fn ip_v4_range_global() {
        assert!(verify_ip_v4_range(PermittedIpRange::Global, Ipv4Addr::UNSPECIFIED).is_err());

        for ip in IP_V4_DENIED_IP_LIST {
            assert!(verify_ip_v4_range(PermittedIpRange::Global, ip).is_err());
        }

        for ip in IP_V4_GLOBAL_IP_LIST {
            assert!(verify_ip_v4_range(PermittedIpRange::Global, ip).is_ok());
        }

        for ip in IP_V4_PRIVATE_IP_LIST {
            assert!(verify_ip_v4_range(PermittedIpRange::Global, ip).is_err());
        }

        for ip in IP_V4_LOCAL_IP_LIST {
            assert!(verify_ip_v4_range(PermittedIpRange::Global, ip).is_err());
        }
    }

    #[test]
    fn ip_v4_range_private() {
        assert!(verify_ip_v4_range(PermittedIpRange::Private, Ipv4Addr::UNSPECIFIED).is_err());

        for ip in IP_V4_DENIED_IP_LIST {
            assert!(verify_ip_v4_range(PermittedIpRange::Private, ip).is_err());
        }

        for ip in IP_V4_GLOBAL_IP_LIST {
            assert!(verify_ip_v4_range(PermittedIpRange::Private, ip).is_ok());
        }

        for ip in IP_V4_PRIVATE_IP_LIST {
            assert!(verify_ip_v4_range(PermittedIpRange::Private, ip).is_ok());
        }

        for ip in IP_V4_LOCAL_IP_LIST {
            assert!(verify_ip_v4_range(PermittedIpRange::Private, ip).is_err());
        }
    }

    #[test]
    fn ip_v4_range_local() {
        assert!(verify_ip_v4_range(PermittedIpRange::Local, Ipv4Addr::UNSPECIFIED).is_ok());

        for ip in IP_V4_DENIED_IP_LIST {
            assert!(verify_ip_v4_range(PermittedIpRange::Local, ip).is_err());
        }

        for ip in IP_V4_GLOBAL_IP_LIST {
            assert!(verify_ip_v4_range(PermittedIpRange::Local, ip).is_ok());
        }

        for ip in IP_V4_PRIVATE_IP_LIST {
            assert!(verify_ip_v4_range(PermittedIpRange::Local, ip).is_ok());
        }

        for ip in IP_V4_LOCAL_IP_LIST {
            assert!(verify_ip_v4_range(PermittedIpRange::Local, ip).is_ok());
        }
    }

    #[test]
    fn ip_v6_range_none() {
        assert!(verify_ip_v6_range(PermittedIpRange::None, Ipv6Addr::UNSPECIFIED).is_err());
        assert!(verify_ip_v6_range(PermittedIpRange::None, Ipv6Addr::LOCALHOST).is_err());
    }

    #[test]
    fn ip_v6_range_global() {
        assert!(verify_ip_v6_range(PermittedIpRange::Global, Ipv6Addr::UNSPECIFIED).is_err());
        assert!(verify_ip_v6_range(PermittedIpRange::Global, Ipv6Addr::LOCALHOST).is_err());
    }

    #[test]
    fn ip_v6_range_private() {
        assert!(verify_ip_v6_range(PermittedIpRange::Private, Ipv6Addr::UNSPECIFIED).is_err());
        assert!(verify_ip_v6_range(PermittedIpRange::Private, Ipv6Addr::LOCALHOST).is_err());
    }

    #[test]
    fn ip_v6_range_local() {
        assert!(verify_ip_v6_range(PermittedIpRange::Local, Ipv6Addr::UNSPECIFIED).is_ok());
        assert!(verify_ip_v6_range(PermittedIpRange::Local, Ipv6Addr::LOCALHOST).is_ok());
    }
}
