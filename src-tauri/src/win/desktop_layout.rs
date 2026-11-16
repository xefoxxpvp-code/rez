//! Saves and restores desktop icon positions (shell `IFolderView`) and the placement of
//! top-level windows on a monitor around a mode switch. Errors are logged, never raised:
//! a failed layout save/restore must never block a resolution switch.
//!
//! Which snapshot to restore, and when, is decided by [`LayoutBook`]; this file only does
//! the Win32 work. The icons snapshot is also written to disk while a stretch session is
//! open, so an unclean exit (crash, kill) can be repaired at the next start.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use base64::Engine as _;
use serde::{Deserialize, Serialize};
use windows::core::{Interface, Result as WinResult, BOOL};
use windows::Win32::Foundation::{HWND, LPARAM, POINT};
use windows::Win32::Graphics::Gdi::{GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITORINFOEXW, MONITOR_DEFAULTTONEAREST};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoTaskMemFree, IServiceProvider, CLSCTX_LOCAL_SERVER, COINIT_APARTMENTTHREADED,
};
use windows::Win32::System::Variant::VARIANT;
use windows::Win32::UI::Shell::Common::ITEMIDLIST;
use windows::Win32::UI::Shell::{
    IFolderView, IShellBrowser, IShellWindows, ILGetSize, ShellWindows, SID_STopLevelBrowser, SVGIO_ALLVIEW,
    SVSI_POSITIONITEM, SWC_DESKTOP, SWFO_NEEDDISPATCH,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetForegroundWindow, GetWindow, GetWindowLongW, GetWindowPlacement, GetWindowThreadProcessId, IsIconic,
    IsWindow, IsWindowVisible, SetWindowPlacement, GWL_EXSTYLE, GW_OWNER, SW_SHOW, SW_SHOWMAXIMIZED, SW_SHOWNOACTIVATE,
    SW_SHOWNORMAL, WINDOWPLACEMENT, WPF_ASYNCWINDOWPLACEMENT, WS_EX_TOOLWINDOW,
};

use crate::layout_book::LayoutBook;
use crate::runtime::LayoutApi;

/// Delay before restoring, so Explorer finishes its own rearrangement after the mode change.
const RESTORE_DELAY_MS: u64 = 800;

type Icons = Vec<(Vec<u8>, POINT)>;
type Windows = Vec<(isize, WINDOWPLACEMENT)>;
type Book = LayoutBook<Windows, Icons>;

pub struct Win32Layout {
    book: Arc<Mutex<Book>>,
    snapshot_file: PathBuf,
}

impl Win32Layout {
    /// `snapshot_file`: where the icons snapshot is kept during a stretch session.
    pub fn new(snapshot_file: PathBuf) -> Self {
        Win32Layout { book: Arc::new(Mutex::new(Book::default())), snapshot_file }
    }

    /// At startup: if a previous run ended while stretched, put its desktop icons back.
    pub fn recover_icons_from_disk(snapshot_file: &std::path::Path) {
        let Ok(json) = std::fs::read_to_string(snapshot_file) else { return };
        match decode_icons(&json) {
            Some(icons) => {
                com_init();
                match restore_icons(&icons) {
                    Ok(()) => log::info!("desktop icons recovered from an unclean exit ({} items)", icons.len()),
                    Err(e) => log::warn!("icon recovery failed: {e}"),
                }
            }
            None => log::warn!("icon snapshot unreadable, discarded"),
        }
        let _ = std::fs::remove_file(snapshot_file);
    }
}

#[derive(Serialize, Deserialize)]
struct StoredIcon {
    pidl: String,
    x: i32,
    y: i32,
}

pub fn encode_icons(icons: &[(Vec<u8>, POINT)]) -> String {
    let stored: Vec<StoredIcon> = icons
        .iter()
        .map(|(pidl, p)| StoredIcon { pidl: base64::engine::general_purpose::STANDARD.encode(pidl), x: p.x, y: p.y })
        .collect();
    serde_json::to_string(&stored).unwrap_or_else(|_| "[]".into())
}

pub fn decode_icons(json: &str) -> Option<Vec<(Vec<u8>, POINT)>> {
    let stored: Vec<StoredIcon> = serde_json::from_str(json).ok()?;
    stored
        .into_iter()
        .map(|s| {
            let pidl = base64::engine::general_purpose::STANDARD.decode(s.pidl).ok()?;
            Some((pidl, POINT { x: s.x, y: s.y }))
        })
        .collect()
}

fn desktop_folder_view() -> WinResult<IFolderView> {
    unsafe {
        let shell_windows: IShellWindows = CoCreateInstance(&ShellWindows, None, CLSCTX_LOCAL_SERVER)?;
        let location = VARIANT::from(0i32); // CSIDL_DESKTOP
        let empty = VARIANT::default();
        let mut hwnd = 0i32;
        let dispatch = shell_windows.FindWindowSW(&location, &empty, SWC_DESKTOP, &mut hwnd, SWFO_NEEDDISPATCH)?;
        let provider: IServiceProvider = dispatch.cast()?;
        let browser: IShellBrowser = provider.QueryService(&SID_STopLevelBrowser)?;
        let view = browser.QueryActiveShellView()?;
        view.cast()
    }
}

fn capture_icons() -> WinResult<Icons> {
    unsafe {
        let view = desktop_folder_view()?;
        let items: windows::Win32::UI::Shell::IEnumIDList = view.Items(SVGIO_ALLVIEW)?;
        let mut out = Vec::new();
        loop {
            let mut pidl = [std::ptr::null_mut::<ITEMIDLIST>(); 1];
            let mut fetched = 0u32;
            if items.Next(&mut pidl, Some(&mut fetched)).is_err() || fetched == 0 {
                break;
            }
            let p = pidl[0];
            if let Ok(pt) = view.GetItemPosition(p) {
                let size = ILGetSize(Some(p)) as usize;
                let bytes = std::slice::from_raw_parts(p as *const u8, size).to_vec();
                out.push((bytes, pt));
            }
            CoTaskMemFree(Some(p as *const _));
        }
        Ok(out)
    }
}

/// Diagnostics: current desktop icon positions (empty if the shell view is unavailable).
pub fn icon_positions() -> Vec<(i32, i32)> {
    com_init();
    capture_icons().map(|v| v.iter().map(|(_, p)| (p.x, p.y)).collect()).unwrap_or_default()
}

fn restore_icons(icons: &[(Vec<u8>, POINT)]) -> WinResult<()> {
    if icons.is_empty() {
        return Ok(());
    }
    unsafe {
        let view = desktop_folder_view()?;
        let pidls: Vec<*const ITEMIDLIST> = icons.iter().map(|(b, _)| b.as_ptr() as *const ITEMIDLIST).collect();
        let points: Vec<POINT> = icons.iter().map(|(_, p)| *p).collect();
        view.SelectAndPositionItems(pidls.len() as u32, pidls.as_ptr(), Some(points.as_ptr()), SVSI_POSITIONITEM.0 as u32)
    }
}

fn monitor_device_of(hwnd: HWND) -> String {
    unsafe {
        let hmon = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFOEXW::default();
        info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
        if !GetMonitorInfoW(hmon, &mut info.monitorInfo as *mut MONITORINFO).as_bool() {
            return String::new();
        }
        let len = info.szDevice.iter().position(|&c| c == 0).unwrap_or(info.szDevice.len());
        String::from_utf16_lossy(&info.szDevice[..len])
    }
}

struct EnumCtx {
    monitor: String,
    /// Process of the foreground window (the game): its windows are left alone.
    skip_pid: u32,
    out: Windows,
}

unsafe extern "system" fn enum_window(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let ctx = unsafe { &mut *(lparam.0 as *mut EnumCtx) };
    unsafe {
        let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
        let owned = GetWindow(hwnd, GW_OWNER).is_ok_and(|o| !o.is_invalid());
        if !IsWindowVisible(hwnd).as_bool() || IsIconic(hwnd).as_bool() || owned || ex_style & WS_EX_TOOLWINDOW.0 != 0 {
            return BOOL(1);
        }
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid == ctx.skip_pid || monitor_device_of(hwnd) != ctx.monitor {
            return BOOL(1);
        }
        let mut wp = WINDOWPLACEMENT { length: std::mem::size_of::<WINDOWPLACEMENT>() as u32, ..Default::default() };
        // Maximized windows re-maximize on their own after a mode change.
        if GetWindowPlacement(hwnd, &mut wp).is_ok() && wp.showCmd != SW_SHOWMAXIMIZED.0 as u32 {
            ctx.out.push((hwnd.0 as isize, wp));
        }
    }
    BOOL(1)
}

fn capture_windows(monitor: &str) -> Windows {
    let mut skip_pid = 0u32;
    unsafe {
        GetWindowThreadProcessId(GetForegroundWindow(), Some(&mut skip_pid));
    }
    let mut ctx = EnumCtx { monitor: monitor.to_string(), skip_pid, out: Vec::new() };
    unsafe {
        let _ = EnumWindows(Some(enum_window), LPARAM(&mut ctx as *mut EnumCtx as isize));
    }
    ctx.out
}

fn restore_windows(windows: &[(isize, WINDOWPLACEMENT)]) {
    for (raw, wp) in windows {
        let hwnd = HWND(*raw as *mut _);
        unsafe {
            // Skip closed windows and windows the user minimized during the session.
            if !IsWindow(Some(hwnd)).as_bool() || IsIconic(hwnd).as_bool() {
                continue;
            }
            let mut wp = *wp;
            if wp.showCmd == SW_SHOWNORMAL.0 as u32 || wp.showCmd == SW_SHOW.0 as u32 {
                wp.showCmd = SW_SHOWNOACTIVATE.0 as u32; // never steal focus from the game
            }
            wp.flags |= WPF_ASYNCWINDOWPLACEMENT;
            if let Err(e) = SetWindowPlacement(hwnd, &wp) {
                log::debug!("SetWindowPlacement failed: {e}");
            }
        }
    }
}

fn com_init() {
    // S_FALSE (already initialized) and RPC_E_CHANGED_MODE are both fine for our use.
    let _ = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
}

fn apply_restore(windows: &Windows, icons: Option<&Icons>, snapshot_file: &std::path::Path) {
    com_init();
    restore_windows(windows);
    if let Some(icons) = icons {
        match restore_icons(icons) {
            Ok(()) => {
                let _ = std::fs::remove_file(snapshot_file);
            }
            Err(e) => log::warn!("desktop icons not restored: {e}"),
        }
    }
}

impl LayoutApi for Win32Layout {
    fn capture(&mut self, monitor: &str) {
        com_init();
        let Ok(mut book) = self.book.lock() else { return };
        let had_icons = book.icons().is_some();
        book.capture(
            monitor,
            || capture_windows(monitor),
            || {
                capture_icons().unwrap_or_else(|e| {
                    log::warn!("desktop icons not captured: {e}");
                    Vec::new()
                })
            },
        );
        if !had_icons
            && let Some(icons) = book.icons()
                && let Err(e) = std::fs::write(&self.snapshot_file, encode_icons(icons)) {
                    log::warn!("icon snapshot not saved: {e}");
                }
    }

    fn restore(&mut self, monitor: &str) {
        let ticket = match self.book.lock() {
            Ok(mut book) => book.schedule_restore(monitor),
            Err(_) => return,
        };
        let (book, monitor, file) = (self.book.clone(), monitor.to_string(), self.snapshot_file.clone());
        let spawned = std::thread::Builder::new().name("rez-layout-restore".into()).spawn(move || {
            std::thread::sleep(Duration::from_millis(RESTORE_DELAY_MS));
            let due = book.lock().ok().and_then(|mut b| b.fire(&monitor, ticket));
            if let Some((windows, icons)) = due {
                apply_restore(&windows, icons.as_ref(), &file);
            }
        });
        if let Err(e) = spawned {
            log::warn!("cannot spawn layout restore thread: {e}");
        }
    }

    fn flush(&mut self) {
        let has_pending = self.book.lock().map(|mut b| {
            let all = b.take_all_pending();
            let any = !all.is_empty();
            (any, all)
        });
        let Ok((any, all)) = has_pending else { return };
        if any {
            std::thread::sleep(Duration::from_millis(RESTORE_DELAY_MS));
        }
        for (windows, icons) in &all {
            apply_restore(windows, icons.as_ref(), &self.snapshot_file);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icons_snapshot_roundtrips_through_json() {
        let icons = vec![(vec![4u8, 0, 1, 2, 0, 0], POINT { x: 12, y: -3 }), (vec![2u8, 0], POINT { x: 1800, y: 900 })];
        let json = encode_icons(&icons);
        let back = decode_icons(&json).unwrap();
        assert_eq!(back.len(), 2);
        assert_eq!(back[0].0, icons[0].0);
        assert_eq!((back[1].1.x, back[1].1.y), (1800, 900));
        assert!(decode_icons("not json").is_none());
    }
}
