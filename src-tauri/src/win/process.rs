//! Running-process checks (Toolhelp snapshot; reads only process names).

use windows::Win32::Foundation::CloseHandle;
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};

/// True if a process whose exe name (lowercase) is in `names_lower` is running.
pub fn is_any_running(names_lower: &[&str]) -> bool {
    unsafe {
        let Ok(snapshot) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else { return false };
        let mut entry = PROCESSENTRY32W { dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32, ..Default::default() };
        let mut found = false;
        let mut ok = Process32FirstW(snapshot, &mut entry).is_ok();
        while ok {
            let len = entry.szExeFile.iter().position(|&c| c == 0).unwrap_or(entry.szExeFile.len());
            let name = String::from_utf16_lossy(&entry.szExeFile[..len]).to_lowercase();
            if names_lower.contains(&name.as_str()) {
                found = true;
                break;
            }
            ok = Process32NextW(snapshot, &mut entry).is_ok();
        }
        let _ = CloseHandle(snapshot);
        found
    }
}

/// FACEIT client / anti-cheat processes.
pub const FACEIT_PROCESSES: [&str; 3] = ["faceit.exe", "faceitclient.exe", "faceitservice.exe"];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_this_test_process_and_not_a_fake_one() {
        let me = std::env::current_exe().unwrap();
        let name = me.file_name().unwrap().to_string_lossy().to_lowercase();
        assert!(is_any_running(&[name.as_str()]));
        assert!(!is_any_running(&["definitely-not-running-rez-xyz.exe"]));
    }
}
