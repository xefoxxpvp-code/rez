//! Extracts a game's own icon from its executable (shell image factory, up to 256 px)
//! and encodes it as PNG, so the UI shows the real game logo.

use windows::core::HSTRING;
use windows::Win32::Foundation::SIZE;
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, DeleteDC, DeleteObject, GetDIBits, GetObjectW, BITMAP, BITMAPINFO, BITMAPINFOHEADER,
    BI_RGB, DIB_RGB_COLORS, HGDIOBJ,
};
use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};
use windows::Win32::UI::Shell::{
    IShellItemImageFactory, SHCreateItemFromParsingName, SIIGBF_BIGGERSIZEOK, SIIGBF_ICONONLY,
};

/// Converts Windows BGRA pixels to RGBA.
pub fn bgra_to_rgba(bgra: &[u8]) -> Vec<u8> {
    bgra.chunks_exact(4).flat_map(|p| [p[2], p[1], p[0], p[3]]).collect()
}

/// Crops RGBA pixels to the bounding box of non-transparent pixels. The shell centers small
/// exe icons inside a large transparent canvas; cropping lets the UI scale the real icon.
pub fn crop_to_content(rgba: &[u8], width: u32, height: u32) -> (Vec<u8>, u32, u32) {
    let (w, h) = (width as usize, height as usize);
    let (mut x0, mut y0, mut x1, mut y1) = (w, h, 0usize, 0usize);
    for y in 0..h {
        for x in 0..w {
            if rgba[(y * w + x) * 4 + 3] > 0 {
                x0 = x0.min(x);
                y0 = y0.min(y);
                x1 = x1.max(x + 1);
                y1 = y1.max(y + 1);
            }
        }
    }
    if x0 >= x1 || y0 >= y1 {
        return (rgba.to_vec(), width, height);
    }
    let mut out = Vec::with_capacity((x1 - x0) * (y1 - y0) * 4);
    for y in y0..y1 {
        out.extend_from_slice(&rgba[(y * w + x0) * 4..(y * w + x1) * 4]);
    }
    (out, (x1 - x0) as u32, (y1 - y0) as u32)
}

pub use crate::imaging::encode_png;

/// PNG of the executable's icon at up to `size` px, or an error message.
pub fn exe_icon_png(exe_path: &str, size: i32) -> Result<Vec<u8>, String> {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let factory: IShellItemImageFactory =
            SHCreateItemFromParsingName(&HSTRING::from(exe_path), None).map_err(|e| e.to_string())?;
        let hbitmap = factory
            .GetImage(SIZE { cx: size, cy: size }, SIIGBF_ICONONLY | SIIGBF_BIGGERSIZEOK)
            .map_err(|e| e.to_string())?;

        let mut bm = BITMAP::default();
        GetObjectW(HGDIOBJ(hbitmap.0), std::mem::size_of::<BITMAP>() as i32, Some(&mut bm as *mut _ as *mut _));
        let (w, h) = (bm.bmWidth, bm.bmHeight);
        let mut info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: w,
                biHeight: -h, // top-down
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bgra = vec![0u8; (w * h * 4) as usize];
        let dc = CreateCompatibleDC(None);
        let lines = GetDIBits(dc, hbitmap, 0, h as u32, Some(bgra.as_mut_ptr() as *mut _), &mut info, DIB_RGB_COLORS);
        let _ = DeleteDC(dc);
        let _ = DeleteObject(HGDIOBJ(hbitmap.0));
        if lines == 0 || w <= 0 || h <= 0 {
            return Err("icône illisible".into());
        }
        let (rgba, cw, ch) = crop_to_content(&bgra_to_rgba(&bgra), w as u32, h as u32);
        encode_png(&rgba, cw, ch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bgra_is_swapped_to_rgba() {
        let bgra = [10u8, 20, 30, 255, 1, 2, 3, 0];
        assert_eq!(bgra_to_rgba(&bgra), vec![30, 20, 10, 255, 3, 2, 1, 0]);
    }

    #[test]
    fn crop_keeps_only_opaque_bounding_box() {
        // 4x4, opaque pixels at (1,1) and (2,2) only.
        let mut rgba = vec![0u8; 4 * 4 * 4];
        for (x, y) in [(1usize, 1usize), (2, 2)] {
            rgba[(y * 4 + x) * 4 + 3] = 255;
        }
        let (out, w, h) = crop_to_content(&rgba, 4, 4);
        assert_eq!((w, h), (2, 2));
        assert_eq!(out.len(), 2 * 2 * 4);
        assert_eq!(out[3], 255); // top-left of the crop is the (1,1) pixel
    }

    #[test]
    fn crop_of_fully_transparent_image_is_unchanged() {
        let rgba = vec![0u8; 2 * 2 * 4];
        assert_eq!(crop_to_content(&rgba, 2, 2), (rgba.clone(), 2, 2));
    }

    #[test]
    fn png_has_signature_and_size() {
        let rgba = vec![255u8; 2 * 3 * 4];
        let bytes = encode_png(&rgba, 2, 3).unwrap();
        assert_eq!(&bytes[..8], &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]);
    }
}
