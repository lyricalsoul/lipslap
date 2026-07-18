//! Debug-only preview registry, in the spirit of SwiftUI/Compose `#Preview`.
//! Themes call `ditto::preview!` next to their own code to register a
//! sample payload; nothing here exists in a release build.
use crate::overlay::OverlaySpec;
use crate::template::Registry;
use crate::types::GenerateData;
use std::convert::Infallible;
use std::sync::Arc;
use warp::{Filter, Rejection};

pub struct Preview {
    pub theme: &'static str,
    pub name: &'static str,
    pub data: fn() -> serde_json::Value,
    pub overlays: fn() -> Vec<OverlaySpec>,
}

inventory::collect!(Preview);

/// Registers a preview under `theme` with a human-readable `name`. `data` is
/// the theme's own JSON payload (what normally arrives as `GenerateData.data`).
/// An optional trailing `overlays` expression (a `Vec<OverlaySpec>`) applies
/// overlays to the preview the same way a real request's `overlays` field
/// would.
#[macro_export]
macro_rules! preview {
    ($theme:expr, $name:expr, $data:expr $(, $overlays:expr)?) => {
        #[cfg(debug_assertions)]
        $crate::inventory::submit! {
            $crate::preview::Preview {
                theme: $theme,
                name: $name,
                data: || $data,
                overlays: || {
                    #[allow(unused_mut)]
                    let mut overlays: Vec<$crate::overlay::OverlaySpec> = Vec::new();
                    $(overlays = $overlays;)?
                    overlays
                },
            }
        }
    };
}

fn all() -> Vec<&'static Preview> {
    inventory::iter::<Preview>().collect()
}

const INDEX_HTML: &str = include_str!("../assets/testingPage/index.html");
const VIEWER_HTML: &str = include_str!("../assets/testingPage/testViewer.html");
const INDEX_CSS: &str = include_str!("../assets/testingPage/index.css");

/// Renders trace entries as a waterfall: horizontal position is when the call
/// started, width is how long it took, both relative to the whole request —
/// so concurrent fetches visibly overlap instead of all reading as full-width
/// bars.
fn trace_waterfall(trace: &[crate::logging::TraceEntry]) -> String {
    if trace.is_empty() {
        return "<p class=\"trace-empty\">no trace recorded</p>".to_string();
    }
    let total = trace
        .iter()
        .map(|e| e.offset_ms + e.duration_ms)
        .max()
        .unwrap_or(1)
        .max(1) as f64;

    let mut sorted: Vec<&crate::logging::TraceEntry> = trace.iter().collect();
    sorted.sort_by_key(|e| e.offset_ms);

    let rows: String = sorted
        .iter()
        .map(|e| {
            let left = e.offset_ms as f64 / total * 100.0;
            let width = (e.duration_ms as f64 / total * 100.0).max(0.4);
            let label = e.label.strip_prefix("generator.").unwrap_or(&e.label);
            format!(
                "<div class=\"trace-row\">\
                    <div class=\"trace-label\" title=\"{label}\">{label}</div>\
                    <div class=\"trace-track\">\
                        <div class=\"trace-bar\" style=\"left:{left}%;width:{width}%\"></div>\
                    </div>\
                    <div class=\"trace-ms\">{ms}ms</div>\
                </div>",
                ms = e.duration_ms,
            )
        })
        .collect();
    format!("<div class=\"trace\">{rows}</div>")
}

fn index_page() -> String {
    let items = all()
        .iter()
        .enumerate()
        .map(|(i, p)| {
            format!(
                "<li><a href=\"/testing/render/{i}\"><span>{}</span><span class=\"theme-tag\">{}</span></a></li>",
                p.name, p.theme
            )
        })
        .collect::<String>();
    INDEX_HTML.replace("{{tests}}", &items)
}

/// Warp routes for the `/testing` dashboard. Only mounted in debug builds
/// (see `server.rs`).
pub fn routes(
    registry: Arc<Registry>,
    client: reqwest::Client,
) -> impl Filter<Extract = (impl warp::Reply,), Error = Rejection> + Clone {
    let index = warp::path("testing")
        .and(warp::path::end())
        .map(|| warp::reply::html(index_page()));

    let css = warp::path!("testing" / "index.css")
        .map(|| warp::reply::with_header(INDEX_CSS, "content-type", "text/css"));

    let render = warp::path!("testing" / "render" / usize).and_then(move |i: usize| {
        let registry = registry.clone();
        let client = client.clone();
        async move {
            let previews = all();
            let Some(preview) = previews.get(i) else {
                return Ok::<_, Infallible>(warp::reply::html("no such preview".to_string()));
            };

            let data = GenerateData {
                id: None,
                theme: preview.theme.to_string(),
                overlays: (preview.overlays)(),
                data: (preview.data)(),
            };

            let (result, trace) = crate::logging::with_trace(crate::generator::generate(&registry, &client, data)).await;

            let html = match result {
                Ok(outcome) => VIEWER_HTML
                    .replace("{{testName}}", preview.name)
                    .replace("{{id}}", &outcome.id)
                    .replace("{{ext}}", outcome.extension)
                    .replace("{{time}}", &outcome.time_ms.to_string())
                    .replace("{{trace}}", &trace_waterfall(&trace)),
                Err(e) => format!("<pre>{e}</pre>"),
            };
            Ok(warp::reply::html(html))
        }
    });

    index.or(css).or(render)
}
