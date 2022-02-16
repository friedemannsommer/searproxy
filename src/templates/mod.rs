use tera::Tera;

pub enum Template {
    Error,
    Header,
    Index,
}

macro_rules! include_template {
    ($name:literal, $tera:expr) => {{
        $tera
            .add_raw_template($name, include_str!($name))
            .expect(concat!("Template '", $name, "' couldn't be compiled"));
    }};
}

const SOURCE_CODE_URL: &str = "https://github.com/friedemannsommer/searproxy";
const ISSUES_URL: &str = "https://github.com/friedemannsommer/searproxy/issues";

static TEMPLATES: once_cell::sync::Lazy<Tera> = once_cell::sync::Lazy::new(|| {
    let mut tera = Tera::default();

    include_template!("base.html", tera);
    include_template!("footer.html", tera);
    include_template!("index.html", tera);
    include_template!("error.html", tera);
    include_template!("header.html", tera);

    tera
});

pub fn render_template(
    template: Template,
    context_opt: Option<tera::Context>,
) -> Result<bytes::Bytes, tera::Error> {
    Ok(bytes::Bytes::from(render_template_string(
        template,
        context_opt,
    )?))
}

pub fn render_template_string(
    template: Template,
    context_opt: Option<tera::Context>,
) -> Result<String, tera::Error> {
    let mut context = context_opt.unwrap_or_default();

    context.insert("source_code_url", SOURCE_CODE_URL);
    context.insert("issues_url", ISSUES_URL);

    Ok(match template {
        Template::Error => TEMPLATES.render("error.html", &context)?,
        Template::Header => TEMPLATES.render("header.html", &context)?,
        Template::Index => TEMPLATES.render("index.html", &context)?,
    })
}
