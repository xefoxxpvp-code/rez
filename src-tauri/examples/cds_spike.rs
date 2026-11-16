//! Spike: does Windows restore a CDS_FULLSCREEN mode when the process that set it is killed?
//! Usage: run it, then `taskkill /F /PID <pid>` from another terminal and watch the screen.
use rez_lib::display::{default_resolution, pick_mode, DisplayApi};
use rez_lib::win::display::Win32Display;

fn main() {
    let d = Win32Display::new();
    let primary = d
        .list_monitors()
        .expect("list_monitors")
        .into_iter()
        .find(|m| m.primary)
        .expect("no primary monitor");
    let native = d.registry_mode(&primary.device).expect("registry_mode");
    let modes = d.list_modes(&primary.device).expect("list_modes");
    let (w, h) = default_resolution(&modes, native);
    let mode = pick_mode(&modes, w, h, native.refresh_hz).expect("no matching mode");
    d.apply_temporary(&primary.device, mode).expect("apply_temporary");
    println!("applied {mode:?} on {} (native {native:?})", primary.device);
    println!("PID={}, run: taskkill /F /PID {}", std::process::id(), std::process::id());
    std::thread::sleep(std::time::Duration::from_secs(120));
    d.restore(&primary.device).ok();
}
