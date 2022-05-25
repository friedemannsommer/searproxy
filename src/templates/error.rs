use crate::templates::base::{blank_ref, Base};

pub fn error<'error_detail>(
    error_detail_opt: &'error_detail Option<crate::server::lib::ErrorMessage<'_, '_>>,
) -> Base<
    impl std::fmt::Display + markup::Render,
    impl std::fmt::Display + markup::Render + 'error_detail,
> {
    Base {
        header: markup::new! {
            h2 { "Request failed" }
        },
        content: markup::new! {
            @ if let Some(error_detail) = &error_detail_opt {
                h3 {
                    "Reason: "
                    @error_detail.name.as_ref()
                }
                p {
                    @error_detail.description.as_ref()
                }
            } else {
                h3 {
                    "While trying to process the request, an unexpected error occurred."
                }
                p {
                    "Consider "
                    @blank_ref("https://github.com/friedemannsommer/searproxy/issues", "opening an issue")
                    "."
                }
            }
        },
    }
}
