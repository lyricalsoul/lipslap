//! A registry of named draw-time overlays, in the same spirit as `preview!`.
//! Consumers register one with `ditto::overlay!("name", |canvas, w, h| ...)`
//! next to wherever makes sense in their own code, then request it per
//! request via `GenerateData.overlays`. Unlike `preview!`, this isn't
//! debug-only — overlays like a watermark or a confidentiality banner are a
//! real runtime feature.
use skia_safe::Canvas;

pub struct Overlay {
    pub name: &'static str,
    pub draw: fn(&Canvas, i32, i32),
}

inventory::collect!(Overlay);

/// Registers a named overlay: `draw(canvas, width, height)` runs after the
/// theme's own drawing, for any generation whose `overlays` list includes
/// `name`.
#[macro_export]
macro_rules! overlay {
    ($name:expr, $draw:expr) => {
        $crate::inventory::submit! {
            $crate::overlay::Overlay { name: $name, draw: $draw }
        }
    };
}

/// Draws every overlay in `names` found in the registry, in order. Unknown
/// names are skipped rather than treated as an error, so a theme doesn't
/// need to validate the list itself.
pub fn apply_overlays(canvas: &Canvas, width: i32, height: i32, names: &[String]) {
    for name in names {
        if let Some(overlay) = inventory::iter::<Overlay>().find(|o| o.name == name.as_str()) {
            (overlay.draw)(canvas, width, height);
        }
    }
}
