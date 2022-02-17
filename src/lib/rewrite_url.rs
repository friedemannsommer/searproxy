#[derive(thiserror::Error, Debug)]
pub enum RewriteUrlError {
    #[error("HMAC instance uninitialized")]
    HmacInstance,
    #[error("URL parsing failed")]
    UrlParse(#[from] url::ParseError),
    #[error("Serialization failed")]
    Serialize(#[from] serde_qs::Error),
    #[error("data URI scheme rejected")]
    DataUriRejected(String),
}

pub fn rewrite_url(base_url: &url::Url, url: &str) -> Result<String, RewriteUrlError> {
    if url.starts_with("data:") {
        return if url.starts_with("data:image/") {
            Ok(String::from(url))
        } else {
            Err(RewriteUrlError::DataUriRejected(String::from(url)))
        };
    }

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

#[cfg(test)]
mod tests {
    use crate::lib::rewrite_url::rewrite_url;

    #[test]
    fn rewrite_relative() {
        crate::lib::test_setup_hmac();

        assert_eq!(
            rewrite_url(&url::Url::parse("https://www.example.com").unwrap(), "/index.html")
                .unwrap()
                .as_str(),
            "./?mortyurl=https%3A%2F%2Fwww.example.com%2Findex.html&mortyhash=7554946c4d3998da8be40b803c938c943f3dbbbb78958addd008b55bcacfb8c0"
        );
    }

    #[test]
    fn rewrite_relative_parent() {
        crate::lib::test_setup_hmac();

        assert_eq!(
            rewrite_url(&url::Url::parse("https://www.example.com/home/about").unwrap(), "../index.html")
                .unwrap()
                .as_str(),
            "./?mortyurl=https%3A%2F%2Fwww.example.com%2Findex.html&mortyhash=7554946c4d3998da8be40b803c938c943f3dbbbb78958addd008b55bcacfb8c0"
        );
    }

    #[test]
    fn rewrite_absolute() {
        crate::lib::test_setup_hmac();

        assert_eq!(
            rewrite_url(&url::Url::parse("https://example.com/").unwrap(), "https://www.example.com/")
                .unwrap()
                .as_str(),
            "./?mortyurl=https%3A%2F%2Fwww.example.com%2F&mortyhash=85870232cac1676c4477f7cae4da7173ccee4002f32e89c16038547aa20175c0"
        );
    }

    #[test]
    fn accept_data_image_png() {
        crate::lib::test_setup_hmac();

        assert_eq!(
            rewrite_url(
                &url::Url::parse("https://example.com/").unwrap(),
                "data:image/png;base64,dGVzdA=="
            )
            .unwrap()
            .as_str(),
            "data:image/png;base64,dGVzdA=="
        );
    }

    #[test]
    fn accept_data_image_jpg() {
        crate::lib::test_setup_hmac();

        assert_eq!(
            rewrite_url(
                &url::Url::parse("https://example.com/").unwrap(),
                "data:image/jpg;base64,dGVzdA=="
            )
            .unwrap()
            .as_str(),
            "data:image/jpg;base64,dGVzdA=="
        );
    }

    #[test]
    fn reject_data_script() {
        crate::lib::test_setup_hmac();

        assert!(rewrite_url(
            &url::Url::parse("https://example.com/").unwrap(),
            "data:application/javascript;base64,dGVzdA=="
        )
        .is_err());
    }

    #[test]
    fn reject_data_text() {
        crate::lib::test_setup_hmac();

        assert!(rewrite_url(
            &url::Url::parse("https://example.com/").unwrap(),
            "data:text/plain;base64,dGVzdA=="
        )
        .is_err());
    }
}
