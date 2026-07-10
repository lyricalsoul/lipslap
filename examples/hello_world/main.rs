//! The simplest possible ditto server: one theme, no input data.
//!   curl -X POST localhost:4444/generate -H 'content-type: application/json' \
//!     -d '{"theme":"hello_world","data":{}}'
use async_trait::async_trait;
use ditto::toolbox::{color, paint};
use ditto::{GenerateData, Registry, ServeOptions, Template};

struct HelloWorldTemplate;

#[async_trait]
impl Template for HelloWorldTemplate {
    fn width(&self) -> i32 {
        800
    }

    fn height(&self) -> i32 {
        400
    }

    async fn draw(&self, _client: &reqwest::Client, _meta: &GenerateData) -> anyhow::Result<ditto::Draw> {
        Ok(Box::new(move |canvas| {
            canvas.draw_color(color("#1e1b4b"), skia_safe::BlendMode::Src);
            let font = ditto::fonts::fallback(48.0);
            canvas.draw_str("Hello, world!", (60.0, 220.0), &font, &paint(color("#f5f5f5")));
        }))
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let registry = Registry::new().register("hello_world", HelloWorldTemplate);
    ditto::serve(registry, ServeOptions::from_env()).await
}
