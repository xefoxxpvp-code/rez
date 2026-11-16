//! Manual check (run only with the user's OK, not while they play): shows the Classique preset
//! over the foreground window if it belongs to `pid`, for 10 seconds.
//! Usage: overlay_check <pid>
use rez_lib::crosshair::director::{OverlayCommand, OverlaySpec};
use rez_lib::crosshair::presets::presets;
use rez_lib::runtime::OverlayApi;
use rez_lib::win::overlay::Win32Overlay;

fn main() {
    let pid: u32 = std::env::args().nth(1).expect("pid").parse().expect("pid number");
    let mut overlay = Win32Overlay::spawn(std::env::temp_dir());
    let spec = OverlaySpec { pid, crosshair: presets()[0].clone(), native: (1920, 1080), undistort: true };
    overlay.apply(OverlayCommand::Show(Box::new(spec)));
    std::thread::sleep(std::time::Duration::from_secs(10));
    overlay.apply(OverlayCommand::Hide);
    std::thread::sleep(std::time::Duration::from_millis(300));
}
