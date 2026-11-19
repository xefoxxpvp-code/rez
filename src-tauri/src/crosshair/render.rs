//! Draws a [`Crosshair`] into an RGBA image. The same output feeds the overlay window and
//! the UI previews, so what the editor shows is exactly what appears in game.

use std::path::Path;

use tiny_skia::{Color, Paint, PathBuilder, Pixmap, PixmapPaint, Rect, Stroke, Transform};

use super::model::{Crosshair, Lines};
use crate::imaging::{decode_png, encode_png, unpremultiply, RgbaImage};

/// Integer pixel layout of one set of arms.
struct Arms {
    gap_x: i32,
    gap_y: i32,
    len_x: i32,
    len_y: i32,
    /// Thickness of horizontal arms (rows) and of vertical arms (columns).
    thick_h: i32,
    thick_v: i32,
}

fn px_round(v: f32) -> i32 {
    v.round() as i32
}

fn arms(lines: &Lines, scale: f32, x_factor: f32) -> Arms {
    let vertical = lines.vertical_length.unwrap_or(lines.length);
    Arms {
        gap_x: px_round(lines.gap * scale * x_factor),
        gap_y: px_round(lines.gap * scale),
        len_x: px_round(lines.length * scale * x_factor).max(1),
        len_y: px_round(vertical * scale).max(1),
        thick_h: px_round(lines.thickness * scale).max(1),
        thick_v: px_round(lines.thickness * scale * x_factor).max(1),
    }
}

/// Rectangles (x, y, w, h) of the arms around center pixel (c, c).
fn arm_rects(a: &Arms, c: i32, t_style: bool) -> Vec<(i32, i32, i32, i32)> {
    let row = c - a.thick_h / 2;
    let col = c - a.thick_v / 2;
    let mut rects = vec![
        (c + a.gap_x, row, a.len_x, a.thick_h),                     // right
        (c - a.gap_x - a.len_x + 1, row, a.len_x, a.thick_h),       // left
        (col, c + a.gap_y, a.thick_v, a.len_y),                     // bottom
    ];
    if !t_style {
        rects.push((col, c - a.gap_y - a.len_y + 1, a.thick_v, a.len_y)); // top
    }
    rects
}

fn paint(r: u8, g: u8, b: u8, opacity: f32, anti_alias: bool) -> Paint<'static> {
    let mut p = Paint::default();
    p.set_color(Color::from_rgba8(r, g, b, (opacity.clamp(0.0, 1.0) * 255.0).round() as u8));
    p.anti_alias = anti_alias;
    p
}

fn fill(pm: &mut Pixmap, (x, y, w, h): (i32, i32, i32, i32), p: &Paint) {
    if let Some(rect) = Rect::from_xywh(x as f32, y as f32, w as f32, h as f32) {
        pm.fill_rect(rect, p, Transform::identity(), None);
    }
}

fn grow((x, y, w, h): (i32, i32, i32, i32), ox: i32, oy: i32) -> (i32, i32, i32, i32) {
    (x - ox, y - oy, w + 2 * ox, h + 2 * oy)
}

fn load_image(c: &Crosshair, image_dir: &Path) -> Option<RgbaImage> {
    let name = c.image.as_deref()?;
    let bytes = std::fs::read(image_dir.join(name)).ok()?;
    decode_png(&bytes).ok()
}

/// Largest canvas side the renderer will ever allocate.
pub const MAX_CANVAS: u32 = 2049;

/// Renders the crosshair centered on an odd, square canvas. Pixels are premultiplied RGBA;
/// the center pixel is (width / 2, height / 2).
pub fn render(c: &Crosshair, scale: f32, x_factor: f32, image_dir: &Path) -> RgbaImage {
    let c = &c.sanitized();
    let scale = if scale.is_finite() { scale.clamp(0.1, 8.0) } else { 1.0 };
    let x_factor = if x_factor.is_finite() { x_factor.clamp(0.1, 4.0) } else { 1.0 };
    let o = if c.outline.enabled { px_round(c.outline.thickness * scale).max(1) } else { 0 };
    let o_x = if c.outline.enabled { px_round(c.outline.thickness * scale * x_factor).max(1) } else { 0 };
    let image = load_image(c, image_dir);
    let line_sets: Vec<(&Lines, Arms)> = [&c.inner, &c.outer]
        .into_iter()
        .filter(|l| l.enabled && image.is_none())
        .map(|l| (l, arms(l, scale, x_factor)))
        .collect();

    // Half-size of the drawing, in pixels.
    let mut extent = 0i32;
    for (_, a) in &line_sets {
        extent = extent.max(a.gap_x + a.len_x).max(a.gap_y + a.len_y);
    }
    let dot = px_round(c.dot.size * scale).max(1);
    if c.dot.enabled && image.is_none() {
        extent = extent.max(dot / 2 + 1);
    }
    let ring_r = c.circle.radius * scale;
    let ring_w = (c.circle.thickness * scale).max(1.0);
    if c.circle.enabled && image.is_none() {
        extent = extent.max((ring_r + ring_w).ceil() as i32);
    }
    let img_scale = c.image_scale.max(0.01) * scale;
    if let Some(img) = &image {
        extent = extent.max(((img.width.max(img.height)) as f32 * img_scale / 2.0).ceil() as i32);
    }
    extent = (extent + o + 2).min((MAX_CANVAS as i32 - 1) / 2);
    let size = (2 * extent + 1) as u32;
    let center = extent;
    let mut pm = Pixmap::new(size, size).expect("non-zero canvas");
    let (r, g, b) = c.rgb();

    if let Some(img) = image {
        if let Some(src) = premultiplied_pixmap(&img) {
            let (w, h) = (img.width as f32 * img_scale * x_factor, img.height as f32 * img_scale);
            let tx = center as f32 + 0.5 - w / 2.0;
            let ty = center as f32 + 0.5 - h / 2.0;
            let pp = PixmapPaint { opacity: c.image_opacity.clamp(0.0, 1.0), ..PixmapPaint::default() };
            let t = Transform::from_scale(img_scale * x_factor, img_scale).post_translate(tx, ty);
            pm.draw_pixmap(0, 0, src.as_ref(), &pp, t, None);
        }
        return finish(pm);
    }

    let outline = paint(0, 0, 0, c.outline.opacity, false);
    let ring = |pm: &mut Pixmap, width: f32, p: &Paint| {
        let cx = center as f32 + 0.5;
        let rect = Rect::from_xywh(cx - ring_r * x_factor, cx - ring_r, 2.0 * ring_r * x_factor, 2.0 * ring_r);
        if let Some(path) = rect.and_then(PathBuilder::from_oval) {
            let stroke = Stroke { width, ..Stroke::default() };
            pm.stroke_path(&path, p, &stroke, Transform::identity(), None);
        }
    };
    let dot_rect = (center - (px_round(c.dot.size * scale * x_factor).max(1)) / 2, center - dot / 2,
                    px_round(c.dot.size * scale * x_factor).max(1), dot);

    // Outlines first, under every shape.
    if o > 0 {
        for (_, a) in &line_sets {
            for rect in arm_rects(a, center, c.t_style) {
                fill(&mut pm, grow(rect, o_x, o), &outline);
            }
        }
        if c.dot.enabled {
            fill(&mut pm, grow(dot_rect, o_x, o), &outline);
        }
        if c.circle.enabled {
            ring(&mut pm, ring_w + 2.0 * o as f32, &paint(0, 0, 0, c.outline.opacity, true));
        }
    }
    for (lines, a) in &line_sets {
        let p = paint(r, g, b, lines.opacity, false);
        for rect in arm_rects(a, center, c.t_style) {
            fill(&mut pm, rect, &p);
        }
    }
    if c.circle.enabled {
        ring(&mut pm, ring_w, &paint(r, g, b, c.circle.opacity, true));
    }
    if c.dot.enabled {
        fill(&mut pm, dot_rect, &paint(r, g, b, c.dot.opacity, false));
    }
    finish(pm)
}

fn premultiplied_pixmap(img: &RgbaImage) -> Option<Pixmap> {
    let mut data = img.pixels.clone();
    for p in data.chunks_exact_mut(4) {
        let a = p[3] as u32;
        for ch in &mut p[..3] {
            *ch = ((*ch as u32 * a + 127) / 255) as u8;
        }
    }
    Pixmap::from_vec(data, tiny_skia::IntSize::from_wh(img.width, img.height)?)
}

fn finish(pm: Pixmap) -> RgbaImage {
    RgbaImage { width: pm.width(), height: pm.height(), pixels: pm.take() }
}

/// PNG preview (straight alpha) for the UI.
pub fn render_png(c: &Crosshair, scale: f32, image_dir: &Path) -> Vec<u8> {
    let mut img = render(c, scale, 1.0, image_dir);
    unpremultiply(&mut img.pixels);
    encode_png(&img.pixels, img.width, img.height).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::crosshair::model::{Circle, Dot, Lines};

    /// Fixed shapes for geometry tests (independent of the shipped presets).
    fn preset(name: &str) -> Crosshair {
        let base = |color: &str| Crosshair { color: color.into(), ..Default::default() };
        match name {
            "Classique" => Crosshair { inner: Lines { length: 6.0, thickness: 2.0, gap: 3.0, ..Lines::default() }, ..base("#00FF00") },
            "T" => Crosshair { inner: Lines { length: 7.0, thickness: 2.0, gap: 4.0, ..Lines::default() }, t_style: true, ..base("#00FFFF") },
            "Point" => Crosshair {
                inner: Lines { enabled: false, ..Lines::default() },
                dot: Dot { enabled: true, size: 4.0, opacity: 1.0 },
                ..base("#FFFFFF")
            },
            "Cercle" => Crosshair {
                inner: Lines { enabled: false, ..Lines::default() },
                dot: Dot { enabled: true, size: 3.0, opacity: 1.0 },
                circle: Circle { enabled: true, radius: 14.0, thickness: 2.0, opacity: 1.0 },
                ..base("#FF4FD8")
            },
            _ => panic!("unknown fixture {name}"),
        }
    }

    /// Straight-alpha RGBA of pixel (x, y).
    fn px(img: &RgbaImage, x: u32, y: u32) -> [u8; 4] {
        let i = ((y * img.width + x) * 4) as usize;
        let p = &img.pixels[i..i + 4];
        [p[0], p[1], p[2], p[3]]
    }

    fn center(img: &RgbaImage) -> (u32, u32) {
        (img.width / 2, img.height / 2)
    }

    fn none() -> &'static Path {
        Path::new("")
    }

    #[test]
    fn canvas_is_odd_and_centered() {
        let img = render(&preset("Classique"), 1.0, 1.0, none());
        assert_eq!(img.width % 2, 1);
        assert_eq!(img.width, img.height);
        assert_eq!(img.pixels.len(), (img.width * img.height * 4) as usize);
    }

    #[test]
    fn center_is_transparent_with_gap() {
        let img = render(&preset("Classique"), 1.0, 1.0, none());
        let (cx, cy) = center(&img);
        assert_eq!(px(&img, cx, cy)[3], 0);
    }

    #[test]
    fn right_arm_pixel_is_green() {
        let img = render(&preset("Classique"), 1.0, 1.0, none());
        let (cx, cy) = center(&img);
        assert_eq!(px(&img, cx + 4, cy), [0, 255, 0, 255]);
    }

    #[test]
    fn outline_pixel_is_black() {
        let img = render(&preset("Classique"), 1.0, 1.0, none());
        let (cx, cy) = center(&img);
        assert_eq!(px(&img, cx + 3 + 6, cy), [0, 0, 0, 255]);
    }

    #[test]
    fn t_style_has_no_top_arm() {
        let img = render(&preset("T"), 1.0, 1.0, none());
        let (cx, cy) = center(&img);
        assert_eq!(px(&img, cx, cy - 6)[3], 0, "top arm must be absent");
        assert_eq!(px(&img, cx, cy + 6), [0, 255, 255, 255], "bottom arm present");
    }

    #[test]
    fn dot_center_is_white() {
        let img = render(&preset("Point"), 1.0, 1.0, none());
        let (cx, cy) = center(&img);
        assert_eq!(px(&img, cx, cy), [255, 255, 255, 255]);
    }

    #[test]
    fn x_factor_shrinks_horizontal_extent() {
        let img = render(&preset("Classique"), 1.0, 0.5, none());
        let (cx, cy) = center(&img);
        assert_eq!(px(&img, cx + 6, cy)[3], 0, "right arm shortened");
        assert_eq!(px(&img, cx, cy + 6), [0, 255, 0, 255], "vertical arm unchanged");
    }

    #[test]
    fn scale_doubles_distances() {
        let img = render(&preset("Classique"), 2.0, 1.0, none());
        let (cx, cy) = center(&img);
        assert_eq!(px(&img, cx + 8, cy), [0, 255, 0, 255]);
        assert_eq!(px(&img, cx + 3, cy)[3], 0, "inside the doubled gap (outline starts at +4)");
    }

    #[test]
    fn circle_ring_is_drawn() {
        let img = render(&preset("Cercle"), 1.0, 1.0, none());
        let (cx, cy) = center(&img);
        let ring = px(&img, cx + 14, cy);
        assert!(ring[3] > 0, "ring pixel should be painted, got {ring:?}");
        assert_eq!(px(&img, cx + 8, cy)[3], 0, "inside the ring is empty");
    }

    #[test]
    fn hostile_crosshair_renders_a_bounded_canvas() {
        let mut c = preset("Classique");
        c.inner.length = 1e30;
        c.circle.enabled = true;
        c.circle.radius = f32::INFINITY;
        let img = render(&c, 4.0, 1.0, none());
        assert!(img.width <= MAX_CANVAS && img.height <= MAX_CANVAS);
    }

    #[test]
    fn render_png_is_a_png() {
        let png = render_png(&preset("Classique"), 2.0, none());
        assert_eq!(&png[..4], &[0x89, b'P', b'N', b'G']);
    }
}
