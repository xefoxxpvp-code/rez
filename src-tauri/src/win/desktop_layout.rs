//! Saves and restores desktop icon positions (shell `IFolderView`) and the placement of
//! top-level windows on a monitor around a mode switch. Errors are logged, never raised:
//! a failed layout save/restore must never block a resolution switch.

use std::collections::HashMap;

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
    EnumWindows, GetWindow, GetWindowLongW, GetWindowPlacement, IsIconic, IsWindow, IsWindowVisible,
    SetWindowPlacement, GWL_EXSTYLE, GW_OWNER, WINDOWPLACEMENT, WS_EX_TOOLWINDOW,
};

use crate::runtime::LayoutApi;

/// Delay before restoring, so Explorer finishes its own rearrangement after the mode change.
const RESTORE_DELAY_MS: u64 = 800;

#[derive(Clone, Default)]
struct Saved {
    /// Desktop items as owned PIDL bytes (Send-safe), with their positions.
    icons: Vec<(Vec<u8>, POINT)>,
    windows: Vec<(isize, WINDOWPLACEMENT)>,
}

#[derive(Default)]
pub struct Win32Layout {
    saved: HashMap<String, Saved>,
}

impl Win32Layout {
    pub fn new() -> Self {
        Self::default()
    }
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

fn capture_icons() -> WinResult<Vec<(Vec<u8>, POINT)>> {
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
    out: Vec<(isize, WINDOWPLACEMENT)>,
}

unsafe extern "system" fn enum_window(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let ctx = unsafe { &mut *(lparam.0 as *mut EnumCtx) };
    unsafe {
        let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
        let owned = GetWindow(hwnd, GW_OWNER).is_ok_and(|o| !o.is_invalid());
        if !IsWindowVisible(hwnd).as_bool() || IsIconic(hwnd).as_bool() || owned || ex_style & WS_EX_TOOLWINDOW.0 != 0 {
            return BOOL(1);
        }
        if monitor_device_of(hwnd) != ctx.monitor {
            return BOOL(1);
        }
        let mut wp = WINDOWPLACEMENT { length: std::mem::size_of::<WINDOWPLACEMENT>() as u32, ..Default::default() };
        if GetWindowPlacement(hwnd, &mut wp).is_ok() {
            ctx.out.push((hwnd.0 as isize, wp));
        }
    }
    BOOL(1)
}

fn capture_windows(monitor: &str) -> Vec<(isize, WINDOWPLACEMENT)> {
    let mut ctx = EnumCtx { monitor: monitor.to_string(), out: Vec::new() };
    unsafe {
        let _ = EnumWindows(Some(enum_window), LPARAM(&mut ctx as *mut EnumCtx as isize));
    }
    ctx.out
}

fn restore_windows(windows: &[(isize, WINDOWPLACEMENT)]) {
    for (raw, wp) in windows {
        let hwnd = HWND(*raw as *mut _);
        unsafe {
            if IsWindow(Some(hwnd)).as_bool() {
                if let Err(e) = SetWindowPlacement(hwnd, wp) {
                    log::debug!("SetWindowPlacement failed: {e}");
                }
            }
        }
    }
}

fn com_init() {
    // S_FALSE (already initialized) and RPC_E_CHANGED_MODE are both fine for our use.
    let _ = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
}

impl LayoutApi for Win32Layout {
    fn capture(&mut self, monitor: &str) {
        com_init();
        let icons = capture_icons().unwrap_or_else(|e| {
            log::warn!("desktop icons not captured: {e}");
            Vec::new()
        });
        let windows = capture_windows(monitor);
        log::info!("layout captured for {monitor}: {} icons, {} windows", icons.len(), windows.len());
        self.saved.insert(monitor.to_string(), Saved { icons, windows });
    }

    fn restore(&mut self, monitor: &str) {
        let Some(saved) = self.saved.remove(monitor) else { return };
        let spawned = std::thread::Builder::new().name("rez-layout-restore".into()).spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(RESTORE_DELAY_MS));
            com_init();
            restore_windows(&saved.windows);
            if let Err(e) = restore_icons(&saved.icons) {
                log::warn!("desktop icons not restored: {e}");
            }
        });
        if let Err(e) = spawned {
            log::warn!("cannot spawn layout restore thread: {e}");
        }
    }
}
