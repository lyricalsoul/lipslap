//! ditto: an image-generation engine. Consumers implement `Template` for
//! their own themes, register them in a `Registry`, and hand that to `serve`.

pub mod fonts;
pub mod generator;
pub mod imaging;
pub mod logging;
pub mod overlay;
pub mod preview;
pub mod server;
pub mod template;
pub mod toolbox;
pub mod types;

pub use inventory;

pub use generator::OutputFormat;
pub use overlay::{apply_overlays, OverlaySpec};
pub use server::{serve, ServeOptions};
pub use template::{Draw, Registry, Template};
pub use types::GenerateData;
