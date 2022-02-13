#[derive(serde::Deserialize, serde::Serialize, Debug)]
pub struct HttpArgs {
    #[serde(rename = "mortyurl")]
    pub url: Option<String>,
    #[serde(rename = "mortyhash")]
    pub hash: Option<String>,
}
