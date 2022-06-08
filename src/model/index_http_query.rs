#[derive(serde::Deserialize, serde::Serialize, Debug)]
pub struct IndexHttpArgs {
    #[serde(alias = "mortyurl")]
    pub url: Option<String>,
    #[serde(alias = "mortyhash")]
    pub hash: Option<String>,
}
