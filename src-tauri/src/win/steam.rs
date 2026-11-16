//! Locates the Steam client folder(s) on this machine.

use std::path::PathBuf;

use windows::core::w;
use windows::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_SZ};

/// Candidate Steam roots: `HKCU\Software\Valve\Steam\SteamPath`, then the default folder.
pub fn steam_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    let mut buf = [0u16; 1024];
    let mut size = (buf.len() * 2) as u32;
    let read = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            w!(r"Software\Valve\Steam"),
            w!("SteamPath"),
            RRF_RT_REG_SZ,
            None,
            Some(buf.as_mut_ptr() as *mut _),
            Some(&mut size),
        )
    };
    if read.is_ok() {
        let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        let path = String::from_utf16_lossy(&buf[..len]).replace('/', "\\");
        if !path.is_empty() {
            roots.push(PathBuf::from(path));
        }
    }
    roots.push(PathBuf::from(r"C:\Program Files (x86)\Steam"));
    roots
}
