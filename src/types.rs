use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateData {
    #[serde(default)]
    pub id: Option<String>,
    pub theme: String,
    
    #[serde(default)]
    pub overlays: Vec<String>,
    pub data: serde_json::Value,
}
