//! Diagnostic: applies a temporary mode on one device and prints a timeline of the
//! attached-monitor topology every 250 ms, during the hold and after a normal restore.
//! Usage: cds_probe <device> <width> <height> <hint|nohint> <hold_secs>
use std::time::{Duration, Instant};
use windows::core::PCWSTR;
use windows::Win32::Graphics::Gdi::{
    ChangeDisplaySettingsExW, CDS_FULLSCREEN, CDS_TYPE, DEVMODEW, DMDFO_STRETCH, DM_DISPLAYFIXEDOUTPUT,
    DM_DISPLAYFREQUENCY, DM_PELSHEIGHT, DM_PELSWIDTH,
};
use rez_lib::display::DisplayApi;
use rez_lib::win::display::Win32Display;

fn snapshot(d: &Win32Display) -> String {
    d.list_monitors()
        .unwrap_or_default()
        .iter()
        .map(|m| {
            let cur = d.current_mode(&m.device).map(|c| format!("{}x{}@{}", c.width, c.height, c.refresh_hz));
            format!("{}{}={}", m.device.trim_start_matches(r"\\.\"), if m.primary { "*" } else { "" }, cur.unwrap_or_else(|e| e.to_string()))
        })
        .collect::<Vec<_>>()
        .join("  ")
}

fn timeline(d: &Win32Display, t0: Instant, secs: u64) {
    let end = Instant::now() + Duration::from_secs(secs);
    let mut last = String::new();
    while Instant::now() < end {
        let s = snapshot(d);
        if s != last {
            println!("  t={:>5}ms  {s}", t0.elapsed().as_millis());
            last = s;
        }
        std::thread::sleep(Duration::from_millis(250));
    }
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let (device, w, h, hint, hold) = (&a[1], a[2].parse().unwrap(), a[3].parse().unwrap(), a[4] == "hint", a[5].parse().unwrap());
    let d = Win32Display::new();
    let native = d.registry_mode(device).expect("registry_mode");
    let name: Vec<u16> = device.encode_utf16().chain(Some(0)).collect();
    let mut dm = DEVMODEW { dmSize: std::mem::size_of::<DEVMODEW>() as u16, ..Default::default() };
    dm.dmPelsWidth = w;
    dm.dmPelsHeight = h;
    dm.dmDisplayFrequency = native.refresh_hz;
    dm.dmFields = DM_PELSWIDTH | DM_PELSHEIGHT | DM_DISPLAYFREQUENCY;
    if hint {
        dm.dmFields |= DM_DISPLAYFIXEDOUTPUT;
        dm.Anonymous1.Anonymous2.dmDisplayFixedOutput = DMDFO_STRETCH;
    }
    let t0 = Instant::now();
    println!("before: {}", snapshot(&d));
    let r = unsafe { ChangeDisplaySettingsExW(PCWSTR(name.as_ptr()), Some(&dm), None, CDS_FULLSCREEN, None) };
    println!("apply {w}x{h}@{} hint={hint} -> code {} after {}ms", native.refresh_hz, r.0, t0.elapsed().as_millis());
    timeline(&d, t0, hold);
    let r = unsafe { ChangeDisplaySettingsExW(PCWSTR(name.as_ptr()), None, None, CDS_TYPE(0), None) };
    println!("restore -> code {} at {}ms", r.0, t0.elapsed().as_millis());
    timeline(&d, t0, 6);
    println!("after: {}", snapshot(&d));
}
