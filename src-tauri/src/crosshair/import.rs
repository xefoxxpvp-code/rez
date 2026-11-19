//! Entry points for importing crosshairs: share codes (auto-detected) and PNG images.

use std::path::Path;

use serde::Serialize;

use super::model::Crosshair;
use super::{cs2, share, valorant};
use crate::imaging::{decode_png, encode_png, RgbaImage};

/// An imported crosshair (not saved yet) plus what could not be reproduced.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Imported {
    pub crosshair: Crosshair,
    pub notes: Vec<String>,
}

/// Detects the code format (REZ, CS2 legacy/current, Valorant) and imports it.
pub fn import_code(code: &str) -> Result<Imported, String> {
    let code = code.trim();
    if code.starts_with("REZ1-") {
        let mut crosshair = share::decode(code)?;
        if crosshair.name.is_empty() {
            crosshair.name = "Import REZ".into();
        }
        return Ok(Imported { crosshair, notes: Vec::new() });
    }
    if code.starts_with("CSGO-") || (code.starts_with("CS") && code.len() == 46) {
        return cs2::parse(code);
    }
    if code.starts_with("0;") {
        return valorant::parse(code);
    }
    Err("Code non reconnu : colle un code Valorant (0;P;…), CS2 (CS… ou CSGO-…) ou REZ (REZ1-…).".into())
}

const MAX_IMAGE_SIDE: u32 = 256;
/// Larger files are refused before decoding (a crosshair image never needs more).
pub const MAX_IMAGE_BYTES: usize = 16 * 1024 * 1024;

fn downscale(img: &RgbaImage) -> RgbaImage {
    let side = img.width.max(img.height);
    if side <= MAX_IMAGE_SIDE {
        return img.clone();
    }
    // u64 arithmetic: no overflow even for the largest accepted images.
    let (iw, ih, max) = (img.width as u64, img.height as u64, MAX_IMAGE_SIDE as u64);
    let w = (iw * max / side as u64).max(1);
    let h = (ih * max / side as u64).max(1);
    let mut pixels = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let sx = x * iw / w;
            let sy = y * ih / h;
            let i = ((sy * iw + sx) * 4) as usize;
            pixels.extend_from_slice(&img.pixels[i..i + 4]);
        }
    }
    RgbaImage { width: w as u32, height: h as u32, pixels }
}

/// Validates a PNG, shrinks it to at most 256 px, stores it in `dir`; returns the file name.
pub fn import_png(bytes: &[u8], dir: &Path) -> Result<String, String> {
    if bytes.len() > MAX_IMAGE_BYTES {
        return Err("Image trop lourde (16 Mo maximum).".into());
    }
    let img = decode_png(bytes)?;
    if img.width == 0 || img.height == 0 {
        return Err("image vide".into());
    }
    let img = downscale(&img);
    let png = encode_png(&img.pixels, img.width, img.height)?;
    let ms = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0);
    let name = format!("img-{ms}.png");
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    std::fs::write(dir.join(&name), png).map_err(|e| format!("image non enregistrée : {e}"))?;
    Ok(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crosshair::presets::presets;

    #[test]
    fn detects_each_code_format() {
        assert!(import_code("0;P;c;5;h;0;0l;4;0o;2;0a;1;0f;0;1b;0").is_ok());
        assert!(import_code("CSGO-Cn37R-YE7vo-pLCAL-aURmZ-z6zkG").is_ok());
        assert!(import_code("  CSvbPubOq37zTGqtsPTP5QTrp5CB4xFXiKRLfzJsm49ZRe  ").is_ok(), "trimmed");
        assert!(import_code(&crate::crosshair::share::encode(&presets()[1])).is_ok());
        let err = import_code("bonjour").unwrap_err();
        assert!(err.contains("non reconnu"));
    }

    #[test]
    fn png_import_rejects_corrupt() {
        let dir = tempfile::tempdir().unwrap();
        assert!(import_png(b"not a png", dir.path()).is_err());
    }

    #[test]
    fn png_import_rejects_heavy_files() {
        let dir = tempfile::tempdir().unwrap();
        let err = import_png(&vec![0u8; MAX_IMAGE_BYTES + 1], dir.path()).unwrap_err();
        assert!(err.contains("lourde"), "{err}");
    }

    #[test]
    fn png_import_downscales_over_256() {
        let dir = tempfile::tempdir().unwrap();
        let big = vec![255u8; 300 * 300 * 4];
        let png = crate::imaging::encode_png(&big, 300, 300).unwrap();
        let name = import_png(&png, dir.path()).unwrap();
        let stored = crate::imaging::decode_png(&std::fs::read(dir.path().join(&name)).unwrap()).unwrap();
        assert!(stored.width <= 256 && stored.height <= 256);
        assert!(name.ends_with(".png"));
    }
}
