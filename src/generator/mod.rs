use crate::template::Registry;
use crate::types::GenerateData;
use skia_safe::Canvas;
use std::time::Instant;

pub struct GenerateOutcome {
    pub id: String,
    pub time_ms: u128,
    pub extension: &'static str,
}

/// A theme's chosen output encoding and quality. Set via `Template::output_format`
#[derive(Clone, Copy)]
pub enum OutputFormat {
    Jpeg { quality: u8 },
    Png,
    Webp { quality: u8 },
}

impl Default for OutputFormat {
    fn default() -> Self {
        OutputFormat::Jpeg { quality: 96 }
    }
}

impl OutputFormat {
    pub fn extension(&self) -> &'static str {
        match self {
            OutputFormat::Jpeg { .. } => "jpg",
            OutputFormat::Png => "png",
            OutputFormat::Webp { .. } => "webp",
        }
    }

    fn into_skia(self) -> (skia_safe::EncodedImageFormat, u32) {
        match self {
            OutputFormat::Jpeg { quality } => (skia_safe::EncodedImageFormat::JPEG, quality as u32),
            OutputFormat::Png => (skia_safe::EncodedImageFormat::PNG, 100),
            OutputFormat::Webp { quality } => (skia_safe::EncodedImageFormat::WEBP, quality as u32),
        }
    }
}

pub async fn generate(
    registry: &Registry,
    client: &reqwest::Client,
    mut data: GenerateData,
) -> anyhow::Result<GenerateOutcome> {
    let id = data
        .id
        .take()
        .unwrap_or_else(|| format!("{:x}{:x}", rand_u64(), rand_u64()));

    let template = registry
        .get(&data.theme)
        .ok_or_else(|| anyhow::anyhow!("Invalid theme"))?;

    let start = Instant::now();
    let draw = template.draw(client, &data).await?;
    let (width, height) = (template.width(), template.height());
    let format = template.output_format();
    let overlays = std::mem::take(&mut data.overlays);

    let bytes = render_image(width, height, format, move |canvas| {
        draw(canvas);
        crate::overlay::apply_overlays(canvas, width, height, &overlays);
    })
    .await?;
    let elapsed = start.elapsed().as_millis();
    crate::logging::debug("generator.generate", &format!("generated {} for {id} in {elapsed}ms", data.theme));

    let extension = format.extension();
    tokio::fs::write(
        crate::imaging::generation_cache_dir().join(format!("{id}.{extension}")),
        bytes,
    )
    .await?;

    Ok(GenerateOutcome {
        id,
        time_ms: elapsed,
        extension,
    })
}

fn rand_u64() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    (nanos as u64) ^ (std::process::id() as u64)
}

async fn render_image(
    width: i32,
    height: i32,
    format: OutputFormat,
    draw: impl FnOnce(&Canvas) + Send + 'static,
) -> anyhow::Result<Vec<u8>> {
    let trace_ctx = crate::logging::capture_trace_context();
    let bytes = tokio::task::spawn_blocking(move || {
        crate::logging::with_trace_context(trace_ctx, || render_image_sync(width, height, format, draw))
    })
    .await??;
    Ok(bytes)
}

fn render_image_sync(width: i32, height: i32, format: OutputFormat, draw: impl FnOnce(&Canvas)) -> anyhow::Result<Vec<u8>> {
    let info = skia_safe::ImageInfo::new(
        skia_safe::ISize::new(width, height),
        skia_safe::ColorType::RGBA8888,
        skia_safe::AlphaType::Premul,
        None,
    );
    let mut surface = skia_safe::surfaces::raster(&info, None, None)
        .ok_or_else(|| anyhow::anyhow!("failed to allocate surface"))?;

    crate::logging::timed("generator.render_image", "draw", || draw(surface.canvas()));

    let snapshot = surface.image_snapshot();
    let (encoded_format, quality) = format.into_skia();
    let encoded = crate::logging::timed("generator.render_image", "encode", || {
        skia_safe::encode::image(None, &snapshot, encoded_format, quality)
    })
    .ok_or_else(|| anyhow::anyhow!("failed to encode image"))?;

    Ok(encoded.as_bytes().to_vec())
}
