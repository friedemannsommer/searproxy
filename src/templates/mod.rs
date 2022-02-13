use tera::Tera;

pub enum Template {
    // Banner,
    Error,
    Index,
    // InfoBox,
}

static BASE_HTML: &str = include_str!("base.html");
static INDEX_HTML: &str = include_str!("index.html");
static ERROR_HTML: &str = include_str!("error.html");
static TEMPLATES: once_cell::sync::Lazy<Tera> = once_cell::sync::Lazy::new(|| {
    let mut tera = Tera::default();

    tera.add_raw_template("base.html", BASE_HTML)
        .expect("Template 'base.html' couldn't be compiled");
    tera.add_raw_template("index.html", INDEX_HTML)
        .expect("Template 'index.html' couldn't be compiled");
    tera.add_raw_template("error.html", ERROR_HTML)
        .expect("Template 'error.html' couldn't be compiled");

    tera
});

pub fn render_minified(template: Template) -> Result<bytes::Bytes, tera::Error> {
    let html = match template {
        Template::Index => TEMPLATES.render("index.html", &tera::Context::default())?,
        Template::Error => TEMPLATES.render("error.html", &tera::Context::default())?,
    };

    Ok(bytes::Bytes::from(minify_html::minify(
        html.as_bytes(),
        &crate::lib::MINIFY_CONFIG,
    )))
}
