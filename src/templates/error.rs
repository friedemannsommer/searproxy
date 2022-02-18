use crate::templates::base::Base;

pub fn error<'error_detail>(
    error_detail_opt: &'error_detail std::option::Option<crate::server::lib::ErrorMessage<'_, '_>>,
) -> Base<
    impl std::fmt::Display + markup::Render,
    impl std::fmt::Display + markup::Render + 'error_detail,
> {
    Base {
        header: markup::new! {
            h2 {
                "Request failed."
            }
        },
        content: markup::new! {
            @ if let Some(error_detail) = &error_detail_opt {
                h3 {
                    @ error_detail.name.as_ref()
                }
                p {
                    @ error_detail.description.as_ref()
                }
            } else {
                h3 {
                    "While trying to process the request, an unexpected error occurred."
                }
                p {
                    "Consider "
                    a["href" = "https://github.com/friedemannsommer/searproxy/issues", "target" = "_blank", "rel" = "noopener noreferrer"] {
                        "opening an issue"
                    }
                    "."
                }
            }
        },
    }
}
