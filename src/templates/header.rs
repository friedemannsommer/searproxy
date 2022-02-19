markup::define! {
    HeaderTemplate(url: std::rc::Rc<url::Url>) {
        div {
            h1 {
                @SelfRef { url: "./", content: "SearProxy" }
            }
            p {
                "This is a proxified and sanitized version, visit "
                @SelfRef { url: url.as_str(), content: "original page" }
                "."
            }
        }
    }

    SelfRef<'url, Content: markup::Render>(url: &'url str, content: Content) {
        a["href" = url, "target" = "_self", "rel" = "noreferrer noopener"] {
            @content
        }
    }
}

pub use HeaderTemplate as Header;
