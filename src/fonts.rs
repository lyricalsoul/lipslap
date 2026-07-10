use skia_safe::{FontMgr, Typeface};
use std::collections::HashMap;
use std::sync::OnceLock;

static FONTS: OnceLock<HashMap<String, Typeface>> = OnceLock::new();
static FALLBACK_TYPEFACE: OnceLock<Typeface> = OnceLock::new();

fn font_name_from_path(path: &std::path::Path) -> String {
    let file_name = path.file_name().unwrap().to_string_lossy().to_string();
    let mut spaced = String::new();
    for c in file_name.chars() {
        if c.is_ascii_uppercase() && !spaced.is_empty() {
            spaced.push(' ');
        }
        spaced.push(c);
    }
    spaced
        .trim()
        .replace('-', "")
        .replace(".ttf", "")
        .replace(".otf", "")
}

fn collect_font_paths(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_font_paths(&path, out);
        } else if path.extension().is_some_and(|e| e == "ttf") {
            out.push(path);
        }
    }
}

pub fn load_fonts() {
    let mut paths = Vec::new();
    collect_font_paths(std::path::Path::new("./assets/fonts"), &mut paths);

    let font_mgr = FontMgr::default();
    let mut map = HashMap::new();
    for path in paths {
        let name = font_name_from_path(&path);
        let Ok(bytes) = std::fs::read(&path) else {
            eprintln!("fonts: failed to read {}", path.display());
            continue;
        };
        
        let typeface = font_mgr
            .new_from_data(&bytes, None)
            .or_else(|| font_mgr.match_family_style(&name, skia_safe::FontStyle::default()));
        match typeface {
            Some(typeface) => {
                map.insert(name, typeface);
            }
            None => eprintln!("fonts: failed to register {name}"),
        }
    }
    let _ = FONTS.set(map);
}

pub fn font(family: &str, size: f32) -> skia_safe::Font {
    let typeface = FONTS
        .get()
        .and_then(|m| m.get(family))
        .cloned()
        .unwrap_or_else(fallback_typeface);
    skia_safe::Font::new(typeface, size)
}

fn fallback_typeface() -> Typeface {
    FALLBACK_TYPEFACE
        .get_or_init(|| {
            FontMgr::default()
                .legacy_make_typeface(None, Default::default())
                .unwrap()
        })
        .clone()
}

pub fn fallback(size: f32) -> skia_safe::Font {
    skia_safe::Font::new(fallback_typeface(), size)
}
