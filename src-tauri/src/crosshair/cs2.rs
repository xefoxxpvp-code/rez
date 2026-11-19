//! Counter-Strike 2 crosshair share codes: legacy `CSGO-xxxxx-…` (v1) and the current
//! `CS…` pixel format (cs2-v1). Decoding follows the MIT-licensed `csgo-sharecode`
//! (akiver) and the community pixel model (SIC, MIT).

use super::import::Imported;
use super::model::{Crosshair, Dot, Lines, Outline};

const DICTIONARY: &str = "ABCDEFGHJKLMNOPQRSTUVWXYZabcdefhijkmnopqrstuvwxyz23456789";
/// cl_crosshaircolor 0..4 (5 = custom RGB).
const PRESET_COLORS: [&str; 5] = ["#FA3232", "#32FA32", "#FAFA32", "#3232FA", "#32FAFA"];

/// Base-57 decoding of a share code into its checksummed bytes.
pub fn decode_bytes(code: &str) -> Result<Vec<u8>, String> {
    let code = code.trim();
    let (body, size, chars) = if let Some(rest) = code.strip_prefix("CSGO-") {
        (rest.replace('-', ""), 18usize, 25usize)
    } else if let Some(rest) = code.strip_prefix("CS") {
        (rest.to_string(), 32, 44)
    } else {
        return Err("Code CS2 invalide".into());
    };
    if body.chars().count() != chars {
        return Err("Code CS2 incomplet".into());
    }
    let mut big = vec![0u8; size];
    for ch in body.chars().rev() {
        let idx = DICTIONARY.find(ch).ok_or("Code CS2 invalide : caractère inconnu")? as u32;
        let mut carry = idx;
        for byte in big.iter_mut().rev() {
            let v = *byte as u32 * 57 + carry;
            *byte = (v & 0xFF) as u8;
            carry = v >> 8;
        }
        if carry != 0 {
            return Err("Code CS2 invalide".into());
        }
    }
    let sum = big[1..].iter().fold(0u8, |acc, b| acc.wrapping_add(*b));
    if big[0] != sum {
        return Err("Code CS2 invalide (somme de contrôle)".into());
    }
    Ok(big)
}

fn hex(r: u8, g: u8, b: u8) -> String {
    format!("#{r:02X}{g:02X}{b:02X}")
}

pub fn parse(code: &str) -> Result<Imported, String> {
    let b = decode_bytes(code)?;
    match (b.len(), b[1]) {
        (18, 1) => Ok(legacy_v1(&b)),
        (18, 3) | (18, 4) => Err("Ancien format CS2 non pris en charge : réexporte ton crosshair depuis CS2 (code CS…).".into()),
        (32, 1) => cs2_v1(&b),
        _ => Err("Version de code CS2 inconnue".into()),
    }
}

/// `CSGO-` v1: sizes in CS:GO units (scale height/480), gap in pixels with a +4 base.
fn legacy_v1(b: &[u8]) -> Imported {
    let gap = (b[2] as i8) as f32 / 10.0;
    let outline_thickness = b[3] as f32 / 2.0;
    let (r, g, bl, a) = (b[4], b[5], b[6], b[7]);
    let color_index = (b[10] & 0x07) as usize;
    let draw_outline = b[10] & 0x08 != 0;
    let thickness = b[12] as f32 / 10.0;
    let style = (b[13] & 0x0E) >> 1;
    let dot = b[13] & 0x10 != 0;
    let alpha_enabled = b[13] & 0x40 != 0;
    let t_style = b[13] & 0x80 != 0;
    let length = (b[14] as u32 | ((b[15] & 0x1F) as u32) << 8) as f32 / 10.0;

    let s = 1080.0 / 480.0;
    let w = (thickness * s).round_ties_even().max(1.0);
    let l = (length * s).round_ties_even();
    let d = (gap + 4.0).trunc();
    let opacity = if alpha_enabled { a as f32 / 255.0 } else { 1.0 };
    let color = PRESET_COLORS.get(color_index).map(|c| c.to_string()).unwrap_or_else(|| hex(r, g, bl));

    let mut notes = Vec::new();
    if !matches!(style, 1 | 4) {
        notes.push("Style dynamique : REZ affiche la version fixe.".to_string());
    }
    let crosshair = Crosshair {
        name: "Import CS2".into(),
        color,
        inner: Lines { enabled: l > 0.0, length: l, vertical_length: None, thickness: w, gap: (w / 2.0).floor() + d, opacity },
        t_style,
        outline: Outline { enabled: draw_outline, thickness: outline_thickness, opacity: 1.0 },
        dot: Dot { enabled: dot, size: w, opacity },
        ..Crosshair::default()
    };
    Imported { crosshair, notes }
}

/// `CS…` v1: pixel values at the screen height stored in the code.
fn cs2_v1(b: &[u8]) -> Result<Imported, String> {
    let height = u16::from_le_bytes([b[2], b[3]]);
    if height == 0 {
        return Err("Code CS2 invalide (hauteur d'écran nulle)".into());
    }
    let r = 1080.0 / height.max(240) as f32;
    let dot = b[4] & 0x40 != 0;
    let t_style = b[4] & 0x80 != 0;
    let (red, green, blue, alpha) = (b[5], b[6], b[7], b[8]);
    let (or, og, ob, oa) = (b[9], b[10], b[11], b[12]);
    let thickness = (b[13] & 0x3F) as f32;
    let outline_mode = b[13] >> 6;
    let gap = i16::from_le_bytes([b[14], b[15]]).clamp(-3840, 3840) as f32;
    let length = b[16] as f32;
    let opacity = alpha as f32 / 255.0;

    let mut notes = Vec::new();
    if outline_mode == 2 {
        notes.push("Contour partiel (haut/gauche) affiché en contour complet.".to_string());
    }
    if outline_mode != 0 && (or, og, ob) != (0, 0, 0) {
        notes.push("Contour de couleur affiché en noir.".to_string());
    }
    let crosshair = Crosshair {
        name: "Import CS2".into(),
        color: hex(red, green, blue),
        inner: Lines { enabled: length > 0.0, length: length * r, vertical_length: None, thickness: thickness * r, gap: gap * r, opacity },
        t_style,
        outline: Outline { enabled: outline_mode != 0, thickness: 1.0, opacity: oa as f32 / 255.0 },
        dot: Dot { enabled: dot, size: thickness * r, opacity },
        ..Crosshair::default()
    };
    Ok(Imported { crosshair, notes })
}

#[cfg(test)]
mod tests {
    use super::*;

    const LEGACY_1: &str = "CSGO-Cn37R-YE7vo-pLCAL-aURmZ-z6zkG";
    const LEGACY_2: &str = "CSGO-LibdP-VCVEd-ESayK-rSivi-2UBtG";
    const CS2_1: &str = "CSvbPubOq37zTGqtsPTP5QTrp5CB4xFXiKRLfzJsm49ZRe";
    const CS2_2: &str = "CSAB6Oby2JUDOrBRy4jRcsR6JOQdhbszKF9GuKQGhKVwAc";

    #[test]
    fn decodes_legacy_bytes() {
        assert_eq!(
            decode_bytes(LEGACY_1).unwrap(),
            vec![0x69, 0x01, 0xF3, 0x04, 0xAF, 0x51, 0xD5, 0x89, 0x06, 0x1E, 0x6D, 0x54, 0x0C, 0xF4, 0x2E, 0, 0, 0]
        );
    }

    #[test]
    fn legacy_v1_maps_to_pixels_at_1080p() {
        let c = parse(LEGACY_1).unwrap().crosshair;
        assert_eq!(c.color, "#AF51D5");
        assert!(c.t_style && c.dot.enabled && c.outline.enabled);
        assert_eq!(c.outline.thickness, 2.0);
        // W = roundEven(1.2*2.25)=3, L = roundEven(4.6*2.25)=10, gap = W/2 + trunc(-1.3+4) = 1 + 2
        assert_eq!((c.inner.thickness, c.inner.length, c.inner.gap), (3.0, 10.0, 3.0));
        assert_eq!(c.dot.size, 3.0);
        assert!((c.inner.opacity - 137.0 / 255.0).abs() < 1e-3);
    }

    #[test]
    fn legacy_v1_preset_color_and_flags_off() {
        let c = parse(LEGACY_2).unwrap().crosshair;
        assert_eq!(c.color, "#32FA32");
        assert!(!c.t_style && !c.dot.enabled && !c.outline.enabled);
        assert_eq!((c.inner.thickness, c.inner.length, c.inner.gap), (1.0, 11.0, 5.0));
        assert_eq!(c.inner.opacity, 1.0);
    }

    #[test]
    fn cs2_v1_scales_from_its_screen_height() {
        let c = parse(CS2_1).unwrap().crosshair;
        assert_eq!(c.color, "#FF0000");
        assert!(c.dot.enabled && !c.outline.enabled && !c.t_style);
        let r = 1080.0 / 768.0;
        assert!((c.inner.thickness - r).abs() < 1e-4);
        assert!((c.inner.length - 5.0 * r).abs() < 1e-4);
        assert_eq!(c.inner.gap, 0.0);
    }

    #[test]
    fn cs2_v1_negative_gap_t_style_and_half_outline() {
        let imported = parse(CS2_2).unwrap();
        let c = imported.crosshair;
        assert_eq!(c.color, "#0A141E");
        assert!(c.t_style && c.outline.enabled);
        assert!((c.inner.gap - (-7.0 * 0.75)).abs() < 1e-4);
        assert!((c.inner.thickness - 9.0).abs() < 1e-4);
        assert!((c.inner.length - 255.0 * 0.75).abs() < 1e-3);
        assert!((c.inner.opacity - 200.0 / 255.0).abs() < 1e-3);
        assert!(!imported.notes.is_empty(), "half outline and dynamic style deserve a note");
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse("CSGO-xxxxx").is_err());
        assert!(parse("CSGO-Cn37R-YE7vo-pLCAL-aURmZ-z6zkH").is_err(), "checksum must fail");
        assert!(parse("CSGO-Cn37R-YE7vo-pLCAL-aURmZ-z6zk0").is_err(), "0 is not in the dictionary");
        assert!(parse("hello").is_err());
    }
}
