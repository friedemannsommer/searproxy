use std::{cell::RefCell, collections::HashSet, rc::Rc};

use lol_html::html_content::Element;

use crate::lib::rewrite_url::rewrite_url;

pub struct HtmlRewrite<'url> {
    output: Rc<RefCell<Vec<u8>>>,
    rewriter: lol_html::HtmlRewriter<'url, Box<dyn FnMut(&[u8])>>,
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

impl<'url> HtmlRewrite<'url> {
    pub fn new(url: &'url url::Url) -> Self {
        let output = Rc::new(RefCell::new(Vec::<u8>::new()));

        Self {
            output: output.clone(),
            rewriter: lol_html::HtmlRewriter::new(
                lol_html::Settings {
                    element_content_handlers: vec![
                        lol_html::element!("*", Self::remove_disallowed_attributes),
                        lol_html::element!("*[href]", Self::transform_href(url)),
                        lol_html::element!("applet", Self::remove_element),
                        lol_html::element!("canvas", Self::remove_element),
                        lol_html::element!("embed", Self::remove_element),
                        lol_html::element!("iframe[src]", Self::transform_src(url)),
                        lol_html::element!("img[src]", Self::transform_src(url)),
                        lol_html::element!("img[srcset]", Self::transform_srcset(url)),
                        lol_html::element!("math", Self::remove_element),
                        lol_html::element!("script", Self::remove_element),
                        lol_html::element!("svg", Self::remove_element),
                    ],
                    ..lol_html::Settings::default()
                },
                Box::new(move |chunk: &[u8]| {
                    output.borrow_mut().extend_from_slice(chunk);
                }),
            ),
        }
    }

    pub fn write(&mut self, data: &[u8]) -> Result<(), lol_html::errors::RewritingError> {
        self.rewriter.write(data)
    }

    pub fn end(self) -> Result<Rc<RefCell<Vec<u8>>>, lol_html::errors::RewritingError> {
        self.rewriter.end()?;

        Ok(self.output)
    }

    fn transform_src(
        base_url: &'_ url::Url,
    ) -> impl Fn(&mut Element) -> Result<(), Box<dyn std::error::Error + Send + Sync>> + '_ {
        |element| {
            let _span = tracing::span!(
                tracing::Level::TRACE,
                "transform_src",
                http.url = base_url.as_str()
            );

            element.set_attribute(
                "src",
                &rewrite_url(base_url, &element.get_attribute("src").unwrap())?,
            )?;

            Ok(())
        }
    }

    fn transform_srcset(
        base_url: &'_ url::Url,
    ) -> impl Fn(&mut Element) -> Result<(), Box<dyn std::error::Error + Send + Sync>> + '_ {
        |element| {
            let _span = tracing::span!(
                tracing::Level::TRACE,
                "transform_srcset",
                http.url = base_url.as_str()
            );
            let src_set_values = element.get_attribute("srcset").unwrap();
            let mut output = String::with_capacity(src_set_values.len());
            let mut offset = 0;

            for group in IMG_SRCSET_REGEX.captures_iter(&src_set_values) {
                if let Some(matched_url) = group.name("url") {
                    let proxied_url = rewrite_url(base_url, matched_url.as_str())?;

                    output.push_str(&src_set_values[offset..matched_url.start()]);
                    output.push_str(&proxied_url);
                    offset = matched_url.end();
                }
            }

            output.push_str(&src_set_values[offset..]);
            element.set_attribute("srcset", &output)?;

            Ok(())
        }
    }

    fn transform_href(
        base_url: &'_ url::Url,
    ) -> impl Fn(&mut Element) -> Result<(), Box<dyn std::error::Error + Send + Sync>> + '_ {
        |element: &mut Element| {
            let _span = tracing::span!(
                tracing::Level::TRACE,
                "transform_href",
                http.url = base_url.as_str()
            );

            element.set_attribute(
                "href",
                &rewrite_url(base_url, &element.get_attribute("href").unwrap())?,
            )?;

            Ok(())
        }
    }

    fn remove_disallowed_attributes(
        element: &mut Element,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let _span = tracing::span!(tracing::Level::TRACE, "remove_disallowed_attributes");
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
        let _span = tracing::span!(tracing::Level::TRACE, "remove_element");

        element.remove();

        Ok(())
    }
}
