//! The crosshair overlay: a transparent, click-through, topmost layered window owned by its
//! own thread. It never touches the game process: it only reads which window is in the
//! foreground and where it is, then draws the crosshair over its center.

use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Mutex, OnceLock};

use windows::core::w;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM};
use windows::Win32::Graphics::Gdi::{
    ClientToScreen, CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetDC, GetMonitorInfoW,
    MonitorFromWindow, ReleaseDC, SelectObject, AC_SRC_ALPHA, AC_SRC_OVER, BITMAPINFO, BITMAPINFOHEADER, BI_RGB,
    BLENDFUNCTION, DIB_RGB_COLORS, HGDIOBJ, MONITORINFO, MONITOR_DEFAULTTONEAREST,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetClientRect, GetForegroundWindow, GetMessageW,
    GetWindowThreadProcessId, PostMessageW, RegisterClassW, SetTimer, SetWindowPos, ShowWindow, TranslateMessage,
    UpdateLayeredWindow, HWND_TOPMOST, MSG, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SW_HIDE, SW_SHOWNOACTIVATE,
    ULW_ALPHA, WM_APP, WM_TIMER, WNDCLASSW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
    WS_EX_TRANSPARENT, WS_POPUP,
};

use crate::crosshair::director::{client_center, geometry, Geometry, OverlayCommand, OverlaySpec};
use crate::crosshair::render::render;
use crate::runtime::OverlayApi;

const WM_REZ_COMMAND: u32 = WM_APP + 1;
const TIMER_ID: usize = 1;
const TICK_MS: u32 = 250;

/// Handle used by the runtime. Commands are queued and the overlay thread is woken up.
pub struct Win32Overlay {
    tx: Sender<OverlayCommand>,
    hwnd: isize,
}

impl Win32Overlay {
    /// Starts the overlay thread; `image_dir` holds imported crosshair PNGs.
    pub fn spawn(image_dir: PathBuf) -> Self {
        let (tx, rx) = mpsc::channel();
        let (hwnd_tx, hwnd_rx) = mpsc::channel();
        let spawned = std::thread::Builder::new()
            .name("rez-overlay".into())
            .spawn(move || run(rx, hwnd_tx, image_dir));
        if let Err(e) = spawned {
            log::warn!("overlay thread not started: {e}");
        }
        let hwnd = hwnd_rx.recv().unwrap_or(0);
        Win32Overlay { tx, hwnd }
    }
}

impl OverlayApi for Win32Overlay {
    fn apply(&mut self, cmd: OverlayCommand) {
        if self.tx.send(cmd).is_ok() && self.hwnd != 0 {
            unsafe {
                let _ = PostMessageW(Some(HWND(self.hwnd as *mut _)), WM_REZ_COMMAND, WPARAM(0), LPARAM(0));
            }
        }
    }
}

/// What is currently drawn, to skip redundant redraws.
#[derive(PartialEq)]
struct Drawn {
    center: (i32, i32),
    geometry: Geometry,
    crosshair: crate::crosshair::model::Crosshair,
}

struct State {
    rx: Receiver<OverlayCommand>,
    spec: Option<OverlaySpec>,
    drawn: Option<Drawn>,
    visible: bool,
    image_dir: PathBuf,
}

static STATE: OnceLock<Mutex<State>> = OnceLock::new();

fn run(rx: Receiver<OverlayCommand>, hwnd_tx: Sender<isize>, image_dir: PathBuf) {
    unsafe {
        let hinstance = GetModuleHandleW(None).map(|h| h.into()).ok();
        let class = WNDCLASSW {
            lpfnWndProc: Some(wnd_proc),
            hInstance: hinstance.unwrap_or_default(),
            lpszClassName: w!("REZ_CROSSHAIR"),
            ..Default::default()
        };
        RegisterClassW(&class);
        let ex = WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE;
        let hwnd = match CreateWindowExW(ex, w!("REZ_CROSSHAIR"), w!("REZ crosshair"), WS_POPUP, 0, 0, 1, 1, None, None, hinstance, None) {
            Ok(h) => h,
            Err(e) => {
                log::warn!("overlay window not created: {e}");
                let _ = hwnd_tx.send(0);
                return;
            }
        };
        let _ = STATE.set(Mutex::new(State { rx, spec: None, drawn: None, visible: false, image_dir }));
        SetTimer(Some(hwnd), TIMER_ID, TICK_MS, None);
        let _ = hwnd_tx.send(hwnd.0 as isize);
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_REZ_COMMAND | WM_TIMER => {
            if let Some(state) = STATE.get()
                && let Ok(mut s) = state.lock() {
                    while let Ok(cmd) = s.rx.try_recv() {
                        match cmd {
                            OverlayCommand::Show(spec) => {
                                s.spec = Some(*spec);
                                s.drawn = None; // new crosshair or game: redraw and re-assert topmost
                            }
                            OverlayCommand::Hide => s.spec = None,
                        }
                    }
                    update(hwnd, &mut s);
                }
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

fn hide(hwnd: HWND, s: &mut State) {
    if s.visible {
        unsafe {
            let _ = ShowWindow(hwnd, SW_HIDE);
        }
        s.visible = false;
        s.drawn = None;
    }
}

/// Re-evaluates position and size from the foreground window; redraws only on change.
fn update(hwnd: HWND, s: &mut State) {
    let Some(spec) = s.spec.clone() else {
        hide(hwnd, s);
        return;
    };
    unsafe {
        let fg = GetForegroundWindow();
        let mut pid = 0u32;
        GetWindowThreadProcessId(fg, Some(&mut pid));
        if fg.is_invalid() || pid != spec.pid {
            hide(hwnd, s);
            return;
        }
        let mut rc = RECT::default();
        if GetClientRect(fg, &mut rc).is_err() {
            hide(hwnd, s);
            return;
        }
        let mut tl = POINT { x: rc.left, y: rc.top };
        let mut br = POINT { x: rc.right, y: rc.bottom };
        let _ = ClientToScreen(fg, &mut tl);
        let _ = ClientToScreen(fg, &mut br);
        let center = client_center(tl.x, tl.y, br.x, br.y, (spec.crosshair.offset_x, spec.crosshair.offset_y));

        let mut mi = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
        let hmon = MonitorFromWindow(fg, MONITOR_DEFAULTTONEAREST);
        let mode = if GetMonitorInfoW(hmon, &mut mi).as_bool() {
            ((mi.rcMonitor.right - mi.rcMonitor.left) as u32, (mi.rcMonitor.bottom - mi.rcMonitor.top) as u32)
        } else {
            spec.native
        };
        let geo = geometry(mode, spec.native, spec.undistort);
        let next = Drawn { center, geometry: geo, crosshair: spec.crosshair.clone() };
        if s.drawn.as_ref() == Some(&next) && s.visible {
            return;
        }
        let newly_shown = !s.visible || s.drawn.is_none();
        if draw(hwnd, &next, &s.image_dir) {
            if newly_shown {
                let _ = SetWindowPos(hwnd, Some(HWND_TOPMOST), 0, 0, 0, 0, SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE);
                let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            }
            s.visible = true;
            s.drawn = Some(next);
        }
    }
}

/// Renders the crosshair and pushes it to the layered window, centered on `d.center`.
fn draw(hwnd: HWND, d: &Drawn, image_dir: &std::path::Path) -> bool {
    let img = render(&d.crosshair, d.geometry.scale, d.geometry.x_factor, image_dir);
    let (w, h) = (img.width as i32, img.height as i32);
    unsafe {
        let screen = GetDC(None);
        let mem = CreateCompatibleDC(Some(screen));
        let info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: w,
                biHeight: -h,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits: *mut core::ffi::c_void = std::ptr::null_mut();
        let ok = match CreateDIBSection(Some(mem), &info, DIB_RGB_COLORS, &mut bits, None, 0) {
            Ok(bitmap) if !bits.is_null() => {
                // Premultiplied RGBA → premultiplied BGRA.
                let dst = std::slice::from_raw_parts_mut(bits as *mut u8, img.pixels.len());
                for (d, p) in dst.chunks_exact_mut(4).zip(img.pixels.chunks_exact(4)) {
                    d.copy_from_slice(&[p[2], p[1], p[0], p[3]]);
                }
                let old = SelectObject(mem, HGDIOBJ(bitmap.0));
                let pos = POINT { x: d.center.0 - w / 2, y: d.center.1 - h / 2 };
                let size = SIZE { cx: w, cy: h };
                let src = POINT { x: 0, y: 0 };
                let blend = BLENDFUNCTION { BlendOp: AC_SRC_OVER as u8, BlendFlags: 0, SourceConstantAlpha: 255, AlphaFormat: AC_SRC_ALPHA as u8 };
                let res = UpdateLayeredWindow(hwnd, Some(screen), Some(&pos), Some(&size), Some(mem), Some(&src), Default::default(), Some(&blend), ULW_ALPHA);
                SelectObject(mem, old);
                let _ = DeleteObject(HGDIOBJ(bitmap.0));
                if let Err(e) = &res {
                    log::warn!("UpdateLayeredWindow failed: {e}");
                }
                res.is_ok()
            }
            _ => false,
        };
        let _ = DeleteDC(mem);
        ReleaseDC(None, screen);
        ok
    }
}
