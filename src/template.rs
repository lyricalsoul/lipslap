use crate::generator::OutputFormat;
use crate::types::GenerateData;
use async_trait::async_trait;
use skia_safe::Canvas;
use std::collections::HashMap;

/// What `Template::draw` hands back: a closure that draws onto the canvas,
/// boxed so it can cross onto ditto's blocking render pool.
pub type Draw = Box<dyn FnOnce(&Canvas) + Send>;

/// A theme a consumer registers with ditto. A theme only draws — `ditto`
/// owns surface allocation, runs the draw off the async runtime, applies any
/// requested overlays, and encodes the result according to `output_format()`.
#[async_trait]
pub trait Template: Send + Sync {
    fn width(&self) -> i32;
    fn height(&self) -> i32;

    /// Output format and quality for this theme's images. Defaults to JPEG,
    /// quality 96.
    fn output_format(&self) -> OutputFormat {
        OutputFormat::default()
    }

    /// Fetches whatever assets this theme needs, then returns a closure that
    /// draws them onto the canvas. `meta.data` holds this theme's own
    /// (yet-undeserialized) payload.
    async fn draw(&self, client: &reqwest::Client, meta: &GenerateData) -> anyhow::Result<Draw>;
}

/// The set of themes a ditto-based server exposes. Consumers build one with
/// `Registry::new().register("name", MyTemplate)` and hand it to `serve`.
#[derive(Default)]
pub struct Registry {
    templates: HashMap<String, Box<dyn Template>>,
}

impl Registry {
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn register(mut self, name: impl Into<String>, template: impl Template + 'static) -> Self {
        self.templates.insert(name.into(), Box::new(template));
        self
    }

    pub fn theme_names(&self) -> Vec<&str> {
        self.templates.keys().map(String::as_str).collect()
    }

    pub fn get(&self, name: &str) -> Option<&dyn Template> {
        self.templates.get(name).map(AsRef::as_ref)
    }
}
