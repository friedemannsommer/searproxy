markup::define! {
    HeaderTemplate<'url>(url: &'url str) {
        div.__sp_header {
            h1 {
                "SearProxy"
            }
            p {
                "This is a proxified and sanitized version, visit "
                a["href" = url, "target" = "_self", "rel" = "noreferrer noopener"] {
                    "original page"
                }
                "."
            }
        }
    }
}

pub use HeaderTemplate as Header;
