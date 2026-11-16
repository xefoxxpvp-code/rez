//! Manual check: prints the Steam artwork found for an executable.
fn main() {
    let exe = std::env::args().nth(1).expect("exe path");
    let roots = rez_lib::win::steam::steam_roots();
    println!("steam roots: {roots:?}");
    println!("{:?}", rez_lib::art::steam_art_for_exe(&exe, &roots));
}
