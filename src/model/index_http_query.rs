#[derive(serde::Deserialize, serde::Serialize, Debug)]
pub struct IndexHttpArgs {
    #[serde(rename = "mortyurl")]
    pub url: Option<String>,
    #[serde(rename = "mortyhash")]
    pub hash: Option<String>,
}
