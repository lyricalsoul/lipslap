//! Draws a row of colored rounded rectangles, showing the toolbox primitives
//! and a theme reading its own JSON payload.
//!   curl -X POST localhost:4444/generate -H 'content-type: application/json' \
//!     -d '{"theme":"squares","data":{}}'
use async_trait::async_trait;
use ditto::toolbox::{color, draw_rounded_rect, paint};
use ditto::{GenerateData, Registry, ServeOptions, Template};
use serde::Deserialize;

#[derive(Deserialize)]
struct SquaresData {
    #[serde(default = "default_colors")]
    colors: Vec<String>,
}

fn default_colors() -> Vec<String> {
    ["#ef4444", "#f59e0b", "#22c55e", "#3b82f6", "#8b5cf6"]
        .into_iter()
        .map(String::from)
        .collect()
}

struct SquaresTemplate;

#[async_trait]
impl Template for SquaresTemplate {
    fn width(&self) -> i32 {
        700
    }

    fn height(&self) -> i32 {
        150
    }

    async fn draw(&self, _client: &reqwest::Client, meta: &GenerateData) -> anyhow::Result<ditto::Draw> {
        let data: SquaresData = serde_json::from_value(meta.data.clone())?;
        Ok(Box::new(move |canvas| {
            canvas.draw_color(color("#111827"), skia_safe::BlendMode::Src);
            for (i, hex) in data.colors.iter().enumerate() {
                let x = 20.0 + i as f32 * 130.0;
                draw_rounded_rect(canvas, x, 25.0, 110.0, 100.0, 16.0, &paint(color(hex)));
            }
        }))
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let registry = Registry::new().register("squares", SquaresTemplate);
    ditto::serve(registry, ServeOptions::from_env()).await
}
