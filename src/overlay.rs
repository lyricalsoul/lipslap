//! A registry of named draw-time overlays, in the same spirit as `preview!`.
//! Consumers register one with `ditto::overlay!("name", |canvas, w, h, data| ...)`
//! next to wherever makes sense in their own code, then request it per
//! request via `GenerateData.overlays`. Unlike `preview!`, this isn't
//! debug-only — overlays like a watermark or a confidentiality banner are a
//! real runtime feature.
use serde::Deserialize;
use skia_safe::Canvas;

pub struct Overlay {
    pub name: &'static str,
    pub draw: fn(&Canvas, i32, i32, &serde_json::Value),
}

inventory::collect!(Overlay);

/// An overlay request: just a name (`"grid"`), or a name plus arbitrary data
/// the overlay itself interprets (`{"name": "duotone", "data": {"colors": [...]}}`).
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum OverlaySpec {
    Name(String),
    WithData {
        name: String,
        #[serde(default)]
        data: serde_json::Value,
    },
}

impl OverlaySpec {
    pub fn new(name: impl Into<String>, data: serde_json::Value) -> Self {
        OverlaySpec::WithData { name: name.into(), data }
    }

    fn name(&self) -> &str {
        match self {
            OverlaySpec::Name(name) => name,
            OverlaySpec::WithData { name, .. } => name,
        }
    }

    fn data(&self) -> &serde_json::Value {
        static NULL: serde_json::Value = serde_json::Value::Null;
        match self {
            OverlaySpec::Name(_) => &NULL,
            OverlaySpec::WithData { data, .. } => data,
        }
    }
}

impl From<&str> for OverlaySpec {
    fn from(name: &str) -> Self {
        OverlaySpec::Name(name.to_string())
    }
}

/// Registers a named overlay: `draw(canvas, width, height, data)` runs after
/// the theme's own drawing, for any generation whose `overlays` list
/// includes `name`. `data` is that request's overlay data, or `null` if none
/// was given.
#[macro_export]
macro_rules! overlay {
    ($name:expr, $draw:expr) => {
        $crate::inventory::submit! {
            $crate::overlay::Overlay { name: $name, draw: $draw }
        }
    };
}

/// Draws every overlay in `specs` found in the registry, in order. Unknown
/// names are skipped rather than treated as an error, so a theme doesn't
/// need to validate the list itself.
pub fn apply_overlays(canvas: &Canvas, width: i32, height: i32, specs: &[OverlaySpec]) {
    for spec in specs {
        if let Some(overlay) = inventory::iter::<Overlay>().find(|o| o.name == spec.name()) {
            (overlay.draw)(canvas, width, height, spec.data());
        }
    }
}
