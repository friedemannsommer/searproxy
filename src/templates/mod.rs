use tera::Tera;

pub enum Template {
    // Banner,
    Error,
    Index,
    // InfoBox,
}

macro_rules! include_template {
    ($name:literal, $tera:expr) => {{
        $tera
            .add_raw_template($name, include_str!($name))
            .expect(concat!("Template '", $name, "' couldn't be compiled"));
    }};
}

static TEMPLATES: once_cell::sync::Lazy<Tera> = once_cell::sync::Lazy::new(|| {
    let mut tera = Tera::default();

    include_template!("base.html", tera);
    include_template!("footer.html", tera);
    include_template!("index.html", tera);
    include_template!("error.html", tera);

    tera
});

pub fn render_minified(
    template: Template,
    context_opt: Option<tera::Context>,
) -> Result<bytes::Bytes, tera::Error> {
    let context = context_opt.unwrap_or_default();
    let html = match template {
        Template::Index => TEMPLATES.render("index.html", &context)?,
        Template::Error => TEMPLATES.render("error.html", &context)?,
    };

    Ok(bytes::Bytes::from(minify_html::minify(
        html.as_bytes(),
        &crate::lib::MINIFY_CONFIG,
    )))
}
