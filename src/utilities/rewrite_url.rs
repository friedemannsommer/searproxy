#[derive(thiserror::Error, Debug)]
pub enum RewriteUrlError {
    #[error("HMAC instance uninitialized")]
    HmacInstance,
    #[error("URL parsing failed")]
    UrlParse(#[from] url::ParseError),
    #[error("Serialization failed")]
    Serialize(#[from] serde_qs::Error),
    #[error("Failed to create UTF-8 string")]
    Utf8String(#[from] std::string::FromUtf8Error),
}

pub fn rewrite_url<'url>(
    base_url: &url::Url,
    url: &'url str,
) -> Result<std::borrow::Cow<'url, str>, RewriteUrlError> {
    use hmac::Mac;

    if url.starts_with("data:") {
        return if url.starts_with("data:image/") {
            Ok(std::borrow::Cow::Borrowed(url))
        } else {
            Ok(std::borrow::Cow::Borrowed(""))
        };
    } else if url.starts_with('#') {
        // since a fragment is client side, there is no need to rewrite this
        return Ok(std::borrow::Cow::Borrowed(url));
    }

    let mut hmac = match crate::utilities::HMAC.get() {
        Some(instance) => instance.clone(),
        None => return Err(RewriteUrlError::HmacInstance),
    };
    let mut next_base_url = base_url.join(url)?;
    // `./` (2) + `?url=` (5) + `&hash=` (6) + "hash" (64) + `next_base_url.len()` (* 2 [url encoding])
    let mut result = Vec::with_capacity(2 + 5 + 6 + 64 + (next_base_url.as_str().len() * 2));
    let next_url_fragment: Option<String> = next_base_url.fragment().map(String::from);

    if next_url_fragment.is_some() {
        // exclude fragment from request URL
        next_base_url.set_fragment(None);
    }

    let next_url = next_base_url.to_string();

    hmac.update(next_url.as_bytes());
    result.extend_from_slice("./?".as_bytes());

    serde_qs::to_writer(
        &crate::model::IndexHttpArgs {
            hash: Some(hex::encode(hmac.finalize().into_bytes())),
            url: Some(next_url),
        },
        &mut result,
    )?;

    if let Some(fragment) = next_url_fragment {
        result.push(b'#');
        result.extend_from_slice(fragment.as_bytes());
    }

    Ok(std::borrow::Cow::Owned(String::from_utf8(result)?))
}

#[cfg(test)]
mod tests {
    use crate::utilities::rewrite_url::rewrite_url;

    #[test]
    fn rewrite_relative() {
        crate::utilities::test_setup_hmac();

        assert_eq!(
            rewrite_url(
                &url::Url::parse("https://www.example.com").unwrap(),
                "/index.html"
            )
            .unwrap(),
            "./?url=https%3A%2F%2Fwww.example.com%2Findex.html&hash=7554946c4d3998da8be40b803c938c943f3dbbbb78958addd008b55bcacfb8c0"
        );
    }

    #[test]
    fn rewrite_relative_parent() {
        crate::utilities::test_setup_hmac();

        assert_eq!(
            rewrite_url(
                &url::Url::parse("https://www.example.com/home/about").unwrap(),
                "../index.html"
            )
            .unwrap(),
            "./?url=https%3A%2F%2Fwww.example.com%2Findex.html&hash=7554946c4d3998da8be40b803c938c943f3dbbbb78958addd008b55bcacfb8c0"
        );
    }

    #[test]
    fn rewrite_absolute() {
        crate::utilities::test_setup_hmac();

        assert_eq!(
            rewrite_url(
                &url::Url::parse("https://example.com/").unwrap(),
                "https://www.example.com/"
            )
            .unwrap(),
            "./?url=https%3A%2F%2Fwww.example.com%2F&hash=85870232cac1676c4477f7cae4da7173ccee4002f32e89c16038547aa20175c0"
        );
    }

    #[test]
    fn accept_data_image_png() {
        crate::utilities::test_setup_hmac();

        assert_eq!(
            rewrite_url(
                &url::Url::parse("https://example.com/").unwrap(),
                "data:image/png;base64,dGVzdA=="
            )
            .unwrap(),
            "data:image/png;base64,dGVzdA=="
        );
    }

    #[test]
    fn accept_data_image_jpg() {
        crate::utilities::test_setup_hmac();

        assert_eq!(
            rewrite_url(
                &url::Url::parse("https://example.com/").unwrap(),
                "data:image/jpg;base64,dGVzdA=="
            )
            .unwrap(),
            "data:image/jpg;base64,dGVzdA=="
        );
    }

    #[test]
    fn reject_data_script() {
        crate::utilities::test_setup_hmac();

        assert_eq!(
            rewrite_url(
                &url::Url::parse("https://example.com/").unwrap(),
                "data:application/javascript;base64,dGVzdA=="
            )
            .unwrap(),
            ""
        );
    }

    #[test]
    fn reject_data_text() {
        crate::utilities::test_setup_hmac();

        assert_eq!(
            rewrite_url(
                &url::Url::parse("https://example.com/").unwrap(),
                "data:text/plain;base64,dGVzdA=="
            )
            .unwrap(),
            ""
        );
    }

    #[test]
    fn pass_through_fragment_ref() {
        crate::utilities::test_setup_hmac();

        assert_eq!(
            rewrite_url(&url::Url::parse("https://example.com/").unwrap(), "#about").unwrap(),
            "#about"
        );
    }

    #[test]
    fn rewrite_prefixed_path_fragment_ref() {
        crate::utilities::test_setup_hmac();

        assert_eq!(
            rewrite_url(
                &url::Url::parse("https://example.com/").unwrap(),
                "/home/#about"
            )
            .unwrap(),
            "./?url=https%3A%2F%2Fexample.com%2Fhome%2F&hash=3af87c981235827014507736715a403ebd2f9c875689318184ba2cc035ea3e61#about"
        );
    }

    #[test]
    fn rewrite_prefixed_fragment_ref() {
        crate::utilities::test_setup_hmac();

        assert_eq!(
            rewrite_url(
                &url::Url::parse("https://example.com/").unwrap(),
                "https://another.example.com/#about"
            )
            .unwrap(),
            "./?url=https%3A%2F%2Fanother.example.com%2F&hash=743bb69ce433c306c9883528f2a7b451531362a1d41bbf6519ed97cdb81b907b#about"
        );
    }
}
