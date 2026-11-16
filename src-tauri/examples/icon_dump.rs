//! Manual check: writes the icon of an executable to a PNG file.
fn main() {
    let a: Vec<String> = std::env::args().collect();
    match rez_lib::win::icon::exe_icon_png(&a[1], 256) {
        Ok(png) => {
            std::fs::write(&a[2], &png).unwrap();
            println!("ok {} bytes", png.len());
        }
        Err(e) => println!("error: {e}"),
    }
}
