use sha1::{Digest, Sha1};
use skia_safe::{
    surfaces, AlphaType, ColorType, EncodedImageFormat, ISize, ImageInfo, Rect, SamplingOptions,
};
use std::path::PathBuf;
use std::time::SystemTime;
use tokio::fs;

pub fn cache_dir() -> PathBuf {
    std::env::var("CACHE_DIR")
        .unwrap_or_else(|_| ".cache/ditto".into())
        .into()
}

pub fn generation_cache_dir() -> PathBuf {
    std::env::var("EXPORT_DIR")
        .unwrap_or_else(|_| format!("{}/generated", cache_dir().display()))
        .into()
}

pub async fn create_directories() -> std::io::Result<()> {
    fs::create_dir_all(cache_dir()).await?;
    fs::create_dir_all(generation_cache_dir()).await?;
    Ok(())
}

pub fn hashed_image_url(id: &str, x: u32, y: u32) -> String {
    let mut hasher = Sha1::new();
    hasher.update(format!("{id}?size={x}x{y}"));
    hex::encode(hasher.finalize())
}

pub fn is_image_cached(id: &str, x: u32, y: u32) -> bool {
    cache_dir()
        .join(format!("{}.jpg", hashed_image_url(id, x, y)))
        .is_file()
}

pub async fn get_image_from_disk(id: &str, x: u32, y: u32) -> Option<Vec<u8>> {
    fs::read(cache_dir().join(format!("{}.jpg", hashed_image_url(id, x, y))))
        .await
        .ok()
}

pub async fn save_image(id: &str, x: u32, y: u32, image: &[u8]) -> std::io::Result<()> {
    fs::create_dir_all(cache_dir()).await?;
    fs::write(
        cache_dir().join(format!("{}.jpg", hashed_image_url(id, x, y))),
        image,
    )
    .await
}

pub fn resize_cover_jpeg(bytes: &[u8], x: u32, y: u32, high_quality: bool) -> Option<Vec<u8>> {
    let image =
        skia_safe::images::deferred_from_encoded_data(skia_safe::Data::new_copy(bytes), None)?;

    let (src_w, src_h) = (image.width() as f32, image.height() as f32);
    let (dst_w, dst_h) = (x as f32, y as f32);

    let scale = (src_w / dst_w).min(src_h / dst_h);
    let (crop_w, crop_h) = (dst_w * scale, dst_h * scale);
    let (crop_x, crop_y) = ((src_w - crop_w) / 2.0, (src_h - crop_h) / 2.0);
    let src_rect = Rect::from_xywh(crop_x, crop_y, crop_w, crop_h);
    let dst_rect = Rect::from_xywh(0.0, 0.0, dst_w, dst_h);

    let info = ImageInfo::new(
        ISize::new(x as i32, y as i32),
        ColorType::RGBA8888,
        AlphaType::Premul,
        None,
    );
    let mut surface = surfaces::raster(&info, None, None)?;
    let canvas = surface.canvas();

    let sampling = if high_quality {
        SamplingOptions::from(skia_safe::CubicResampler::mitchell())
    } else {
        SamplingOptions::default()
    };
    canvas.draw_image_rect_with_sampling_options(
        &image,
        Some((&src_rect, skia_safe::canvas::SrcRectConstraint::Strict)),
        dst_rect,
        sampling,
        &skia_safe::Paint::default(),
    );

    let snapshot = surface.image_snapshot();
    
    let format = if image.alpha_type() == AlphaType::Opaque {
        EncodedImageFormat::JPEG
    } else {
        EncodedImageFormat::PNG
    };
    let quality = if format == EncodedImageFormat::JPEG { 90 } else { 100 };
    let data = skia_safe::encode::image(None, &snapshot, format, quality)?;
    Some(data.as_bytes().to_vec())
}

pub async fn download_image(
    client: &reqwest::Client,
    url: &str,
    x: u32,
    y: u32,
    high_quality: bool,
) -> anyhow::Result<Vec<u8>> {
    let response = client.get(url).send().await?;
    if !response.status().is_success() {
        anyhow::bail!("failed to download image {url} (status {})", response.status());
    }
    let bytes = response.bytes().await?.to_vec();

    let resized = resize_cover_jpeg(&bytes, x, y, high_quality)
        .ok_or_else(|| anyhow::anyhow!("failed to decode/resize image {url}"))?;
    save_image(url, x, y, &resized).await?;
    Ok(resized)
}

pub async fn load_image(
    client: &reqwest::Client,
    url: &str,
    x: u32,
    y: u32,
    high_quality: bool,
) -> anyhow::Result<skia_safe::Image> {
    let bytes = if let Some(cached) = get_image_from_disk(url, x, y).await {
        cached
    } else {
        download_image(client, url, x, y, high_quality).await?
    };

    skia_safe::images::deferred_from_encoded_data(skia_safe::Data::new_copy(&bytes), None)
        .ok_or_else(|| anyhow::anyhow!("failed to decode image {url}"))
}

pub async fn load_image_from_assets(path: &str, x: u32, y: u32) -> anyhow::Result<skia_safe::Image> {
    let bytes = if let Some(cached) = get_image_from_disk(path, x, y).await {
        cached
    } else {
        let source = fs::read(format!("./assets/images/{path}")).await?;
        let resized = resize_cover_jpeg(&source, x, y, true)
            .ok_or_else(|| anyhow::anyhow!("failed to decode/resize asset {path}"))?;
        save_image(path, x, y, &resized).await?;
        resized
    };

    skia_safe::images::deferred_from_encoded_data(skia_safe::Data::new_copy(&bytes), None)
        .ok_or_else(|| anyhow::anyhow!("failed to decode asset {path}"))
}

pub async fn clear_cache() -> std::io::Result<()> {
    remove_older_than(&cache_dir(), 2.0).await?;
    remove_older_than(&generation_cache_dir(), 0.0).await?;
    Ok(())
}

async fn remove_older_than(dir: &PathBuf, max_age_days: f64) -> std::io::Result<()> {
    let mut entries = fs::read_dir(dir).await?;
    while let Some(entry) = entries.next_entry().await? {
        let metadata = entry.metadata().await?;
        if metadata.is_dir() {
            continue;
        }
        let created = metadata.created().unwrap_or(SystemTime::now());
        let age_days = created
            .elapsed()
            .unwrap_or_default()
            .as_secs_f64()
            / 60.0
            / 60.0
            / 24.0;
        if age_days > max_age_days {
            let _ = fs::remove_file(entry.path()).await;
        }
    }
    Ok(())
}
