use askama::Template;

#[allow(dead_code)]
#[derive(Template)]
#[template(path = "index.html")]
pub struct Index<'a> {
    pub(super) title: &'a str,
    pub(super) description: &'a str,
    pub(super) voices: Vec<String>,
    pub(super) api_base: String
}

#[allow(dead_code)]
#[derive(Template)]
#[template(path = "demo.html")]
pub struct Demo<'a> {
    pub(super) title: &'a str,
    pub(super) description: &'a str,
    pub(super) voices: Vec<String>
}
