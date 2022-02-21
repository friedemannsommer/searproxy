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
    if url.starts_with("data:") {
        return if url.starts_with("data:image/") {
            Ok(String::from(url))
        } else {
            Ok(String::new())
        };
    } else if url.starts_with('#') {
        // since a fragment is client side, there is no need to rewrite this
        return Ok(String::from(url));
    }

    let mut hmac = match crate::lib::HMAC.get() {
        Some(instance) => instance.clone(),
        None => return Err(RewriteUrlError::HmacInstance),
    };
    let mut next_base_url = base_url.join(url)?;
    // `./?` (3) + `mortyurl` (8) + `mortyhash` (9) + "hash" (64) + `next_base_url.len()` (* 2 [for url encoding])
    let mut result = String::with_capacity(3 + 8 + 9 + 64 + (next_base_url.as_str().len() * 2));
    let next_url_fragment: Option<String> = next_base_url.fragment().map(String::from);
    let _capacity = result.capacity();

    if next_url_fragment.is_some() {
        // exclude fragment from request URL
        next_base_url.set_fragment(None);
    }

    let next_url = next_base_url.to_string();

    hmac.update(next_url.as_bytes());
    result.push_str("./?");
    result.push_str(
        serde_qs::to_string(&crate::model::IndexHttpArgs {
            hash: Some(hex::encode(&hmac.finalize())),
            url: Some(next_url),
        })?
        .as_str(),
    );

    if let Some(fragment) = next_url_fragment {
        result.push('#');
        result.push_str(fragment.as_str());
    }

    Ok(result)
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

        assert_eq!(
            rewrite_url(
                &url::Url::parse("https://example.com/").unwrap(),
                "data:application/javascript;base64,dGVzdA=="
            )
            .unwrap(),
            String::new()
        );
    }

    #[test]
    fn reject_data_text() {
        crate::lib::test_setup_hmac();

        assert_eq!(
            rewrite_url(
                &url::Url::parse("https://example.com/").unwrap(),
                "data:text/plain;base64,dGVzdA=="
            )
            .unwrap(),
            String::new()
        );
    }

    #[test]
    fn pass_through_fragment_ref() {
        crate::lib::test_setup_hmac();

        assert_eq!(
            rewrite_url(&url::Url::parse("https://example.com/").unwrap(), "#about").unwrap(),
            "#about"
        );
    }

    #[test]
    fn rewrite_prefixed_path_fragment_ref() {
        crate::lib::test_setup_hmac();

        assert_eq!(
            rewrite_url(&url::Url::parse("https://example.com/").unwrap(), "/home/#about").unwrap(),
            "./?mortyurl=https%3A%2F%2Fexample.com%2Fhome%2F&mortyhash=3af87c981235827014507736715a403ebd2f9c875689318184ba2cc035ea3e61#about"
        );
    }

    #[test]
    fn rewrite_prefixed_fragment_ref() {
        crate::lib::test_setup_hmac();

        assert_eq!(
            rewrite_url(&url::Url::parse("https://example.com/").unwrap(), "https://another.example.com/#about").unwrap(),
            "./?mortyurl=https%3A%2F%2Fanother.example.com%2F&mortyhash=743bb69ce433c306c9883528f2a7b451531362a1d41bbf6519ed97cdb81b907b#about"
        );
    }
}
