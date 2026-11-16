//! Manual check: prints every attached monitor, its registry mode and its distinct resolutions.
use rez_lib::display::{distinct_resolutions, DisplayApi};
use rez_lib::win::display::Win32Display;

fn main() {
    let d = Win32Display::new();
    println!("save_topology (read-only query): {:?}", d.save_topology());
    for mon in d.list_monitors().expect("list_monitors") {
        println!("{} | {} | primary={}", mon.device, mon.label, mon.primary);
        println!("  registry: {:?}", d.registry_mode(&mon.device));
        println!("  current:  {:?}", d.current_mode(&mon.device));
        let modes = d.list_modes(&mon.device).unwrap_or_default();
        let res: Vec<String> = distinct_resolutions(&modes).iter().map(|(w, h)| format!("{w}x{h}")).collect();
        println!("  resolutions: {}", res.join(", "));
    }
}
