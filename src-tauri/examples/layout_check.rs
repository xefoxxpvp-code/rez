//! Manual check: capture layout, switch the primary monitor to 1280x960 for 3 s, restore mode, restore layout.
use rez_lib::display::{pick_mode, DisplayApi};
use rez_lib::runtime::LayoutApi;
use rez_lib::win::{desktop_layout::Win32Layout, display::Win32Display};

fn main() {
    let d = Win32Display::new();
    let primary = d.list_monitors().unwrap().into_iter().find(|m| m.primary).unwrap();
    let native = d.registry_mode(&primary.device).unwrap();
    let mode = pick_mode(&d.list_modes(&primary.device).unwrap(), 1280, 960, native.refresh_hz).expect("1280x960");
    let before = rez_lib::win::desktop_layout::icon_positions();
    let mut layout = Win32Layout::new(std::env::temp_dir().join("rez-icons-check.json"));
    layout.capture(&primary.device);
    d.apply_temporary(&primary.device, mode).unwrap();
    std::thread::sleep(std::time::Duration::from_secs(3));
    let during = rez_lib::win::desktop_layout::icon_positions();
    d.restore(&primary.device).unwrap();
    layout.restore(&primary.device);
    std::thread::sleep(std::time::Duration::from_secs(2)); // let the delayed restore thread run
    let after = rez_lib::win::desktop_layout::icon_positions();
    let moved_during = before.iter().zip(&during).filter(|(a, b)| a != b).count();
    let moved_after = before.iter().zip(&after).filter(|(a, b)| a != b).count();
    println!("icons: {} | moved while stretched: {moved_during} | still moved after restore: {moved_after}", before.len());
}
