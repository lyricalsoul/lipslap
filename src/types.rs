use crate::overlay::OverlaySpec;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateData {
    #[serde(default)]
    pub id: Option<String>,
    pub theme: String,

    #[serde(default)]
    pub overlays: Vec<OverlaySpec>,
    pub data: serde_json::Value,
}
