use std::{cell::RefCell, collections::HashSet, rc::Rc};

use lol_html::html_content::{Element, EndTag, TextChunk};

use crate::lib::rewrite_css::CssRewrite;
use crate::lib::rewrite_url::rewrite_url;

type CssRewriteRef = Rc<RefCell<Option<CssRewrite>>>;
type StyleHashList = Rc<RefCell<Vec<String>>>;

pub struct HtmlRewrite<'html> {
    output: Rc<RefCell<Vec<u8>>>,
    rewriter: lol_html::HtmlRewriter<'html, Box<dyn FnMut(&[u8])>>,
    style_hashes: StyleHashList,
}

pub struct HtmlRewriteResult {
    pub html: Vec<u8>,
    pub style_hashes: Vec<String>,
}

static IMG_SRCSET_REGEX: once_cell::sync::Lazy<regex::Regex> = once_cell::sync::Lazy::new(|| {
    regex::Regex::new(r"(?P<url>[\w#!:.?+=&%@!\-/]+)(\s+(?:[0-9]+\.)?[0-9]+[xw]\s*[,$]?|$)")
        .expect("RegExp compilation failed")
});

static ALLOWED_ATTRIBUTES: once_cell::sync::Lazy<HashSet<&'static str>> =
    once_cell::sync::Lazy::new(|| {
        HashSet::from([
            "abbr",
            "accesskey",
            "align",
            "alt",
            "as",
            "autocomplete",
            "charset",
            "checked",
            "class",
            "content",
            "contenteditable",
            "contextmenu",
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

        Self {
            output: output.clone(),
            rewriter: lol_html::HtmlRewriter::new(
                lol_html::Settings {
                    element_content_handlers: vec![
                        lol_html::element!("applet", Self::remove_element),
                        lol_html::element!("canvas", Self::remove_element),
                        lol_html::element!("embed", Self::remove_element),
                        lol_html::element!("math", Self::remove_element),
                        lol_html::element!("script", Self::remove_element),
                        lol_html::element!("svg", Self::remove_element),
                        lol_html::element!("*", Self::remove_disallowed_attributes),
                        lol_html::element!("*[href]", Self::transform_href(url.clone())),
                        lol_html::element!("*[src]", Self::transform_src(url.clone())),
                        lol_html::element!("img[srcset]", Self::transform_srcset(url.clone())),
                        lol_html::element!(
                            "style",
                            Self::transform_style(
                                url.clone(),
                                css_rewriter.clone(),
                                style_hashes.clone()
                            )
                        ),
                        lol_html::text!("style", Self::write_style(css_rewriter)),
                        lol_html::element!("body", Self::append_proxy_header(url)),
                        lol_html::element!("head", Self::append_proxy_styles),
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
    ) -> impl Fn(&mut Element) -> Result<(), Box<dyn std::error::Error + Send + Sync>> + 'html {
        move |element| {
            element.set_attribute(
                "src",
                &rewrite_url(base_url.as_ref(), &element.get_attribute("src").unwrap())?,
            )?;

            Ok(())
        }
    }

    fn transform_srcset(
        base_url: Rc<url::Url>,
    ) -> impl Fn(&mut Element) -> Result<(), Box<dyn std::error::Error + Send + Sync>> + 'html {
        move |element| {
            let src_set_values = element.get_attribute("srcset").unwrap();
            let mut output = String::with_capacity(src_set_values.len());
            let mut offset = 0;

            for group in IMG_SRCSET_REGEX.captures_iter(&src_set_values) {
                if let Some(matched_url) = group.name("url") {
                    let proxy_url = rewrite_url(base_url.as_ref(), matched_url.as_str())?;

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
    ) -> impl Fn(&mut Element) -> Result<(), Box<dyn std::error::Error + Send + Sync>> + 'html {
        move |element: &mut Element| {
            element.set_attribute(
                "href",
                &rewrite_url(base_url.as_ref(), &element.get_attribute("href").unwrap())?,
            )?;

            Ok(())
        }
    }

    fn transform_style(
        base_url: Rc<url::Url>,
        css_rewriter: CssRewriteRef,
        style_hashes: StyleHashList,
    ) -> impl Fn(&mut Element) -> Result<(), Box<dyn std::error::Error + Send + Sync>> + 'html {
        move |element: &mut Element| {
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
    ) -> impl Fn(&mut EndTag) -> Result<(), Box<dyn std::error::Error + Send + Sync>> + 'static
    {
        move |end| {
            let current_css_rewriter = css_rewriter.replace(None);
            let css_bytes = current_css_rewriter.unwrap().end()?;

            style_hashes.borrow_mut().push(format!(
                "'sha256-{}'",
                base64::encode(hmac_sha256::Hash::hash(css_bytes.as_slice()))
            ));

            end.before(
                std::str::from_utf8(&css_bytes)?,
                lol_html::html_content::ContentType::Text,
            );

            Ok(())
        }
    }

    fn write_style(
        css_rewriter: CssRewriteRef,
    ) -> impl FnMut(&mut TextChunk) -> Result<(), Box<dyn std::error::Error + Send + Sync>> + 'html
    {
        move |text: &mut TextChunk| {
            css_rewriter
                .borrow_mut()
                .as_mut()
                .unwrap()
                .write(text.as_str().as_bytes())?;
            text.remove();
            Ok(())
        }
    }

    fn append_proxy_header(
        base_url: Rc<url::Url>,
    ) -> impl Fn(&mut Element) -> Result<(), Box<dyn std::error::Error + Send + Sync>> + 'html {
        move |element: &mut Element| {
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
        element: &mut Element,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        element.append(
            "<link rel=\"stylesheet\" href=\"./header.css\">",
            lol_html::html_content::ContentType::Html,
        );

        Ok(())
    }

    fn remove_disallowed_attributes(
        element: &mut Element,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
        element: &mut Element,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        element.remove();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use crate::lib::rewrite_html::HtmlRewrite;

    #[test]
    fn rewrite_a_href_relative_n_1() {
        crate::lib::test_setup_hmac();

        let mut rewriter = HtmlRewrite::new(Rc::new(
            url::Url::parse("https://www.example.com/index.html").unwrap(),
        ));

        rewriter.write(b"<a href='/'>main</a>").unwrap();

        assert_eq!(std::str::from_utf8( rewriter.end().unwrap().html.as_slice()).unwrap(), "<a href=\"./?mortyurl=https%3A%2F%2Fwww.example.com%2F&mortyhash=85870232cac1676c4477f7cae4da7173ccee4002f32e89c16038547aa20175c0\">main</a>");
    }

    #[test]
    fn rewrite_img_src_relative_n_1() {
        crate::lib::test_setup_hmac();

        let mut rewriter = HtmlRewrite::new(Rc::new(
            url::Url::parse("https://www.example.com/").unwrap(),
        ));

        rewriter.write(b"<img src='/logo.png'>").unwrap();

        assert_eq!(std::str::from_utf8( rewriter.end().unwrap().html.as_slice()).unwrap(), "<img src=\"./?mortyurl=https%3A%2F%2Fwww.example.com%2Flogo.png&mortyhash=2aa2717d139a63b3f3fc43fa862c8a73fc7814f1140b5279fc2758bc9d8cc1f9\">");
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

        assert_eq!(std::str::from_utf8( rewriter.end().unwrap().html.as_slice()).unwrap(), "<iframe src=\"./?mortyurl=https%3A%2F%2Fwww.example.com%2Ftest.html&mortyhash=48b7184730b6c78c9b4231f70560f92bdc09188ab27871d9489a372b3b47a9e1\"></iframe>");
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
            "<img class='image'>"
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
            "<img srcset=\"./?mortyurl=https%3A%2F%2Fwww.example.com%2Fheader640.png&mortyhash=bf2aa9174435adfc3616a7bbb7f34e42cc7935e34feb23e0f6001b3acf2ceee0 640w, ./?mortyurl=https%3A%2F%2Fwww.example.com%2Fheader960.png&mortyhash=197fbfa4294a326f377651d2297f8ed5bf45018210e8615c7ee5dd7fad7037ec 960w, ./?mortyurl=https%3A%2F%2Fwww.example.com%2Fheader1024.png&mortyhash=d056d2f2316e7d9a1be4f34d7b430af80a610a87dc7616ae6d8d3d27cd84aef1 1024w, ./?mortyurl=https%3A%2F%2Fwww.example.com%2Fheader.png&mortyhash=890ee860e875afc9c56d972f1f44d64b55d93aeaf73a7f24e1cd43fc5806a414\">"
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
            "<body><div class=\"__sp_header\"><h1><a href=\"./\" target=\"_self\" rel=\"noreferrer noopener\">SearProxy</a></h1><p>This is a proxified and sanitized version, visit <a href=\"https://www.example.com/index.html\" target=\"_self\" rel=\"noreferrer noopener\">original page</a>.</p></div><h1>Test</h1></body>"
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
            "<head><title>Test</title><link rel=\"stylesheet\" href=\"./header.css\"></head>"
        );
    }
}
