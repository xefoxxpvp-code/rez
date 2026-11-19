//! Writes PNG previews of the five presets (scale 4, on a dark background) to a folder.
use rez_lib::crosshair::{presets::presets, render::render};

fn main() {
    let out = std::path::PathBuf::from(std::env::args().nth(1).expect("output dir"));
    for c in presets() {
        let img = render(&c, 4.0, 1.0, std::path::Path::new(""));
        // Composite premultiplied pixels over a dark grey game-like background.
        let bg = [40u32, 44, 52];
        let mut rgba = Vec::with_capacity(img.pixels.len());
        for p in img.pixels.as_chunks::<4>().0 {
            let a = p[3] as u32;
            for i in 0..3 {
                rgba.push((p[i] as u32 + bg[i] * (255 - a) / 255).min(255) as u8);
            }
            rgba.push(255);
        }
        let png = rez_lib::imaging::encode_png(&rgba, img.width, img.height).unwrap();
        std::fs::write(out.join(format!("{}.png", c.id)), png).unwrap();
    }
    println!("ok");
}
