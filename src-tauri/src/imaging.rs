//! Small image helpers: PNG encode/decode and alpha conversions.

/// RGBA pixels, row-major. Whether alpha is premultiplied is stated by the producer.
#[derive(Clone, Debug, PartialEq)]
pub struct RgbaImage {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

/// Encodes straight-alpha RGBA as PNG.
pub fn encode_png(rgba: &[u8], width: u32, height: u32) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
        writer.write_image_data(rgba).map_err(|e| e.to_string())?;
    }
    Ok(out)
}

/// Images wider or taller than this are refused before any pixel buffer is allocated.
pub const MAX_DECODE_SIDE: u32 = 4096;

/// Decodes any 8-bit PNG to straight-alpha RGBA.
pub fn decode_png(bytes: &[u8]) -> Result<RgbaImage, String> {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().map_err(|e| format!("image illisible : {e}"))?;
    let header = reader.info();
    if header.width > MAX_DECODE_SIDE || header.height > MAX_DECODE_SIDE {
        return Err(format!("image trop grande (max {MAX_DECODE_SIDE} px de côté)"));
    }
    let mut buf = vec![0u8; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).map_err(|e| format!("image illisible : {e}"))?;
    let data = &buf[..info.buffer_size()];
    let pixels: Vec<u8> = match info.color_type {
        png::ColorType::Rgba => data.to_vec(),
        png::ColorType::Rgb => data.as_chunks::<3>().0.iter().flat_map(|p| [p[0], p[1], p[2], 255]).collect(),
        png::ColorType::GrayscaleAlpha => data.as_chunks::<2>().0.iter().flat_map(|p| [p[0], p[0], p[0], p[1]]).collect(),
        png::ColorType::Grayscale => data.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        png::ColorType::Indexed => return Err("image illisible : palette non convertie".into()),
    };
    Ok(RgbaImage { width: info.width, height: info.height, pixels })
}

/// Premultiplied RGBA → straight RGBA, in place.
pub fn unpremultiply(px: &mut [u8]) {
    for p in px.as_chunks_mut::<4>().0 {
        let a = p[3] as u32;
        if a > 0 && a < 255 {
            for c in &mut p[..3] {
                *c = ((*c as u32 * 255 + a / 2) / a).min(255) as u8;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn png_roundtrip() {
        let pixels = vec![255, 0, 0, 255, 0, 0, 255, 128];
        let png = encode_png(&pixels, 2, 1).unwrap();
        let img = decode_png(&png).unwrap();
        assert_eq!((img.width, img.height), (2, 1));
        assert_eq!(img.pixels, pixels);
    }

    #[test]
    fn unpremultiply_restores_color() {
        let mut px = vec![64, 0, 0, 128];
        unpremultiply(&mut px);
        assert_eq!(px, vec![128, 0, 0, 128]); // 64*255/128 = 127.5, rounded to nearest
    }

    #[test]
    fn decode_rejects_oversized_images_before_allocating() {
        let wide = vec![0u8; (MAX_DECODE_SIDE as usize + 1) * 4];
        let png = encode_png(&wide, MAX_DECODE_SIDE + 1, 1).unwrap();
        assert!(decode_png(&png).is_err());
    }

    #[test]
    fn decode_rejects_garbage() {
        assert!(decode_png(b"not a png").is_err());
    }
}
