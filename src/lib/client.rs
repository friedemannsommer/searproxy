use crate::model::IndexHttpArgs;
use futures_util::StreamExt;
use tracing::{debug, span};

const MAX_REDIRECTS: u8 = 30;

#[derive(thiserror::Error, Debug)]
pub enum ClientError {
    #[error("config is uninitialized")]
    GlobalConfig,
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
    #[error("CSS parsing failed")]
    CssParse,
    #[error("CSS formatting / rewriting failed")]
    CssPrint,
    #[error("HTML rewriting failed")]
    HtmlRewrite(#[from] lol_html::errors::RewritingError),
    #[error("Querystring encoding failed")]
    QuerystringEncoding(#[from] serde_qs::Error),
}

pub struct ClientResponse {
    pub body: bytes::Bytes,
    pub content_disposition: Option<reqwest::header::HeaderValue>,
    pub content_type: mime::Mime,
}

static IMG_SRCSET_REGEX: once_cell::sync::Lazy<regex::Regex> = once_cell::sync::Lazy::new(|| {
    regex::Regex::new(r"(?P<url>[\w#!:.?+=&%@!\-/]+)(\s+(?:[0-9]+\.)?[0-9]+[xw]\s*[,$]?|$)")
        .expect("RegExp compilation failed")
});

pub async fn fetch_validate_url(
    method: reqwest::Method,
    url: &str,
    hash: &str,
    acceptable_languages: &str,
) -> Result<ClientResponse, ClientError> {
    let _span = tracing::span!(
        tracing::Level::TRACE,
        "fetch_validate_url",
        http.method = method.as_str(),
        http.url = url
    )
    .entered();
    let mut hmac = match crate::lib::HMAC.get() {
        Some(instance) => instance.clone(),
        None => return Err(ClientError::HmacInstance),
    };

    let hash_bytes = hex::decode(hash)?;
    let computed_hash = {
        let _hmac_span = span!(tracing::Level::TRACE, "hmac_validation");

        hmac.update(url.as_bytes());
        hmac.finalize()
    };

    if hash_bytes == computed_hash {
        debug!("{} '{}'", method, url);
        return fetch_transform_url(method, url, acceptable_languages, 0).await;
    }

    debug!(
        "rejecting request for: '{}' (invalid hash: {:?} != {:?})",
        url, hash_bytes, computed_hash
    );

    Err(ClientError::InvalidHash)
}

#[async_recursion::async_recursion(?Send)]
async fn fetch_transform_url(
    method: reqwest::Method,
    url: &str,
    acceptable_languages: &str,
    depth: u8,
) -> Result<ClientResponse, ClientError> {
    let _span = tracing::span!(
        tracing::Level::TRACE,
        "fetch_transform_url",
        http.method = method.as_str(),
        http.url = url,
        depth = depth,
    )
    .entered();
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

    match status_code {
        301 | 302 | 303 | 307 | 308 => {
            if depth < MAX_REDIRECTS
                && match crate::lib::GLOBAL_CONFIG.get() {
                    Some(config) => config.follow_redirect,
                    None => return Err(ClientError::GlobalConfig),
                }
            {
                if let Some(location) = response.headers().get(reqwest::header::LOCATION) {
                    let next_url = url::Url::parse(url)?.join(location.to_str()?)?;

                    return fetch_transform_url(
                        method,
                        next_url.as_str(),
                        acceptable_languages,
                        depth + 1,
                    )
                    .await;
                }
            }

            Err(ClientError::UnexpectedStatusCode(status_code))
        }
        200 => transform_response(response).await,
        _ => Err(ClientError::UnexpectedStatusCode(status_code)),
    }
}

async fn transform_response(response: reqwest::Response) -> Result<ClientResponse, ClientError> {
    let _span = tracing::span!(
        tracing::Level::TRACE,
        "transform_response",
        http.url = response.url().as_str(),
        http.status_code = response.status().as_u16()
    )
    .entered();
    let headers = response.headers();
    let content_type: mime::Mime = match headers.get(reqwest::header::CONTENT_TYPE) {
        Some(value) => value.to_str()?.parse()?,
        None => mime::TEXT_PLAIN,
    };

    Ok(
        if content_type == mime::TEXT_HTML || content_type == mime::TEXT_HTML_UTF_8 {
            ClientResponse {
                body: transform_html(response).await?,
                content_disposition: None,
                content_type,
            }
        } else if content_type == mime::TEXT_CSS || content_type == mime::TEXT_CSS_UTF_8 {
            ClientResponse {
                body: transform_stylesheet(response).await?,
                content_disposition: None,
                content_type,
            }
        } else {
            ClientResponse {
                content_disposition: headers.get(reqwest::header::CONTENT_DISPOSITION).cloned(),
                body: response.bytes().await?,
                content_type,
            }
        },
    )
}

async fn transform_html(response: reqwest::Response) -> Result<bytes::Bytes, ClientError> {
    let _span = tracing::span!(
        tracing::Level::TRACE,
        "transform_html",
        http.url = response.url().as_str()
    )
    .entered();
    let base_url = response.url().clone();
    let transform_href = |
        element: &mut lol_html::html_content::Element,
    | -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        element.set_attribute("href", &rewrite_url(&base_url, &element.get_attribute("href").unwrap())?)?;
        Ok(())
    };
    let mut output_vec = Vec::default();
    let mut rewriter = lol_html::HtmlRewriter::new(
        lol_html::Settings {
            element_content_handlers: vec![
                lol_html::element!("a[href]", transform_href),
                lol_html::element!("link[href]", transform_href),
                lol_html::element!("script", html_remove_element),
                lol_html::element!("applet", html_remove_element),
                lol_html::element!("canvas", html_remove_element),
                lol_html::element!("embed", html_remove_element),
                lol_html::element!("math", html_remove_element),
                lol_html::element!("svg", html_remove_element),
                lol_html::element!("img[src]", |img| {
                    img.set_attribute(
                        "src",
                        &rewrite_url(&base_url, &img.get_attribute("src").unwrap())?,
                    )?;
                    Ok(())
                }),
                lol_html::element!("img[srcset]", |img| {
                    let src_set_values = img.get_attribute("srcset").unwrap();
                    let mut output = String::with_capacity(src_set_values.len());
                    let mut offset = 0;

                    for group in IMG_SRCSET_REGEX.captures_iter(&src_set_values) {
                        if let Some(matched_url) = group.name("url") {
                            let proxied_url = rewrite_url(&base_url, matched_url.as_str())?;

                            output.push_str(&src_set_values[offset..matched_url.start()]);
                            output.push_str(&proxied_url);
                            offset = matched_url.end();
                        }
                    }

                    output.push_str(&src_set_values[offset..]);
                    img.set_attribute("srcset", &output)?;

                    Ok(())
                }),
            ],
            ..lol_html::Settings::default()
        },
        |chunk: &[u8]| output_vec.extend_from_slice(chunk),
    );
    let mut stream = response.bytes_stream();

    while let Some(chunk_res) = stream.next().await {
        rewriter.write(chunk_res?.as_ref())?;
    }

    rewriter.end()?;

    Ok(bytes::Bytes::from(minify_html::minify(
        &output_vec,
        &crate::lib::MINIFY_CONFIG,
    )))
}

fn html_remove_element(
    element: &mut lol_html::html_content::Element,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    element.remove();
    Ok(())
}

async fn transform_stylesheet(response: reqwest::Response) -> Result<bytes::Bytes, ClientError> {
    let _span = tracing::span!(
        tracing::Level::TRACE,
        "transform_stylesheet",
        css.url = response.url().as_str()
    )
    .entered();
    let base_url = response.url().clone();
    let raw_css_text = response.text().await?;
    // todo: the CSS parser seemingly doesn't support browser quirks ("quirks mode")
    let mut stylesheet = match parcel_css::stylesheet::StyleSheet::parse(
        String::from("_sp_main.css"),
        &raw_css_text,
        parcel_css::stylesheet::ParserOptions {
            css_modules: true,
            custom_media: false,
            nesting: true,
        },
    ) {
        Ok(value) => value,
        Err(err) => {
            debug!("{:?}", err);
            return Err(ClientError::CssParse);
        }
    };

    for rule in &mut stylesheet.rules.0 {
        match rule {
            parcel_css::rules::CssRule::Import(import_rule) => {
                import_rule.url =
                    cssparser::CowRcStr::from(rewrite_url(&base_url, &import_rule.url)?);
            }
            parcel_css::rules::CssRule::FontFace(font_face_rule) => {
                for property in &mut font_face_rule.properties {
                    if let parcel_css::rules::font_face::FontFaceProperty::Source(sources) =
                        property
                    {
                        for source in sources {
                            if let parcel_css::rules::font_face::Source::Url(remote_source) = source
                            {
                                remote_source.url.url = cssparser::CowRcStr::from(rewrite_url(
                                    &base_url,
                                    &remote_source.url.url,
                                )?);
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    Ok(bytes::Bytes::from(
        match stylesheet.to_css(parcel_css::stylesheet::PrinterOptions {
            analyze_dependencies: false,
            minify: true,
            pseudo_classes: None,
            source_map: false,
            targets: None,
        }) {
            Ok(printer_result) => printer_result,
            Err(err) => {
                debug!("{:?}", err);
                return Err(ClientError::CssPrint);
            }
        }
        .code,
    ))
}

fn rewrite_url(base_url: &url::Url, url: &str) -> Result<String, ClientError> {
    let _span = tracing::span!(
        tracing::Level::TRACE,
        "rewrite_url",
        http.base_url = base_url.as_str(),
        http.url = url
    )
    .entered();
    let mut hmac = match crate::lib::HMAC.get() {
        Some(instance) => instance.clone(),
        None => return Err(ClientError::HmacInstance),
    };
    let next_base_url = base_url.join(url)?;
    let next_url = next_base_url.to_string();

    hmac.update(next_url.as_bytes());

    Ok(format!(
        "./?{}",
        serde_qs::to_string(&IndexHttpArgs {
            hash: Some(hex::encode(&hmac.finalize())),
            url: Some(next_url)
        })?
    ))
}
