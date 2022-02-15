#[derive(thiserror::Error, Debug)]
pub enum RewriteUrlError {
    #[error("HMAC instance uninitialized")]
    HmacInstance,
    #[error("URL parsing failed")]
    UrlParse(#[from] url::ParseError),
    #[error("Serialization failed")]
    Serialize(#[from] serde_qs::Error),
}

pub fn rewrite_url(base_url: &url::Url, url: &str) -> Result<String, RewriteUrlError> {
    let _span = tracing::span!(
        tracing::Level::TRACE,
        "rewrite_url",
        http.base_url = base_url.as_str(),
        http.url = url
    )
    .entered();
    let mut hmac = match crate::lib::HMAC.get() {
        Some(instance) => instance.clone(),
        None => return Err(RewriteUrlError::HmacInstance),
    };
    let next_base_url = base_url.join(url)?;
    let next_url = next_base_url.to_string();

    hmac.update(next_url.as_bytes());

    Ok(format!(
        "./?{}",
        serde_qs::to_string(&crate::model::IndexHttpArgs {
            hash: Some(hex::encode(&hmac.finalize())),
            url: Some(next_url)
        })?
    ))
}
