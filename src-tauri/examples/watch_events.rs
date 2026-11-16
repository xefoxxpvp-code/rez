//! Manual check: prints every Input produced by the Win32 event sources.
use std::sync::mpsc;

fn main() {
    let (tx, rx) = mpsc::channel();
    rez_lib::win::events::start(tx.clone()).expect("start events");
    println!("watching... switch windows, lock/unlock the session (Ctrl+C to quit)");
    for input in rx {
        if let rez_lib::runtime::Input::Foreground { pid, .. } = &input {
            rez_lib::win::events::watch_process(*pid, tx.clone());
        }
        println!("{input:?}");
    }
}
