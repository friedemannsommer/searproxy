use crate::templates::base::{self_ref, Base};

pub fn redirect(
    client_redirect: crate::lib::ClientRedirect,
) -> Base<impl std::fmt::Display + markup::Render, impl std::fmt::Display + markup::Render> {
    let status_code = client_redirect.status_code;

    Base {
        header: markup::new! {
            h2 { "Server returned redirect" }
            p {
                "Status code:"
                @status_code.as_str()
            }
        },
        content: markup::new! {
            h3 { "If you want to follow the returned URL, click the link below:" }
            @self_ref(&client_redirect.internal_url, &client_redirect.external_url)
        },
    }
}
