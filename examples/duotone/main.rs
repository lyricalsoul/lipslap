//! Fetches an image and applies a duotone effect via
//! `toolbox::draw_duotone_image` (see `toolbox.rs` for how it uses skia's
//! color-filter pipeline instead of a per-pixel loop).
//!   curl -X POST localhost:4444/generate -H 'content-type: application/json' \
//!     -d '{"theme":"duotone","data":{"url":"https://picsum.photos/500"}}'
use async_trait::async_trait;
use ditto::imaging;
use ditto::toolbox::draw_duotone_image;
use ditto::{GenerateData, Registry, ServeOptions, Template};
use serde::Deserialize;

#[derive(Deserialize)]
struct DuotoneData {
    url: String,
    #[serde(default = "default_colors")]
    colors: Vec<String>,
}

fn default_colors() -> Vec<String> {
    vec!["#1e1b4b".to_string(), "#f472b6".to_string()]
}

struct DuotoneTemplate;

#[async_trait]
impl Template for DuotoneTemplate {
    fn width(&self) -> i32 {
        500
    }

    fn height(&self) -> i32 {
        500
    }

    async fn draw(&self, client: &reqwest::Client, meta: &GenerateData) -> anyhow::Result<ditto::Draw> {
        let data: DuotoneData = serde_json::from_value(meta.data.clone())?;
        let image = imaging::load_image(client, &data.url, 500, 500, true).await?;
        Ok(Box::new(move |canvas| {
            let colors: Vec<&str> = data.colors.iter().map(String::as_str).collect();
            draw_duotone_image(canvas, &image, 0.0, 0.0, &colors);
        }))
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let registry = Registry::new().register("duotone", DuotoneTemplate);
    ditto::serve(registry, ServeOptions::from_env()).await
}
