//! Fetches an image over HTTP and draws it, showing `imaging::load_image`
//! used from a theme.
//!   curl -X POST localhost:4444/generate -H 'content-type: application/json' \
//!     -d '{"theme":"image_from_url","data":{"url":"https://picsum.photos/500"}}'
use async_trait::async_trait;
use ditto::imaging;
use ditto::toolbox::draw_image_at;
use ditto::{GenerateData, Registry, ServeOptions, Template};
use serde::Deserialize;

#[derive(Deserialize)]
struct ImageFromUrlData {
    url: String,
}

struct ImageFromUrlTemplate;

#[async_trait]
impl Template for ImageFromUrlTemplate {
    fn width(&self) -> i32 {
        500
    }

    fn height(&self) -> i32 {
        500
    }

    async fn draw(&self, client: &reqwest::Client, meta: &GenerateData) -> anyhow::Result<ditto::Draw> {
        let data: ImageFromUrlData = serde_json::from_value(meta.data.clone())?;
        let image = imaging::load_image(client, &data.url, 500, 500, true).await?;
        Ok(Box::new(move |canvas| {
            draw_image_at(canvas, &image, 0.0, 0.0);
        }))
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let registry = Registry::new().register("image_from_url", ImageFromUrlTemplate);
    ditto::serve(registry, ServeOptions::from_env()).await
}
