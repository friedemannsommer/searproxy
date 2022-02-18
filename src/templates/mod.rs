mod base;
mod error;
mod header;
mod index;

pub enum Template<'error_name, 'error_description, 'url> {
    Error(Option<crate::server::lib::ErrorMessage<'error_name, 'error_description>>),
    Header(&'url str),
    Index,
}

pub fn render_template(template: Template) -> bytes::Bytes {
    bytes::Bytes::from(render_template_string(template))
}

pub fn render_template_string(template: Template) -> String {
    match template {
        Template::Error(error_detail) => error::error(&error_detail).to_string(),
        Template::Header(url) => header::Header { url }.to_string(),
        Template::Index => index::index().to_string(),
    }
}
