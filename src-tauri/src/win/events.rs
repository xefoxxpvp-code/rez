//! Win32 event sources: foreground-window changes (out-of-context WinEvent hook, no
//! injection into any process), display/session changes via a hidden top-level window,
//! and process-exit watches.

use std::sync::mpsc::{self, Sender};
use std::sync::{Mutex, OnceLock};

use windows::core::{w, PWSTR};
use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Gdi::{GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITORINFOEXW, MONITOR_DEFAULTTONEAREST};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::RemoteDesktop::{WTSRegisterSessionNotification, NOTIFY_FOR_THIS_SESSION};
use windows::Win32::Foundation::{STILL_ACTIVE, WAIT_OBJECT_0};
use windows::Win32::System::Threading::{
    GetExitCodeProcess, OpenProcess, QueryFullProcessImageNameW, WaitForSingleObject, INFINITE, PROCESS_NAME_WIN32,
    PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE,
};
use windows::Win32::UI::Accessibility::{SetWinEventHook, HWINEVENTHOOK};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, GetWindowThreadProcessId, RegisterClassW,
    TranslateMessage, EVENT_SYSTEM_FOREGROUND, MSG, WINDOW_EX_STYLE, WINEVENT_OUTOFCONTEXT, WM_DISPLAYCHANGE,
    WM_ENDSESSION, WM_WTSSESSION_CHANGE, WNDCLASSW, WS_POPUP,
};

use crate::runtime::Input;

const WTS_SESSION_UNLOCK: usize = 0x8;

static TX: OnceLock<Mutex<Sender<Input>>> = OnceLock::new();

fn send(input: Input) {
    if let Some(tx) = TX.get()
        && let Ok(tx) = tx.lock() {
            let _ = tx.send(input);
        }
}

fn exe_name_of(pid: u32) -> String {
    unsafe {
        let Ok(handle) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else {
            log::warn!("cannot open process {pid} to read its image name");
            return String::new();
        };
        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(handle, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut len);
        let _ = CloseHandle(handle);
        if ok.is_err() {
            log::warn!("cannot read image name of process {pid}");
            return String::new();
        }
        let path = String::from_utf16_lossy(&buf[..len as usize]);
        path.rsplit('\\').next().unwrap_or(&path).to_string()
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

unsafe extern "system" fn on_foreground(
    _hook: HWINEVENTHOOK,
    _event: u32,
    hwnd: HWND,
    _id_object: i32,
    _id_child: i32,
    _thread: u32,
    _time: u32,
) {
    if hwnd.is_invalid() {
        return;
    }
    let mut pid = 0u32;
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
    if pid == 0 {
        return;
    }
    send(Input::Foreground { pid, exe_name: exe_name_of(pid), monitor: monitor_device_of(hwnd) });
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_DISPLAYCHANGE => send(Input::DisplayMaybeChanged),
        WM_WTSSESSION_CHANGE if wparam.0 == WTS_SESSION_UNLOCK => send(Input::DisplayMaybeChanged),
        // Synchronous: the session may end before the runtime thread gets to run.
        WM_ENDSESSION if wparam.0 != 0 => crate::win::display::restore_all(),
        _ => {}
    }
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

/// Starts the event thread (WinEvent hook + hidden window + message loop).
/// Returns once the hook is installed.
pub fn start(tx: Sender<Input>) -> std::io::Result<()> {
    if TX.set(Mutex::new(tx)).is_err() {
        return Err(std::io::Error::other("events already started"));
    }
    let (ready_tx, ready_rx) = mpsc::channel::<Result<(), String>>();
    std::thread::Builder::new().name("rez-events".into()).spawn(move || unsafe {
        let hook = SetWinEventHook(
            EVENT_SYSTEM_FOREGROUND,
            EVENT_SYSTEM_FOREGROUND,
            None,
            Some(on_foreground),
            0,
            0,
            WINEVENT_OUTOFCONTEXT,
        );
        if hook.is_invalid() {
            let _ = ready_tx.send(Err("SetWinEventHook failed".into()));
            return;
        }
        let hinstance = GetModuleHandleW(None).map(|h| h.into()).ok();
        let class = WNDCLASSW { lpfnWndProc: Some(wnd_proc), hInstance: hinstance.unwrap_or_default(), lpszClassName: w!("REZ_EVENTS"), ..Default::default() };
        RegisterClassW(&class);
        match CreateWindowExW(WINDOW_EX_STYLE(0), w!("REZ_EVENTS"), w!("REZ"), WS_POPUP, 0, 0, 0, 0, None, None, hinstance, None) {
            Ok(hwnd) => {
                if let Err(e) = WTSRegisterSessionNotification(hwnd, NOTIFY_FOR_THIS_SESSION) {
                    log::warn!("WTSRegisterSessionNotification failed: {e}");
                }
            }
            Err(e) => log::warn!("hidden event window not created: {e}"),
        }
        let _ = ready_tx.send(Ok(()));
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    })?;
    match ready_rx.recv() {
        Ok(Ok(())) => Ok(()),
        Ok(Err(e)) => Err(std::io::Error::other(e)),
        Err(_) => Err(std::io::Error::other("event thread died during startup")),
    }
}

/// Sends `ProcessExited { pid }` when the process ends. Waits on the process handle; if the
/// wait cannot be used (some anti-cheats strip SYNCHRONIZE), polls the exit code instead.
/// Never reports an exit it did not observe.
pub fn watch_process(pid: u32, tx: Sender<Input>) {
    let spawned = std::thread::Builder::new().name(format!("rez-watch-{pid}")).spawn(move || unsafe {
        if let Ok(handle) = OpenProcess(PROCESS_SYNCHRONIZE, false, pid) {
            let waited = WaitForSingleObject(handle, INFINITE);
            let _ = CloseHandle(handle);
            if waited == WAIT_OBJECT_0 {
                let _ = tx.send(Input::ProcessExited { pid });
                return;
            }
            log::warn!("wait on process {pid} failed ({waited:?}); polling instead");
        }
        let Ok(handle) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else {
            log::warn!("cannot watch process {pid}");
            return;
        };
        loop {
            std::thread::sleep(std::time::Duration::from_secs(1));
            let mut code = 0u32;
            match GetExitCodeProcess(handle, &mut code) {
                Ok(()) if code == STILL_ACTIVE.0 as u32 => continue,
                Ok(()) => {
                    let _ = tx.send(Input::ProcessExited { pid });
                    break;
                }
                Err(e) => {
                    log::warn!("cannot poll process {pid}: {e}");
                    break;
                }
            }
        }
        let _ = CloseHandle(handle);
    });
    if let Err(e) = spawned {
        log::warn!("cannot spawn watch thread for {pid}: {e}");
    }
}
