use crate::templates::base::Base;

pub fn index(
) -> Base<impl std::fmt::Display + markup::Render, impl std::fmt::Display + markup::Render> {
    Base {
        header: markup::new! {
            h2 {
                "This is a SearX & SearXNG compatible web proxy which excludes potentially malicious HTML tags. It also rewrites links
        to external resources to prevent leaks."
            }
        },
        content: markup::new! {
            h3 {
                b {
                    "WARNING"
                }
                " direct URL opening is not supported."
            }
        },
    }
}
