//! A minimal HTTP server exposing one theme, showing how a consumer wires up
//! `ditto::serve` and registers overlays. Run it, then:
//!   curl -X POST localhost:4444/generate -H 'content-type: application/json' \
//!     -d '{"theme":"greeting","data":{"name":"world"},"overlays":["grid","watermark"]}'
use async_trait::async_trait;
use ditto::toolbox::{color, paint};
use ditto::{GenerateData, Registry, ServeOptions, Template};
use serde::Deserialize;

ditto::overlay!("grid", |canvas, w, h| {
    let line = paint(color("#ffffff"));
    let mut x = 0;
    while x < w {
        canvas.draw_line((x as f32, 0.0), (x as f32, h as f32), &line);
        x += 20;
    }
    let mut y = 0;
    while y < h {
        canvas.draw_line((0.0, y as f32), (w as f32, y as f32), &line);
        y += 20;
    }
});

ditto::overlay!("watermark", |canvas, w, _h| {
    let font = ditto::fonts::fallback(20.0);
    let text = "DEMO";
    canvas.draw_str(text, (w as f32 - 90.0, 30.0), &font, &paint(color("#f5f5f5")));
});

#[derive(Deserialize)]
struct GreetingData {
    name: String,
}

struct GreetingTemplate;

#[async_trait]
impl Template for GreetingTemplate {
    fn width(&self) -> i32 {
        800
    }

    fn height(&self) -> i32 {
        400
    }

    async fn draw(&self, _client: &reqwest::Client, meta: &GenerateData) -> anyhow::Result<ditto::Draw> {
        let data: GreetingData = serde_json::from_value(meta.data.clone())?;
        Ok(Box::new(move |canvas| {
            canvas.draw_color(color("#1e1b4b"), skia_safe::BlendMode::Src);
            let font = ditto::fonts::fallback(48.0);
            let text = format!("Hello, {}!", data.name);
            canvas.draw_str(&text, (60.0, 220.0), &font, &paint(color("#f5f5f5")));
        }))
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let registry = Registry::new().register("greeting", GreetingTemplate);
    ditto::serve(registry, ServeOptions::from_env()).await
}
