mod base;
mod error;
mod header;
mod index;
mod redirect;

pub enum Template<'error_name, 'error_description> {
    Error(Option<crate::server::lib::ErrorMessage<'error_name, 'error_description>>),
    Header(std::rc::Rc<url::Url>),
    Index,
    Redirect(crate::utilities::ClientRedirect),
}

pub fn render_template_string(template: Template<'_, '_>) -> String {
    match template {
        Template::Error(error_detail) => error::error(&error_detail).to_string(),
        Template::Header(url) => header::Header { url }.to_string(),
        Template::Index => index::index().to_string(),
        Template::Redirect(url) => redirect::redirect(url).to_string(),
    }
}
