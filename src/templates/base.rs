pub use ExternalLinkTemplate as ExternalLink;
pub use Layout as Base;

use crate::assets::MAIN_STYLESHEET;

markup::define! {
    Layout<Header: markup::Render, Content: markup::Render>(
        header: Header,
        content: Content
    ) {
        @markup::doctype()
        html["lang" = "en"] {
            head {
                meta["charset" = "UTF-8"];
                meta["name" = "color-scheme", "content" = "dark light"];
                meta["name" = "viewport", "content" = "width=device-width, initial-scale=1 , maximum-scale=1.0, user-scalable=1"];
                title { "SearProxy" }
                link["rel" = "icon", "type" = "image/png", "sizes" = "32x32", "href" = "./favicon-32x32.png"];
                link["rel" = "icon", "type" = "image/png", "sizes" = "16x16", "href" = "./favicon-16x16.png"];
                link["rel" = "icon", "type" = "image/ico", "sizes" = "16x16", "href" = "./favicon.ico"];
                style { @markup::raw(MAIN_STYLESHEET) }
            }
            body {
                header {
                    h1 {
                        "SearProxy"
                    }
                    @header
                }
                main { @content }
                footer {
                    p {
                        @blank_ref("https://github.com/friedemannsommer/searproxy", "Source code")
                        " | "
                        @blank_ref("https://friedemannsommer.github.io/searproxy/licenses.html", "Open source licenses")
                    }
                    p {
                        "This product includes software developed by the OpenSSL Project for use in the OpenSSL Toolkit. ("
                        @blank_ref("https://www.openssl.org/", "www.openssl.org")
                        ")"
                    }
                }
            }
        }
    }

    ExternalLinkTemplate<'url, Content: markup::Render>(
        url: &'url str,
        content: Content
    ) {
        a["href" = url, "target" = "_blank", "rel" = "noopener noreferrer"] {
            @content
        }
    }
}

#[inline]
pub fn blank_ref<Content: markup::Render>(
    url: &str,
    content: Content,
) -> ExternalLinkTemplate<Content> {
    ExternalLinkTemplate { url, content }
}
