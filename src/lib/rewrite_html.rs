use std::{cell::RefCell, collections::HashSet, rc::Rc};

use lol_html::html_content::{Element, EndTag, TextChunk};

use crate::{
    assets::HEADER_STYLESHEET,
    lib::{rewrite_css::CssRewrite, rewrite_url::rewrite_url},
};

type CssRewriteRef = Rc<RefCell<Option<CssRewrite>>>;
type NoScriptBuffer = Rc<RefCell<String>>;
type OutputSink = Box<dyn Fn(&[u8])>;
type StyleHashList = Rc<RefCell<Vec<String>>>;

pub struct HtmlRewrite<'html> {
    output: Rc<RefCell<Vec<u8>>>,
    rewriter: lol_html::HtmlRewriter<'html, OutputSink>,
    style_hashes: StyleHashList,
}

pub struct HtmlRewriteResult {
    pub html: Vec<u8>,
    pub style_hashes: Vec<String>,
}

const ALLOWED_META_EQUIV_VALUES: [&str; 3] = ["content-type", "refresh", "x-ua-compatible"];
const ALLOWED_META_ATTRIBUTES: [&str; 3] = ["charset", "content", "http-equiv"];
const ALLOWED_LINK_REL_VALUES: [&str; 7] = [
    "alternate stylesheet",
    "alternate",
    "help",
    "icon",
    "license",
    "shortcut icon",
    "stylesheet",
];

static IMG_SRCSET_REGEX: once_cell::sync::Lazy<regex::Regex> = once_cell::sync::Lazy::new(|| {
    regex::Regex::new(
        r"(?P<url>[\w#!;:.,?~+=*&%@!(')$/\-\[\]]+)(?:\s+(?:[0-9]+\.)?[0-9]+[xw]\s*(?:[0-9]+h)?\s*,?|$)",
    )
    .expect("RegExp compilation failed")
});

static META_EQUIV_REFRESH: once_cell::sync::Lazy<regex::Regex> = once_cell::sync::Lazy::new(|| {
    regex::Regex::new(r"(?i)[0-9]+\s*;\s*url\s*=\s*(?P<url>[^$]+)")
        .expect("RegExp compilation failed")
});

static HEADER_STYLE_ELEMENT: once_cell::sync::Lazy<String> =
    once_cell::sync::Lazy::new(|| format!("<style>{}</style>", HEADER_STYLESHEET));

static ALLOWED_ATTRIBUTES: once_cell::sync::Lazy<HashSet<&'static str>> =
    once_cell::sync::Lazy::new(|| {
        HashSet::from([
            "abbr",
            "accesskey",
            "action",
            "align",
            "alt",
            "as",
            "autocomplete",
            "charset",
            "checked",
            "class",
            "content",
            "contenteditable",
            "csp",
            "dir",
            "disabled",
            "for",
            "frameborder",
            "height",
            "hidden",
            "href",
            "hreflang",
            "id",
            "lang",
            "loading",
            "media",
            "method",
            "name",
            "nowrap",
            "placeholder",
            "prefetch",
            "property",
            "rel",
            "sandbox",
            "scrolling",
            "sizes",
            "spellcheck",
            "src",
            "srcset",
            "tabindex",
            "target",
            "title",
            "translate",
            "type",
            "value",
            "width",
        ])
    });

impl<'html> HtmlRewrite<'html> {
    pub fn new(url: Rc<url::Url>) -> Self {
        let output = Rc::new(RefCell::new(Vec::<u8>::new()));
        let css_rewriter: CssRewriteRef = Rc::new(RefCell::new(None));
        let style_hashes: StyleHashList = Rc::new(RefCell::new(Vec::<String>::new()));
        let noscript_buf: NoScriptBuffer = Rc::new(RefCell::new(String::new()));

        Self {
            output: output.clone(),
            rewriter: lol_html::HtmlRewriter::new(
                lol_html::Settings {
                    element_content_handlers: vec![
                        lol_html::element!("*", Self::remove_disallowed_attributes),
                        lol_html::element!("*[href]", Self::transform_href(url.clone())),
                        lol_html::element!("*[src]", Self::transform_src(url.clone())),
                        lol_html::element!("applet", Self::remove_element),
                        lol_html::element!("base", Self::remove_element),
                        lol_html::element!("body", Self::append_proxy_header(url.clone())),
                        lol_html::element!("canvas", Self::remove_element),
                        lol_html::element!("embed", Self::remove_element),
                        lol_html::element!("form", Self::transform_form(url.clone())),
                        lol_html::element!("head", Self::append_proxy_styles),
                        lol_html::element!(
                            "img",
                            Self::transform_img(match crate::lib::shared::GLOBAL_CONFIG.get() {
                                Some(config) => config.lazy_images,
                                _ => false,
                            })
                        ),
                        lol_html::element!("img[srcset]", Self::transform_srcset(url.clone())),
                        lol_html::element!("source[srcset]", Self::transform_srcset(url.clone())),
                        lol_html::element!("link", Self::filter_link_elements),
                        lol_html::element!("math", Self::remove_element),
                        lol_html::element!("meta", Self::filter_meta_elements(url.clone())),
                        lol_html::element!(
                            "noscript",
                            Self::transform_noscript(
                                url.clone(),
                                noscript_buf.clone(),
                                style_hashes.clone()
                            )
                        ),
                        lol_html::element!("script", Self::remove_element),
                        lol_html::element!(
                            "style",
                            Self::transform_style(url, css_rewriter.clone(), style_hashes.clone())
                        ),
                        lol_html::element!("svg", Self::remove_element),
                        lol_html::text!("noscript", Self::write_noscript_content(noscript_buf)),
                        lol_html::text!("style", Self::write_style(css_rewriter)),
                    ],
                    ..lol_html::Settings::default()
                },
                Box::new(move |chunk: &[u8]| {
                    output.borrow_mut().extend_from_slice(chunk);
                }),
            ),
            style_hashes,
        }
    }

    pub fn write(&mut self, data: &[u8]) -> Result<(), lol_html::errors::RewritingError> {
        self.rewriter.write(data)
    }

    pub fn end(self) -> Result<HtmlRewriteResult, lol_html::errors::RewritingError> {
        self.rewriter.end()?;

        Ok(HtmlRewriteResult {
            html: self.output.take(),
            style_hashes: self.style_hashes.take(),
        })
    }

    fn transform_src(
        base_url: Rc<url::Url>,
    ) -> impl Fn(&mut Element<'_, '_>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> + 'html
    {
        move |element| {
            element.set_attribute(
                "src",
                &rewrite_url(
                    base_url.as_ref(),
                    Self::get_unchecked_attribute_value(element, "src").as_str(),
                )?,
            )?;

            Ok(())
        }
    }

    fn transform_img(
        allow_lazy: bool,
    ) -> impl Fn(&mut Element<'_, '_>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> + 'html
    {
        move |element: &mut Element<'_, '_>| {
            if allow_lazy {
                element.set_attribute("loading", "lazy")?;
            } else {
                element.remove_attribute("loading");
            }

            element.set_attribute("decoding", "async")?;

            Ok(())
        }
    }

    fn transform_srcset(
        base_url: Rc<url::Url>,
    ) -> impl Fn(&mut Element<'_, '_>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> + 'html
    {
        move |element| {
            let src_set_values = element.get_attribute("srcset").unwrap();
            let mut output = String::with_capacity(src_set_values.len());
            let mut offset = 0;

            for group in IMG_SRCSET_REGEX.captures_iter(&src_set_values) {
                if let Some(matched_url) = group.name("url") {
                    let html_decoded = Self::html_entity_decode(matched_url.as_str());
                    let proxy_url = rewrite_url(base_url.as_ref(), html_decoded.as_str())?;

                    output.push_str(&src_set_values[offset..matched_url.start()]);
                    output.push_str(&proxy_url);
                    offset = matched_url.end()
                }
            }

            output.push_str(&src_set_values[offset..]);
            element.set_attribute("srcset", &output)?;

            Ok(())
        }
    }

    fn transform_href(
        base_url: Rc<url::Url>,
    ) -> impl Fn(&mut Element<'_, '_>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> + 'html
    {
        move |element: &mut Element<'_, '_>| {
            element.set_attribute(
                "href",
                &rewrite_url(
                    base_url.as_ref(),
                    Self::get_unchecked_attribute_value(element, "href").as_str(),
                )?,
            )?;

            Ok(())
        }
    }

    fn transform_style(
        base_url: Rc<url::Url>,
        css_rewriter: CssRewriteRef,
        style_hashes: StyleHashList,
    ) -> impl Fn(&mut Element<'_, '_>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> + 'html
    {
        move |element: &mut Element<'_, '_>| {
            css_rewriter.replace(Some(CssRewrite::new(base_url.clone())));
            element.on_end_tag(Self::flush_style(
                css_rewriter.clone(),
                style_hashes.clone(),
            ))?;

            Ok(())
        }
    }

    fn flush_style(
        css_rewriter: CssRewriteRef,
        style_hashes: StyleHashList,
    ) -> impl Fn(&mut EndTag<'_>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> + 'static
    {
        move |end| {
            let current_css_rewriter = css_rewriter.replace(None);
            let css_bytes = current_css_rewriter.unwrap().end()?;

            style_hashes.borrow_mut().push(format!(
                "'sha256-{}'",
                base64::encode({
                    use sha2::Digest;

                    let mut hasher = sha2::Sha256::new();

                    hasher.update(css_bytes.as_slice());
                    hasher.finalize()
                })
            ));

            end.before(
                std::str::from_utf8(&css_bytes)?,
                lol_html::html_content::ContentType::Html,
            );

            Ok(())
        }
    }

    fn write_style(
        css_rewriter: CssRewriteRef,
    ) -> impl FnMut(&mut TextChunk<'_>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> + 'html
    {
        move |text: &mut TextChunk<'_>| {
            css_rewriter
                .borrow_mut()
                .as_mut()
                .unwrap()
                .write(text.as_str().as_bytes())?;
            text.remove();
            Ok(())
        }
    }

    fn transform_form(
        base_url: Rc<url::Url>,
    ) -> impl Fn(&mut Element<'_, '_>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        move |element: &mut Element<'_, '_>| {
            use std::str::FromStr;

            element.set_attribute("target", "_self")?;

            if let Some(method) = element.get_attribute("method") {
                element.prepend(
                    format!(
                        r#"<input type="hidden" name="_searproxy_origin_method" value="{}">"#,
                        actix_web::http::Method::from_str(&method)?
                    )
                    .as_str(),
                    lol_html::html_content::ContentType::Html,
                );
            }

            element.set_attribute("method", "POST")?;

            if let Some(action) = element.get_attribute("action") {
                element.set_attribute(
                    "action",
                    &rewrite_url(
                        base_url.as_ref(),
                        Self::html_entity_decode(action.trim()).as_str(),
                    )?,
                )?;
            }

            Ok(())
        }
    }

    fn filter_link_elements(
        element: &mut Element<'_, '_>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(rel) = element.get_attribute("rel") {
            if !ALLOWED_LINK_REL_VALUES.contains(&rel.to_ascii_lowercase().as_str()) {
                element.remove()
            }
        } else {
            element.remove()
        }

        Ok(())
    }

    fn filter_meta_elements(
        base_url: Rc<url::Url>,
    ) -> impl Fn(&mut Element<'_, '_>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        move |element: &mut Element<'_, '_>| {
            if let Some(http_equiv) = element.get_attribute("http-equiv") {
                let lc_equiv = http_equiv.to_ascii_lowercase();
                let lc_equiv_trim = lc_equiv.trim();

                if !ALLOWED_META_EQUIV_VALUES.contains(&lc_equiv_trim) {
                    element.remove()
                }

                if lc_equiv_trim == "refresh" {
                    if let Some(content) = element.get_attribute("content") {
                        if let Some(refresh_capture) = META_EQUIV_REFRESH.captures(&content) {
                            if let Some(url_match) = refresh_capture.name("url") {
                                let html_decoded =
                                    Self::html_entity_decode(url_match.as_str().trim());

                                element.set_attribute(
                                    "content",
                                    format!(
                                        "{}{}",
                                        &content[..url_match.start()],
                                        rewrite_url(&base_url, html_decoded.as_str())?
                                    )
                                    .as_str(),
                                )?;

                                return Ok(());
                            }
                        }
                    }

                    element.remove()
                }
            } else if !element.has_attribute("charset") {
                element.remove()
            }

            Ok(())
        }
    }

    fn transform_noscript(
        base_url: Rc<url::Url>,
        noscript_buf: NoScriptBuffer,
        style_hashes: StyleHashList,
    ) -> impl Fn(&mut Element<'_, '_>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> + 'html
    {
        move |element| {
            element.on_end_tag(Self::flush_noscript_content(
                base_url.clone(),
                noscript_buf.clone(),
                style_hashes.clone(),
            ))?;
            element.remove();

            Ok(())
        }
    }

    fn flush_noscript_content(
        base_url: Rc<url::Url>,
        noscript_buf: NoScriptBuffer,
        style_hashes: StyleHashList,
    ) -> impl Fn(&mut EndTag<'_>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> + 'static
    {
        move |end| {
            let mut rewriter = HtmlRewrite::new(base_url.clone());

            rewriter.write(noscript_buf.take().as_bytes())?;

            let result = rewriter.end()?;
            let mut hash_list = style_hashes.borrow_mut();

            for hash in result.style_hashes {
                hash_list.push(hash);
            }

            end.after(
                String::from_utf8(result.html)?.as_str(),
                lol_html::html_content::ContentType::Html,
            );

            Ok(())
        }
    }

    fn write_noscript_content(
        noscript_buf: NoScriptBuffer,
    ) -> impl FnMut(&mut TextChunk<'_>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> + 'html
    {
        move |chunk| {
            noscript_buf.borrow_mut().push_str(chunk.as_str());

            Ok(())
        }
    }

    fn append_proxy_header(
        base_url: Rc<url::Url>,
    ) -> impl Fn(&mut Element<'_, '_>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> + 'html
    {
        move |element: &mut Element<'_, '_>| {
            element.prepend(
                crate::templates::render_template_string(crate::templates::Template::Header(
                    base_url.clone(),
                ))
                .as_str(),
                lol_html::html_content::ContentType::Html,
            );

            Ok(())
        }
    }

    fn append_proxy_styles(
        element: &mut Element<'_, '_>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        element.append(
            &HEADER_STYLE_ELEMENT,
            lol_html::html_content::ContentType::Html,
        );

        Ok(())
    }

    fn remove_disallowed_attributes(
        element: &mut Element<'_, '_>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if element.tag_name() == "meta" {
            let mut should_remove = false;

            for attr in element.attributes() {
                if !ALLOWED_META_ATTRIBUTES.contains(&attr.name().as_str()) {
                    should_remove = true;
                    break;
                }
            }

            if should_remove {
                element.remove()
            }

            return Ok(());
        }

        let mut remove_attributes = Vec::<String>::new();

        for attr in element.attributes() {
            let attr_name = attr.name();

            if !ALLOWED_ATTRIBUTES.contains(attr_name.as_str()) {
                remove_attributes.push(attr_name);
            }
        }

        for attr_name in remove_attributes {
            element.remove_attribute(&attr_name);
        }

        Ok(())
    }

    fn remove_element(
        element: &mut Element<'_, '_>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        element.remove();

        Ok(())
    }

    fn get_unchecked_attribute_value(element: &Element<'_, '_>, name: &str) -> String {
        Self::html_entity_decode(element.get_attribute(name).unwrap().as_str())
    }

    fn html_entity_decode(value: &str) -> String {
        let mut result = String::with_capacity(value.len());

        htmlentity::entity::decode_to(value, &mut result);

        result
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use crate::lib::rewrite_html::{HtmlRewrite, HEADER_STYLE_ELEMENT};

    #[test]
    fn rewrite_a_href_relative_n_1() {
        crate::lib::test_setup_hmac();

        let mut rewriter = HtmlRewrite::new(Rc::new(
            url::Url::parse("https://www.example.com/index.html").unwrap(),
        ));

        rewriter.write(b"<a href='/'>main</a>").unwrap();

        assert_eq!(
            std::str::from_utf8(rewriter.end().unwrap().html.as_slice()).unwrap(),
            "<a href=\"./?url=https%3A%2F%2Fwww.example.com%2F\
        &hash=85870232cac1676c4477f7cae4da7173ccee4002f32e89c16038547aa20175c0\">main</a>"
        );
    }

    #[test]
    fn rewrite_a_href_relative_html_entity_n_1() {
        crate::lib::test_setup_hmac();

        let mut rewriter = HtmlRewrite::new(Rc::new(
            url::Url::parse("https://www.example.com/index.html").unwrap(),
        ));

        rewriter
            .write(b"<a href='/?a=b&amp;c=d'>example</a>")
            .unwrap();

        assert_eq!(
            std::str::from_utf8(rewriter.end().unwrap().html.as_slice()).unwrap(),
            "<a href=\"./?url=https%3A%2F%2Fwww.example.com%2F%3Fa%3Db%26c%3Dd\
            &hash=b0162773021eaca309a695af2dfed9240bb7f2686ac6efe62fc802fa6e833889\">example</a>"
        );
    }

    #[test]
    fn rewrite_img_src_relative_n_1() {
        crate::lib::test_setup_hmac();

        let mut rewriter = HtmlRewrite::new(Rc::new(
            url::Url::parse("https://www.example.com/").unwrap(),
        ));

        rewriter.write(b"<img src='/logo.png'>").unwrap();

        assert_eq!(
            std::str::from_utf8(rewriter.end().unwrap().html.as_slice()).unwrap(),
            "<img src=\"./?url=https%3A%2F%2Fwww.example.com%2Flogo.png\
            &hash=2aa2717d139a63b3f3fc43fa862c8a73fc7814f1140b5279fc2758bc9d8cc1f9\" \
            decoding=\"async\">"
        );
    }

    #[test]
    fn rewrite_img_src_relative_html_entity_n_1() {
        crate::lib::test_setup_hmac();

        let mut rewriter = HtmlRewrite::new(Rc::new(
            url::Url::parse("https://www.example.com/").unwrap(),
        ));

        rewriter.write(b"<img src='/logo&comma;png'>").unwrap();

        assert_eq!(
            std::str::from_utf8(rewriter.end().unwrap().html.as_slice()).unwrap(),
            "<img src=\"./?url=https%3A%2F%2Fwww.example.com%2Flogo%2Cpng\
            &hash=09f005c2152e803e8f29b43dd6431773f97c2ec9412a484ac3f9dc7c1697c877\" \
            decoding=\"async\">"
        );
    }

    #[test]
    fn rewrite_iframe_src_relative_n_1() {
        crate::lib::test_setup_hmac();

        let mut rewriter = HtmlRewrite::new(Rc::new(
            url::Url::parse("https://www.example.com/").unwrap(),
        ));

        rewriter
            .write(b"<iframe src='/test.html'></iframe>")
            .unwrap();

        assert_eq!(
            std::str::from_utf8(rewriter.end().unwrap().html.as_slice()).unwrap(),
            "<iframe src=\"./?url=https%3A%2F%2Fwww.example.com%2Ftest.html\
            &hash=48b7184730b6c78c9b4231f70560f92bdc09188ab27871d9489a372b3b47a9e1\"></iframe>"
        );
    }

    #[test]
    fn rewrite_iframe_src_relative_html_entity_n_1() {
        crate::lib::test_setup_hmac();

        let mut rewriter = HtmlRewrite::new(Rc::new(
            url::Url::parse("https://www.example.com/").unwrap(),
        ));

        rewriter
            .write(b"<iframe src='/test&comma;html'></iframe>")
            .unwrap();

        assert_eq!(
            std::str::from_utf8(rewriter.end().unwrap().html.as_slice()).unwrap(),
            "<iframe src=\"./?url=https%3A%2F%2Fwww.example.com%2Ftest%2Chtml\
            &hash=6e211a524206716e0d77aaae300aa5b697609ead62908172833c267a68d5a6ee\"></iframe>"
        );
    }

    #[test]
    fn rewrite_img_attributes_n_1() {
        crate::lib::test_setup_hmac();

        let mut rewriter = HtmlRewrite::new(Rc::new(
            url::Url::parse("https://www.example.com/").unwrap(),
        ));

        rewriter.write(b"<img class='image' onmouseover='javascript:console.log(this)' onerror='javascript:alert(\"failed\")'>").unwrap();

        assert_eq!(
            std::str::from_utf8(rewriter.end().unwrap().html.as_slice()).unwrap(),
            "<img class='image' decoding=\"async\">"
        );
    }

    #[test]
    fn rewrite_img_srcset_n_1() {
        crate::lib::test_setup_hmac();

        let mut rewriter = HtmlRewrite::new(Rc::new(
            url::Url::parse("https://www.example.com/").unwrap(),
        ));

        rewriter.write(b"<img srcset='header640.png 640w, header960.png 960w, header1024.png 1024w, header.png'>").unwrap();

        assert_eq!(
            std::str::from_utf8(rewriter.end().unwrap().html.as_slice()).unwrap(),
            "<img srcset=\"./?url=https%3A%2F%2Fwww.example.com%2Fheader640.png&hash=bf2aa9174435adfc3616a7bbb7f34e42cc7935e34feb23e0f6001b3acf2ceee0 640w, \
            ./?url=https%3A%2F%2Fwww.example.com%2Fheader960.png&hash=197fbfa4294a326f377651d2297f8ed5bf45018210e8615c7ee5dd7fad7037ec 960w, \
            ./?url=https%3A%2F%2Fwww.example.com%2Fheader1024.png&hash=d056d2f2316e7d9a1be4f34d7b430af80a610a87dc7616ae6d8d3d27cd84aef1 1024w, \
            ./?url=https%3A%2F%2Fwww.example.com%2Fheader.png&hash=890ee860e875afc9c56d972f1f44d64b55d93aeaf73a7f24e1cd43fc5806a414\" \
            decoding=\"async\">"
        );
    }

    #[test]
    fn rewrite_img_srcset_html_entity_n_1() {
        crate::lib::test_setup_hmac();

        let mut rewriter = HtmlRewrite::new(Rc::new(
            url::Url::parse("https://www.example.com/").unwrap(),
        ));

        rewriter
            .write(b"<img srcset='header640&comma;png 640w, header&amp;png'>")
            .unwrap();

        assert_eq!(
            std::str::from_utf8(rewriter.end().unwrap().html.as_slice()).unwrap(),
            "<img srcset=\"./?url=https%3A%2F%2Fwww.example.com%2Fheader640%2Cpng\
            &hash=1d8c952a54f1680f0735c2f7e5129e0d7c5e721e4e96375db8c939f432db0b92 640w, \
            ./?url=https%3A%2F%2Fwww.example.com%2Fheader%26png&hash=\
            da2a4431c30590e5fc1f0697677e62abe82327417228df6fe689af036c9828a5\" decoding=\"async\">"
        );
    }

    #[test]
    fn rewrite_iframe_attributes_n_1() {
        crate::lib::test_setup_hmac();

        let mut rewriter = HtmlRewrite::new(Rc::new(
            url::Url::parse("https://www.example.com/").unwrap(),
        ));

        rewriter
            .write(b"<iframe height='1' width='1' onclick='javascript:alert(1)'></iframe>")
            .unwrap();

        assert_eq!(
            std::str::from_utf8(rewriter.end().unwrap().html.as_slice()).unwrap(),
            "<iframe height='1' width='1'></iframe>"
        );
    }

    #[test]
    fn remove_applet_n_1() {
        crate::lib::test_setup_hmac();

        let mut rewriter = HtmlRewrite::new(Rc::new(
            url::Url::parse("https://www.example.com/index.html").unwrap(),
        ));

        rewriter.write(b"<applet />").unwrap();

        assert_eq!(
            std::str::from_utf8(rewriter.end().unwrap().html.as_slice()).unwrap(),
            ""
        );
    }

    #[test]
    fn remove_canvas_n_1() {
        crate::lib::test_setup_hmac();

        let mut rewriter = HtmlRewrite::new(Rc::new(
            url::Url::parse("https://www.example.com/index.html").unwrap(),
        ));

        rewriter.write(b"<canvas />").unwrap();

        assert_eq!(
            std::str::from_utf8(rewriter.end().unwrap().html.as_slice()).unwrap(),
            ""
        );
    }

    #[test]
    fn remove_embed_n_1() {
        crate::lib::test_setup_hmac();

        let mut rewriter = HtmlRewrite::new(Rc::new(
            url::Url::parse("https://www.example.com/index.html").unwrap(),
        ));

        rewriter.write(b"<embed />").unwrap();

        assert_eq!(
            std::str::from_utf8(rewriter.end().unwrap().html.as_slice()).unwrap(),
            ""
        );
    }

    #[test]
    fn remove_math_n_1() {
        crate::lib::test_setup_hmac();

        let mut rewriter = HtmlRewrite::new(Rc::new(
            url::Url::parse("https://www.example.com/index.html").unwrap(),
        ));

        rewriter.write(b"<math />").unwrap();

        assert_eq!(
            std::str::from_utf8(rewriter.end().unwrap().html.as_slice()).unwrap(),
            ""
        );
    }

    #[test]
    fn remove_script_n_1() {
        crate::lib::test_setup_hmac();

        let mut rewriter = HtmlRewrite::new(Rc::new(
            url::Url::parse("https://www.example.com/index.html").unwrap(),
        ));

        rewriter.write(b"<script />").unwrap();

        assert_eq!(
            std::str::from_utf8(rewriter.end().unwrap().html.as_slice()).unwrap(),
            ""
        );
    }

    #[test]
    fn remove_svg_n_1() {
        crate::lib::test_setup_hmac();

        let mut rewriter = HtmlRewrite::new(Rc::new(
            url::Url::parse("https://www.example.com/index.html").unwrap(),
        ));

        rewriter.write(b"<svg />").unwrap();

        assert_eq!(
            std::str::from_utf8(rewriter.end().unwrap().html.as_slice()).unwrap(),
            ""
        );
    }

    #[test]
    fn rewrite_body_n_1() {
        crate::lib::test_setup_hmac();

        let mut rewriter = HtmlRewrite::new(Rc::new(
            url::Url::parse("https://www.example.com/index.html").unwrap(),
        ));

        rewriter.write(b"<body><h1>Test</h1></body>").unwrap();

        assert_eq!(
            std::str::from_utf8(rewriter.end().unwrap().html.as_slice()).unwrap(),
            // this is pretty finicky... (and will break if the "header.html" formatting changes)
            "<body><div><h1><a href=\"./\" target=\"_self\" rel=\"noreferrer noopener\">SearProxy</a> is neither the owner nor the author of this content.</h1>\
            <p>Scripts are deactivated. Web page appearance may have changed. Visit the <a href=\"https://www.example.com/index.html\" target=\"_self\" rel=\"noreferrer noopener\">\
            original page</a>.</p></div><h1>Test</h1></body>"
        );
    }

    #[test]
    fn rewrite_head_n_1() {
        crate::lib::test_setup_hmac();

        let mut rewriter = HtmlRewrite::new(Rc::new(
            url::Url::parse("https://www.example.com/index.html").unwrap(),
        ));

        rewriter.write(b"<head><title>Test</title></head>").unwrap();

        assert_eq!(
            std::str::from_utf8(rewriter.end().unwrap().html.as_slice()).unwrap(),
            format!(
                "<head><title>Test</title>{}</head>",
                HEADER_STYLE_ELEMENT.as_str()
            )
        );
    }

    #[test]
    fn rewrite_style_plain_n_1() {
        crate::lib::test_setup_hmac();

        let mut rewriter = HtmlRewrite::new(Rc::new(
            url::Url::parse("https://www.example.com/").unwrap(),
        ));

        rewriter
            .write(b"<head><style>a{color:red}</style></head>")
            .unwrap();

        assert_eq!(
            std::str::from_utf8(rewriter.end().unwrap().html.as_slice()).unwrap(),
            format!(
                "<head><style>a{{color:red}}</style>{}</head>",
                HEADER_STYLE_ELEMENT.as_str()
            )
        );
    }

    #[test]
    fn rewrite_style_url_n_1() {
        crate::lib::test_setup_hmac();

        let mut rewriter = HtmlRewrite::new(Rc::new(
            url::Url::parse("https://www.example.com/").unwrap(),
        ));

        rewriter
            .write(b"<head><style>body{background-image:url('/main.css')}</style></head>")
            .unwrap();

        assert_eq!(
            std::str::from_utf8(rewriter.end().unwrap().html.as_slice()).unwrap(),
            format!("<head><style>\
            body{{background-image:url('./?url=https%3A%2F%2Fwww.example.com%2Fmain.css&hash=7d40cd69599262cfe009ac148491a37e9ec47dcf2386c2807bc2255fff6d5fa3')}}\
            </style>{}</head>",HEADER_STYLE_ELEMENT.as_str())
        );
    }

    #[test]
    fn rewrite_style_url_n_3() {
        crate::lib::test_setup_hmac();

        let mut rewriter = HtmlRewrite::new(Rc::new(
            url::Url::parse("https://www.example.com/").unwrap(),
        ));

        rewriter
            .write(b"<head><style>url('/main.css')</style><style>url('/index.css')</style><style>url('/theme.css')</style></head>")
            .unwrap();

        assert_eq!(
            std::str::from_utf8(rewriter.end().unwrap().html.as_slice()).unwrap(),
            format!("<head><style>url('./?url=https%3A%2F%2Fwww.example.com%2Fmain.css&hash=7d40cd69599262cfe009ac148491a37e9ec47dcf2386c2807bc2255fff6d5fa3')</style>\
            <style>url('./?url=https%3A%2F%2Fwww.example.com%2Findex.css&hash=de26b17e7788f85987457601375a920242dee16379bd17769fe6b6fbcb90cfcf')</style>\
            <style>url('./?url=https%3A%2F%2Fwww.example.com%2Ftheme.css&hash=ddc8ae45cdbef1f3ddfc778ba578b36666f3b2541de07d5efbc1a2584a3e913c')</style>\
            {}</head>",HEADER_STYLE_ELEMENT.as_str())
        );
    }

    #[test]
    fn rewrite_link_icon_n_1() {
        crate::lib::test_setup_hmac();

        let mut rewriter = HtmlRewrite::new(Rc::new(
            url::Url::parse("https://www.example.com/").unwrap(),
        ));

        rewriter
            .write(b"<link rel=\"icon\" href=\"favicon.ico\">")
            .unwrap();

        assert_eq!(
            std::str::from_utf8(rewriter.end().unwrap().html.as_slice()).unwrap(),
            "<link rel=\"icon\" href=\"./?url=https%3A%2F%2Fwww.example.com%2Ffavicon.ico&\
            hash=fc10bed0a5b7786553e4f658be6029176875e29fe645f32251c0b7427b4f057d\">"
        );
    }

    #[test]
    fn rewrite_link_icon_html_entity_n_1() {
        crate::lib::test_setup_hmac();

        let mut rewriter = HtmlRewrite::new(Rc::new(
            url::Url::parse("https://www.example.com/").unwrap(),
        ));

        rewriter
            .write(b"<link rel=\"icon\" href=\"favicon&comma;ico\">")
            .unwrap();

        assert_eq!(
            std::str::from_utf8(rewriter.end().unwrap().html.as_slice()).unwrap(),
            "<link rel=\"icon\" href=\"./?url=https%3A%2F%2Fwww.example.com%2Ffavicon%2Cico&\
            hash=7d143d57f49c3f4d096dbd79f17530a1fa2783373db3fb0dadf19bccdd4b4dd2\">"
        );
    }

    #[test]
    fn rewrite_link_shortcut_icon_n_1() {
        crate::lib::test_setup_hmac();

        let mut rewriter = HtmlRewrite::new(Rc::new(
            url::Url::parse("https://www.example.com/").unwrap(),
        ));

        rewriter
            .write(b"<link rel=\"shortcut icon\" href=\"favicon.ico\">")
            .unwrap();

        assert_eq!(
            std::str::from_utf8(rewriter.end().unwrap().html.as_slice()).unwrap(),
            "<link rel=\"shortcut icon\" href=\"./?url=https%3A%2F%2Fwww.example.com%2Ffavicon.ico&hash=fc10bed0a5b7786553e4f658be6029176875e29fe645f32251c0b7427b4f057d\">"
        );
    }

    #[test]
    fn rewrite_link_stylesheet_n_1() {
        crate::lib::test_setup_hmac();

        let mut rewriter = HtmlRewrite::new(Rc::new(
            url::Url::parse("https://www.example.com/").unwrap(),
        ));

        rewriter
            .write(b"<link href=\"default.css\" rel=\"stylesheet\" type=\"text/css\">")
            .unwrap();

        assert_eq!(
            std::str::from_utf8(rewriter.end().unwrap().html.as_slice()).unwrap(),
            "<link href=\"./?url=https%3A%2F%2Fwww.example.com%2Fdefault.css&hash=766b24ce591a42a33d5946c2c7382586c8f2ab501b40f5e154416298feb2565f\" rel=\"stylesheet\" type=\"text/css\">"
        );
    }

    #[test]
    fn rewrite_link_alternate_stylesheet_n_1() {
        crate::lib::test_setup_hmac();

        let mut rewriter = HtmlRewrite::new(Rc::new(
            url::Url::parse("https://www.example.com/").unwrap(),
        ));

        rewriter
            .write(b"<link href=\"basic.css\" rel=\"alternate stylesheet\" type=\"text/css\">")
            .unwrap();

        assert_eq!(
            std::str::from_utf8(rewriter.end().unwrap().html.as_slice()).unwrap(),
            "<link href=\"./?url=https%3A%2F%2Fwww.example.com%2Fbasic.css&hash=d5e4d42ff654522f6560ce8f8e689aab36c2df2bed12109b6d90b506a519d785\" rel=\"alternate stylesheet\" type=\"text/css\">"
        );
    }

    #[test]
    fn rewrite_link_help_n_1() {
        crate::lib::test_setup_hmac();

        let mut rewriter = HtmlRewrite::new(Rc::new(
            url::Url::parse("https://www.example.com/").unwrap(),
        ));

        rewriter.write(b"<link href=\"/a\" rel=\"help\">").unwrap();

        assert_eq!(
            std::str::from_utf8(rewriter.end().unwrap().html.as_slice()).unwrap(),
            "<link href=\"./?url=https%3A%2F%2Fwww.example.com%2Fa&hash=d2269853e1eda4c3f07592ef3742218dfa63c210d29f0fe3ea16f460efa164e8\" rel=\"help\">"
        );
    }

    #[test]
    fn rewrite_link_license_n_1() {
        crate::lib::test_setup_hmac();

        let mut rewriter = HtmlRewrite::new(Rc::new(
            url::Url::parse("https://www.example.com/").unwrap(),
        ));

        rewriter
            .write(b"<link href=\"/a\" rel=\"license\">")
            .unwrap();

        assert_eq!(
            std::str::from_utf8(rewriter.end().unwrap().html.as_slice()).unwrap(),
            "<link href=\"./?url=https%3A%2F%2Fwww.example.com%2Fa&hash=d2269853e1eda4c3f07592ef3742218dfa63c210d29f0fe3ea16f460efa164e8\" rel=\"license\">"
        );
    }

    #[test]
    fn rewrite_link_alternate_n_1() {
        crate::lib::test_setup_hmac();

        let mut rewriter = HtmlRewrite::new(Rc::new(
            url::Url::parse("https://www.example.com/").unwrap(),
        ));

        rewriter
            .write(b"<link href=\"/rss\" rel=\"alternate\" type=\"application/rss+xml\">")
            .unwrap();

        assert_eq!(
            std::str::from_utf8(rewriter.end().unwrap().html.as_slice()).unwrap(),
            "<link href=\"./?url=https%3A%2F%2Fwww.example.com%2Frss&hash=39c396215376bfb80ae7cfca44b10d145d593d8e326fc2138841bf03cddd042a\" rel=\"alternate\" type=\"application/rss+xml\">"
        );
    }

    #[test]
    fn rewrite_meta_content_type_n_1() {
        crate::lib::test_setup_hmac();

        let mut rewriter = HtmlRewrite::new(Rc::new(
            url::Url::parse("https://www.example.com/").unwrap(),
        ));

        rewriter
            .write(b"<meta http-equiv=\"content-type\" content=\"text/html; charset=utf-8\">")
            .unwrap();

        assert_eq!(
            std::str::from_utf8(rewriter.end().unwrap().html.as_slice()).unwrap(),
            "<meta http-equiv=\"content-type\" content=\"text/html; charset=utf-8\">"
        );
    }

    #[test]
    fn rewrite_meta_ua_compatible_n_1() {
        crate::lib::test_setup_hmac();

        let mut rewriter = HtmlRewrite::new(Rc::new(
            url::Url::parse("https://www.example.com/").unwrap(),
        ));

        rewriter
            .write(b"<meta http-equiv=\"x-ua-compatible\" content=\"IE=edge\">")
            .unwrap();

        assert_eq!(
            std::str::from_utf8(rewriter.end().unwrap().html.as_slice()).unwrap(),
            "<meta http-equiv=\"x-ua-compatible\" content=\"IE=edge\">"
        );
    }

    #[test]
    fn rewrite_meta_refresh_n_1() {
        crate::lib::test_setup_hmac();

        let mut rewriter = HtmlRewrite::new(Rc::new(
            url::Url::parse("https://www.example.com/").unwrap(),
        ));

        rewriter
            .write(b"<meta http-equiv=\"refresh\" content=\"1;url=/a\">")
            .unwrap();

        assert_eq!(
            std::str::from_utf8(rewriter.end().unwrap().html.as_slice()).unwrap(),
            "<meta http-equiv=\"refresh\" content=\"1;url=./?url=https%3A%2F%2Fwww.example.com%2Fa&hash=d2269853e1eda4c3f07592ef3742218dfa63c210d29f0fe3ea16f460efa164e8\">"
        );
    }

    #[test]
    fn rewrite_form_method_get_n_1() {
        crate::lib::test_setup_hmac();

        let mut rewriter = HtmlRewrite::new(Rc::new(
            url::Url::parse("https://www.example.com/").unwrap(),
        ));

        rewriter
            .write(b"<form method=\"get\" action=\"/a\"></form>")
            .unwrap();

        assert_eq!(
            std::str::from_utf8(rewriter.end().unwrap().html.as_slice()).unwrap(),
            "<form method=\"POST\" action=\"./?url=https%3A%2F%2Fwww.example.com%2Fa&hash=d2269853e1eda4c3f07592ef3742218dfa63c210d29f0fe3ea16f460efa164e8\" target=\"_self\">\
            <input type=\"hidden\" name=\"_searproxy_origin_method\" value=\"get\"></form>"
        );
    }

    #[test]
    fn rewrite_form_method_post_n_1() {
        crate::lib::test_setup_hmac();

        let mut rewriter = HtmlRewrite::new(Rc::new(
            url::Url::parse("https://www.example.com/").unwrap(),
        ));

        rewriter
            .write(b"<form method=\"Post\" action=\"/a\"></form>")
            .unwrap();

        assert_eq!(
            std::str::from_utf8(rewriter.end().unwrap().html.as_slice()).unwrap(),
            "<form method=\"POST\" action=\"./?url=https%3A%2F%2Fwww.example.com%2Fa&hash=d2269853e1eda4c3f07592ef3742218dfa63c210d29f0fe3ea16f460efa164e8\" target=\"_self\">\
            <input type=\"hidden\" name=\"_searproxy_origin_method\" value=\"Post\"></form>"
        );
    }

    #[test]
    fn rewrite_form_no_method_n_1() {
        crate::lib::test_setup_hmac();

        let mut rewriter = HtmlRewrite::new(Rc::new(
            url::Url::parse("https://www.example.com/").unwrap(),
        ));

        rewriter.write(b"<form action=\"/a\"></form>").unwrap();

        assert_eq!(
            std::str::from_utf8(rewriter.end().unwrap().html.as_slice()).unwrap(),
            "<form action=\"./?url=https%3A%2F%2Fwww.example.com%2Fa&hash=d2269853e1eda4c3f07592ef3742218dfa63c210d29f0fe3ea16f460efa164e8\" target=\"_self\" method=\"POST\"></form>"
        );
    }

    #[test]
    fn rewrite_form_no_action_n_1() {
        crate::lib::test_setup_hmac();

        let mut rewriter = HtmlRewrite::new(Rc::new(
            url::Url::parse("https://www.example.com/").unwrap(),
        ));

        rewriter.write(b"<form></form>").unwrap();

        assert_eq!(
            std::str::from_utf8(rewriter.end().unwrap().html.as_slice()).unwrap(),
            "<form target=\"_self\" method=\"POST\"></form>"
        );
    }

    #[test]
    fn rewrite_valid_width_img_srcset_n_1() {
        crate::lib::test_setup_hmac();

        let mut rewriter = HtmlRewrite::new(Rc::new(
            url::Url::parse("https://www.example.com/").unwrap(),
        ));

        rewriter
            .write(b"<img srcset=\"https://ex.amp.le 1w\">")
            .unwrap();

        assert_eq!(
            std::str::from_utf8(rewriter.end().unwrap().html.as_slice()).unwrap(),
            "<img srcset=\"./?url=https%3A%2F%2Fex.amp.le%2F&hash=35e41e2ec2517a437522f9c921536eb6650c63fd8e9e34d8c5a001494c17481b 1w\" decoding=\"async\">"
        );
    }

    #[test]
    fn rewrite_valid_width_source_srcset_n_1() {
        crate::lib::test_setup_hmac();

        let mut rewriter = HtmlRewrite::new(Rc::new(
            url::Url::parse("https://www.example.com/").unwrap(),
        ));

        rewriter
            .write(b"<source srcset=\"https://ex.amp.le 1w\">")
            .unwrap();

        assert_eq!(
            std::str::from_utf8(rewriter.end().unwrap().html.as_slice()).unwrap(),
            "<source srcset=\"./?url=https%3A%2F%2Fex.amp.le%2F&hash=35e41e2ec2517a437522f9c921536eb6650c63fd8e9e34d8c5a001494c17481b 1w\">"
        );
    }

    #[test]
    fn rewrite_valid_density_img_srcset_n_1() {
        crate::lib::test_setup_hmac();

        let mut rewriter = HtmlRewrite::new(Rc::new(
            url::Url::parse("https://www.example.com/").unwrap(),
        ));

        rewriter
            .write(b"<img srcset=\"https://ex.amp.le 1x\">")
            .unwrap();

        assert_eq!(
            std::str::from_utf8(rewriter.end().unwrap().html.as_slice()).unwrap(),
            "<img srcset=\"./?url=https%3A%2F%2Fex.amp.le%2F&hash=35e41e2ec2517a437522f9c921536eb6650c63fd8e9e34d8c5a001494c17481b 1x\" decoding=\"async\">"
        );
    }

    #[test]
    fn rewrite_valid_density_source_srcset_n_1() {
        crate::lib::test_setup_hmac();

        let mut rewriter = HtmlRewrite::new(Rc::new(
            url::Url::parse("https://www.example.com/").unwrap(),
        ));

        rewriter
            .write(b"<source srcset=\"https://ex.amp.le 1x\">")
            .unwrap();

        assert_eq!(
            std::str::from_utf8(rewriter.end().unwrap().html.as_slice()).unwrap(),
            "<source srcset=\"./?url=https%3A%2F%2Fex.amp.le%2F&hash=35e41e2ec2517a437522f9c921536eb6650c63fd8e9e34d8c5a001494c17481b 1x\">"
        );
    }

    #[test]
    fn rewrite_valid_data_source_srcset_n1() {
        crate::lib::test_setup_hmac();

        let mut rewriter = HtmlRewrite::new(Rc::new(
            url::Url::parse("https://www.example.com/").unwrap(),
        ));

        rewriter
            .write(b"<source srcset=\"data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7\">")
            .unwrap();

        assert_eq!(
            std::str::from_utf8(rewriter.end().unwrap().html.as_slice()).unwrap(),
            "<source srcset=\"data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7\">"
        );
    }

    #[test]
    fn rewrite_valid_density_data_source_srcset_n1() {
        crate::lib::test_setup_hmac();

        let mut rewriter = HtmlRewrite::new(Rc::new(
            url::Url::parse("https://www.example.com/").unwrap(),
        ));

        rewriter
            .write(b"<source srcset=\"data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7 1x\">")
            .unwrap();

        assert_eq!(
            std::str::from_utf8(rewriter.end().unwrap().html.as_slice()).unwrap(),
            "<source srcset=\"data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7 1x\">"
        );
    }

    #[test]
    fn rewrite_invalid_img_srcset_n_1() {
        crate::lib::test_setup_hmac();

        let mut rewriter = HtmlRewrite::new(Rc::new(
            url::Url::parse("https://www.example.com/").unwrap(),
        ));

        rewriter
            .write(b"<img srcset=\"https://ex.amp.le 1w 2h\">")
            .unwrap();

        assert_eq!(
            std::str::from_utf8(rewriter.end().unwrap().html.as_slice()).unwrap(),
            "<img srcset=\"./?url=https%3A%2F%2Fex.amp.le%2F&hash=35e41e2ec2517a437522f9c921536eb6650c63fd8e9e34d8c5a001494c17481b 1w 2h\" decoding=\"async\">"
        );
    }

    #[test]
    fn rewrite_invalid_source_srcset_n_1() {
        crate::lib::test_setup_hmac();

        let mut rewriter = HtmlRewrite::new(Rc::new(
            url::Url::parse("https://www.example.com/").unwrap(),
        ));

        rewriter
            .write(b"<source srcset=\"https://ex.amp.le 1w 2h\">")
            .unwrap();

        assert_eq!(
            std::str::from_utf8(rewriter.end().unwrap().html.as_slice()).unwrap(),
            "<source srcset=\"./?url=https%3A%2F%2Fex.amp.le%2F&hash=35e41e2ec2517a437522f9c921536eb6650c63fd8e9e34d8c5a001494c17481b 1w 2h\">"
        );
    }

    #[test]
    fn rewrite_noscript_n_1() {
        crate::lib::test_setup_hmac();

        let mut rewriter = HtmlRewrite::new(Rc::new(
            url::Url::parse("https://www.example.com/").unwrap(),
        ));

        rewriter
            .write(b"<noscript><h1>No JavaScript!</h1></noscript>")
            .unwrap();

        assert_eq!(
            std::str::from_utf8(rewriter.end().unwrap().html.as_slice()).unwrap(),
            "<h1>No JavaScript!</h1>"
        );
    }

    #[test]
    fn rewrite_noscript_n_3() {
        crate::lib::test_setup_hmac();

        let mut rewriter = HtmlRewrite::new(Rc::new(
            url::Url::parse("https://www.example.com/").unwrap(),
        ));

        rewriter
            .write(b"<noscript><h1>No</h1></noscript><h2>Yes</h2><noscript><h3>Maybe</h3></noscript><h4>Definitely</h4><noscript><h5>Enough</h5></noscript>")
            .unwrap();

        assert_eq!(
            std::str::from_utf8(rewriter.end().unwrap().html.as_slice()).unwrap(),
            "<h1>No</h1><h2>Yes</h2><h3>Maybe</h3><h4>Definitely</h4><h5>Enough</h5>"
        );
    }

    #[test]
    fn rewrite_noscript_style_n_3() {
        crate::lib::test_setup_hmac();

        let mut rewriter = HtmlRewrite::new(Rc::new(
            url::Url::parse("https://www.example.com/").unwrap(),
        ));

        rewriter
            .write(b"<noscript><style>a{opacity:1}</style></noscript><noscript><style>b{opacity:1}</style></noscript><noscript><style>c{opacity:1}</style></noscript>")
            .unwrap();

        let result = rewriter.end().unwrap();

        assert_eq!(
            std::str::from_utf8(result.html.as_slice()).unwrap(),
            "<style>a{opacity:1}</style><style>b{opacity:1}</style><style>c{opacity:1}</style>"
        );
        assert_eq!(result.style_hashes.len(), 3);
    }

    #[test]
    fn rewrite_head_noscript_n_1() {
        crate::lib::test_setup_hmac();

        let mut rewriter = HtmlRewrite::new(Rc::new(
            url::Url::parse("https://www.example.com/").unwrap(),
        ));

        rewriter
            .write(b"<html><head><noscript><style>img{opacity:1}</style></noscript></head></html>")
            .unwrap();

        let result = rewriter.end().unwrap();

        assert_eq!(
            std::str::from_utf8(result.html.as_slice()).unwrap(),
            format!(
                "<html><head><style>img{{opacity:1}}</style>{}</head></html>",
                HEADER_STYLE_ELEMENT.as_str()
            )
        );
        assert_eq!(result.style_hashes.len(), 1);
    }

    #[test]
    fn rewrite_body_noscript_n_1() {
        crate::lib::test_setup_hmac();

        let url = Rc::new(url::Url::parse("https://www.example.com/").unwrap());

        let mut rewriter = HtmlRewrite::new(url.clone());

        rewriter
            .write(b"<html><body><noscript><style>img{opacity:1}</style><a href=\"https://www.example.com/\">example</a></noscript></body></html>")
            .unwrap();

        let result = rewriter.end().unwrap();

        assert_eq!(
            std::str::from_utf8(result.html.as_slice()).unwrap(),
            format!(
                "<html><body>{}<style>img{{opacity:1}}</style><a href=\"./?url=https%3A%2F%2Fwww.example.com%2F&hash=85870232cac1676c4477f7cae4da7173ccee4002f32e89c16038547aa20175c0\">example</a></body></html>",
                crate::templates::render_template_string(crate::templates::Template::Header(url))
            )
        );
        assert_eq!(result.style_hashes.len(), 1);
    }
}
