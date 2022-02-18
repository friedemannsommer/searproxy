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
                link["rel" = "stylesheet", "href" = "./main.css"];
            }
            body {
                header {
                    h1 {
                        "SearProxy"
                    }
                    @header
                }
                main {
                    @content
                }
                footer {
                    "The "
                    a["href" = "https://github.com/friedemannsommer/searproxy", "target" = "_blank", "rel" = "noopener noreferrer"] {
                        "source code"
                    }
                    " is publicly available."
                }
            }
        }
    }
}

pub use Layout as Base;
