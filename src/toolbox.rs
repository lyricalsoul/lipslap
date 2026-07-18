use skia_safe::{
    gradient, Canvas, ClipOp, Color, Color4f, ColorFilter, ColorMatrix, Font, Image, Paint, Point, RRect, Rect, TileMode,
};

fn parse_hex(hex: &str) -> (i32, i32, i32) {
    let hex = hex.trim_start_matches('#');
    let r = i32::from_str_radix(&hex[0..2], 16).unwrap_or(0);
    let g = i32::from_str_radix(&hex[2..4], 16).unwrap_or(0);
    let b = i32::from_str_radix(&hex[4..6], 16).unwrap_or(0);
    (r, g, b)
}

pub fn color(hex: &str) -> Color {
    let (r, g, b) = parse_hex(hex);
    Color::from_rgb(r as u8, g as u8, b as u8)
}

pub fn paint(c: Color) -> Paint {
    let mut p = Paint::default();
    p.set_anti_alias(true);
    p.set_color(c);
    p
}

pub fn color_variation(hex: &str, variation: f32) -> String {
    let (r, g, b) = parse_hex(hex);
    let adjust = |v: i32| ((v as f32 + (v as f32 * (variation / 100.0)).round()).clamp(0.0, 255.0)) as u8;
    format!("#{:02x}{:02x}{:02x}", adjust(r), adjust(g), adjust(b))
}

pub fn contrast_color(hex: &str) -> Color {
    let (r, g, b) = parse_hex(hex);
    let brightness = (r * 299 + g * 587 + b * 114) as f32 / 1000.0;
    if brightness >= 128.0 {
        Color::BLACK
    } else {
        Color::WHITE
    }
}

pub fn rrect(x: f32, y: f32, w: f32, h: f32, radius: f32) -> RRect {
    RRect::new_rect_xy(Rect::from_xywh(x, y, w, h), radius, radius)
}

pub fn draw_rounded_rect(canvas: &Canvas, x: f32, y: f32, w: f32, h: f32, radius: f32, paint: &Paint) {
    canvas.draw_rrect(rrect(x, y, w, h, radius), paint);
}

pub fn clip_rounded_rect(canvas: &Canvas, x: f32, y: f32, w: f32, h: f32, radius: f32) {
    canvas.clip_rrect(rrect(x, y, w, h, radius), Some(ClipOp::Intersect), Some(true));
}

pub fn rotate_radians(canvas: &Canvas, radians: f32) {
    canvas.rotate(radians.to_degrees(), None);
}

pub fn rotate_degrees(canvas: &Canvas, degrees: f32) {
    canvas.rotate(degrees, None);
}

pub fn draw_gradient_rectangle(canvas: &Canvas, x: f32, y: f32, w: f32, h: f32, hex: &str, radius: f32) {
    let colors = [Color4f::from(color(hex)), Color4f::from(color(&color_variation(hex, -35.0)))];
    let gradient_colors = gradient::Colors::new_evenly_spaced(&colors, TileMode::Clamp, None);
    let gradient_spec = gradient::Gradient::new(gradient_colors, gradient::Interpolation::default());
    let shader = gradient::shaders::linear_gradient((Point::new(0.0, y), Point::new(0.0, y + h)), &gradient_spec, None);

    let mut p = Paint::default();
    p.set_shader(shader);
    p.set_anti_alias(true);

    if radius > 0.0 {
        draw_rounded_rect(canvas, x, y, w, h, radius, &p);
    } else {
        canvas.draw_rect(Rect::from_xywh(x, y, w, h), &p);
    }
}

fn smooth_sampling() -> skia_safe::SamplingOptions {
    skia_safe::SamplingOptions::from(skia_safe::FilterMode::Linear)
}

pub fn draw_circle_image(canvas: &Canvas, image: &Image, x: f32, y: f32, radius: f32) {
    canvas.save();
    let path = skia_safe::Path::circle(Point::new(x + radius, y + radius), radius, None);
    canvas.clip_path(&path, Some(ClipOp::Intersect), Some(true));
    let dst = Rect::from_xywh(x, y, radius * 2.0, radius * 2.0);
    canvas.draw_image_rect_with_sampling_options(image, None, dst, smooth_sampling(), &Paint::default());
    canvas.restore();
}

pub fn draw_image_at(canvas: &Canvas, image: &Image, x: f32, y: f32) {
    canvas.draw_image_with_sampling_options(image, Point::new(x, y), smooth_sampling(), None);
}

pub fn duotone_filter(colors: &[&str]) -> ColorFilter {
    let stops: Vec<Color> = colors.iter().map(|c| color(c)).collect();
    let mut r_table = [0u8; 256];
    let mut g_table = [0u8; 256];
    let mut b_table = [0u8; 256];
    for i in 0..256 {
        let c = sample_gradient(&stops, i as f32 / 255.0);
        r_table[i] = c.r();
        g_table[i] = c.g();
        b_table[i] = c.b();
    }
    let palette = skia_safe::color_filters::table_argb(None, Some(&r_table), Some(&g_table), Some(&b_table))
        .expect("table color filter");

    // reduces every pixel to its average-brightness gray value first
    let grayscale = color_filters_matrix_average();

    skia_safe::color_filters::compose(palette, grayscale).expect("compose color filter")
}

fn color_filters_matrix_average() -> ColorFilter {
    #[rustfmt::skip]
    let m = ColorMatrix::new(
        1.0 / 3.0, 1.0 / 3.0, 1.0 / 3.0, 0.0, 0.0,
        1.0 / 3.0, 1.0 / 3.0, 1.0 / 3.0, 0.0, 0.0,
        1.0 / 3.0, 1.0 / 3.0, 1.0 / 3.0, 0.0, 0.0,
        0.0, 0.0, 0.0, 1.0, 0.0,
    );
    skia_safe::color_filters::matrix(&m, None)
}

fn sample_gradient(stops: &[Color], t: f32) -> Color {
    if stops.len() == 1 {
        return stops[0];
    }
    let scaled = t.clamp(0.0, 1.0) * (stops.len() - 1) as f32;
    let i = (scaled as usize).min(stops.len() - 2);
    lerp_color(stops[i], stops[i + 1], scaled - i as f32)
}

fn lerp_color(a: Color, b: Color, t: f32) -> Color {
    let lerp = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color::from_rgb(lerp(a.r(), b.r()), lerp(a.g(), b.g()), lerp(a.b(), b.b()))
}

pub fn draw_duotone_image(canvas: &Canvas, image: &Image, x: f32, y: f32, colors: &[&str]) {
    let mut p = Paint::default();
    p.set_color_filter(duotone_filter(colors));
    canvas.draw_image_with_sampling_options(image, Point::new(x, y), smooth_sampling(), Some(&p));
}

pub fn measure_text(font: &Font, text: &str) -> f32 {
    font.measure_str(text, None).0
}

/// A font family + size, bundled since every text-drawing helper here needs
/// both together.
#[derive(Clone, Copy)]
pub struct TextStyle<'a> {
    pub family: &'a str,
    pub size: f32,
}

pub fn write_text_considering_width(canvas: &Canvas, paint: &Paint, style: TextStyle, text: &str, pos: Point, max_width: f32) {
    let mut current_size = style.size;
    let mut font = crate::fonts::font(style.family, current_size);
    while measure_text(&font, text) > max_width && current_size > 1.0 {
        current_size -= 1.0;
        font = crate::fonts::font(style.family, current_size);
    }
    canvas.draw_str(text, pos, &font, paint);
}

pub fn write_text_centralized_considering(
    canvas: &Canvas,
    paint: &Paint,
    style: TextStyle,
    text: &str,
    left_boundary: f32,
    y: f32,
    max_width: f32,
) {
    let font = crate::fonts::font(style.family, style.size);
    let width = measure_text(&font, text);
    let x = (left_boundary + (max_width - width) / 2.0).abs();
    write_text_considering_width(canvas, paint, style, text, Point::new(x, y), max_width);
}

pub fn draw_text_shadow(canvas: &Canvas, family: &str, size: f32, text: &str, x: f32, y: f32) {
    let font = crate::fonts::font(family, size);
    let p = paint(Color::from_argb(77, 0, 0, 0));
    canvas.draw_str(text, Point::new(x + 1.0, y + 2.0), &font, &p);
}

pub fn readable_number(n: f64) -> String {
    if n < 1000.0 {
        format!("{n}")
    } else if n < 1_000_000.0 {
        format!("{:.1}k", n / 1000.0)
    } else if n < 1_000_000_000.0 {
        format!("{:.1}m", n / 1_000_000.0)
    } else {
        format!("{:.1}b", n / 1_000_000_000.0)
    }
}

pub struct GridCell {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

pub fn grid_layout(count: usize, area: Rect, aspect_ratio: f32, gap: f32) -> Vec<GridCell> {
    if count == 0 {
        return Vec::new();
    }

    let mut best: Option<(usize, usize, f32, f32)> = None;
    for cols in 1..=count {
        let rows = count.div_ceil(cols);
        let mut cell_w = (area.width() - (cols as f32 - 1.0) * gap) / cols as f32;
        let mut cell_h = cell_w / aspect_ratio;
        if rows as f32 * cell_h + (rows as f32 - 1.0) * gap > area.height() {
            cell_h = (area.height() - (rows as f32 - 1.0) * gap) / rows as f32;
            cell_w = cell_h * aspect_ratio;
        }
        if cell_w <= 0.0 || cell_h <= 0.0 {
            continue;
        }
        let is_better = match best {
            None => true,
            Some((_, _, best_w, _)) => cell_w > best_w,
        };
        if is_better {
            best = Some((cols, rows, cell_w, cell_h));
        }
    }
    let Some((cols, rows, cell_w, cell_h)) = best else {
        return Vec::new();
    };

    let grid_h = rows as f32 * cell_h + (rows as f32 - 1.0) * gap;
    let top = area.top + (area.height() - grid_h) / 2.0;

    let mut cells = Vec::with_capacity(count);
    for row in 0..rows {
        let items_in_row = if row == rows - 1 { count - row * cols } else { cols };
        let row_w = items_in_row as f32 * cell_w + (items_in_row as f32 - 1.0) * gap;
        let left = area.left + (area.width() - row_w) / 2.0;
        let y = top + row as f32 * (cell_h + gap);
        for col in 0..items_in_row {
            let x = left + col as f32 * (cell_w + gap);
            cells.push(GridCell { x, y, w: cell_w, h: cell_h });
        }
    }
    cells
}

pub fn draw_text_wrapped(canvas: &Canvas, paint: &Paint, style: TextStyle, text: &str, pos: Point, max_width: f32, line_height: f32) {
    let font = crate::fonts::font(style.family, style.size);
    let (x, mut y) = (pos.x, pos.y);
    let mut line = String::new();
    let words: Vec<&str> = text.split(' ').collect();
    for (n, word) in words.iter().enumerate() {
        let test_line = format!("{line}{word} ");
        if measure_text(&font, &test_line) > max_width && n > 0 {
            canvas.draw_str(&line, Point::new(x, y), &font, paint);
            line = format!("{word} ");
            y += line_height;
        } else {
            line = test_line;
        }
    }
    canvas.draw_str(&line, Point::new(x, y), &font, paint);
}
