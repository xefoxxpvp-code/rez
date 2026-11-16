//! Win32 implementation of [`DisplayApi`] using `ChangeDisplaySettingsExW`.
//!
//! Modes are always applied with `CDS_FULLSCREEN` (temporary, never written to the
//! registry) and restored by passing a null DEVMODE, which reloads the registry mode.

use windows::core::PCWSTR;
use windows::Win32::Graphics::Gdi::{
    ChangeDisplaySettingsExW, EnumDisplayDevicesW, EnumDisplaySettingsExW, CDS_FULLSCREEN, CDS_TYPE,
    DEVMODEW, DISPLAY_DEVICEW, DISPLAY_DEVICE_ATTACHED_TO_DESKTOP, DISPLAY_DEVICE_PRIMARY_DEVICE,
    DISP_CHANGE_SUCCESSFUL, DMDFO_STRETCH, DM_DISPLAYFIXEDOUTPUT, DM_DISPLAYFREQUENCY, DM_PELSHEIGHT,
    DM_PELSWIDTH, ENUM_CURRENT_SETTINGS, ENUM_DISPLAY_SETTINGS_FLAGS, ENUM_DISPLAY_SETTINGS_MODE,
    ENUM_REGISTRY_SETTINGS,
};

use windows::Win32::Devices::Display::{
    GetDisplayConfigBufferSizes, QueryDisplayConfig, SetDisplayConfig, DISPLAYCONFIG_MODE_INFO,
    DISPLAYCONFIG_PATH_INFO, QDC_ONLY_ACTIVE_PATHS, SDC_ALLOW_CHANGES, SDC_APPLY, SDC_USE_SUPPLIED_DISPLAY_CONFIG,
};
use std::sync::Mutex;

use crate::display::{DisplayApi, DisplayError, DisplayMode, MonitorInfo};

type Topology = (Vec<DISPLAYCONFIG_PATH_INFO>, Vec<DISPLAYCONFIG_MODE_INFO>);

#[derive(Default)]
pub struct Win32Display {
    saved_topology: Mutex<Option<Topology>>,
}

impl Win32Display {
    pub fn new() -> Self {
        Self::default()
    }
}

fn query_topology() -> Result<Topology, DisplayError> {
    unsafe {
        let (mut np, mut nm) = (0u32, 0u32);
        let err = GetDisplayConfigBufferSizes(QDC_ONLY_ACTIVE_PATHS, &mut np, &mut nm);
        if err.is_err() {
            return Err(DisplayError::Win32(format!("GetDisplayConfigBufferSizes: {err:?}")));
        }
        let mut paths = vec![DISPLAYCONFIG_PATH_INFO::default(); np as usize];
        let mut modes = vec![DISPLAYCONFIG_MODE_INFO::default(); nm as usize];
        let err = QueryDisplayConfig(QDC_ONLY_ACTIVE_PATHS, &mut np, paths.as_mut_ptr(), &mut nm, modes.as_mut_ptr(), None);
        if err.is_err() {
            return Err(DisplayError::Win32(format!("QueryDisplayConfig: {err:?}")));
        }
        paths.truncate(np as usize);
        modes.truncate(nm as usize);
        Ok((paths, modes))
    }
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn from_wide(buf: &[u16]) -> String {
    let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..len])
}

fn new_devmode() -> DEVMODEW {
    DEVMODEW { dmSize: std::mem::size_of::<DEVMODEW>() as u16, ..Default::default() }
}

fn to_mode(dm: &DEVMODEW) -> DisplayMode {
    DisplayMode { width: dm.dmPelsWidth, height: dm.dmPelsHeight, refresh_hz: dm.dmDisplayFrequency }
}

fn enum_settings(device: &str, index: ENUM_DISPLAY_SETTINGS_MODE) -> Option<DEVMODEW> {
    let name = wide(device);
    let mut dm = new_devmode();
    let ok = unsafe {
        EnumDisplaySettingsExW(PCWSTR(name.as_ptr()), index, &mut dm, ENUM_DISPLAY_SETTINGS_FLAGS(0))
    };
    ok.as_bool().then_some(dm)
}

/// All display adapters' outputs currently attached to the desktop.
fn attached_devices() -> Vec<DISPLAY_DEVICEW> {
    let mut out = Vec::new();
    for i in 0.. {
        let mut dd = DISPLAY_DEVICEW { cb: std::mem::size_of::<DISPLAY_DEVICEW>() as u32, ..Default::default() };
        if !unsafe { EnumDisplayDevicesW(PCWSTR::null(), i, &mut dd, 0) }.as_bool() {
            break;
        }
        if dd.StateFlags & DISPLAY_DEVICE_ATTACHED_TO_DESKTOP == DISPLAY_DEVICE_ATTACHED_TO_DESKTOP {
            out.push(dd);
        }
    }
    out
}

/// Monitor name for an adapter output (second-level `EnumDisplayDevicesW`).
fn monitor_label(device: &str) -> Option<String> {
    let name = wide(device);
    let mut dd = DISPLAY_DEVICEW { cb: std::mem::size_of::<DISPLAY_DEVICEW>() as u32, ..Default::default() };
    unsafe { EnumDisplayDevicesW(PCWSTR(name.as_ptr()), 0, &mut dd, 0) }
        .as_bool()
        .then(|| from_wide(&dd.DeviceString))
}

fn change(device: &str, dm: Option<&DEVMODEW>, flags: CDS_TYPE) -> i32 {
    let name = wide(device);
    let res = unsafe {
        ChangeDisplaySettingsExW(
            PCWSTR(name.as_ptr()),
            dm.map(|d| d as *const DEVMODEW),
            None,
            flags,
            None,
        )
    };
    res.0
}

/// Restores every attached display to its registry mode. Never panics; used by the
/// panic hook, the guardian process and `WM_ENDSESSION`.
pub fn restore_all() {
    for dd in attached_devices() {
        let device = from_wide(&dd.DeviceName);
        let code = change(&device, None, CDS_TYPE(0));
        if code != DISP_CHANGE_SUCCESSFUL.0 {
            log::warn!("restore_all: {device} returned {code}");
        }
    }
}

impl DisplayApi for Win32Display {
    fn list_monitors(&self) -> Result<Vec<MonitorInfo>, DisplayError> {
        Ok(attached_devices()
            .into_iter()
            .map(|dd| {
                let device = from_wide(&dd.DeviceName);
                let label = monitor_label(&device)
                    .filter(|l| !l.is_empty())
                    .unwrap_or_else(|| from_wide(&dd.DeviceString));
                let primary = dd.StateFlags & DISPLAY_DEVICE_PRIMARY_DEVICE == DISPLAY_DEVICE_PRIMARY_DEVICE;
                MonitorInfo { device, label, primary }
            })
            .collect())
    }

    fn list_modes(&self, device: &str) -> Result<Vec<DisplayMode>, DisplayError> {
        let mut modes = Vec::new();
        for i in 0.. {
            match enum_settings(device, ENUM_DISPLAY_SETTINGS_MODE(i)) {
                Some(dm) if dm.dmBitsPerPel == 32 => {
                    let mode = to_mode(&dm);
                    if !modes.contains(&mode) {
                        modes.push(mode);
                    }
                }
                Some(_) => {}
                None => break,
            }
        }
        if modes.is_empty() {
            return Err(DisplayError::Win32(format!("aucun mode pour {device}")));
        }
        Ok(modes)
    }

    fn registry_mode(&self, device: &str) -> Result<DisplayMode, DisplayError> {
        enum_settings(device, ENUM_REGISTRY_SETTINGS)
            .map(|dm| to_mode(&dm))
            .ok_or_else(|| DisplayError::Win32(format!("mode registre illisible pour {device}")))
    }

    fn current_mode(&self, device: &str) -> Result<DisplayMode, DisplayError> {
        enum_settings(device, ENUM_CURRENT_SETTINGS)
            .map(|dm| to_mode(&dm))
            .ok_or_else(|| DisplayError::Win32(format!("mode courant illisible pour {device}")))
    }

    fn apply_temporary(&self, device: &str, mode: DisplayMode) -> Result<(), DisplayError> {
        let mut dm = new_devmode();
        dm.dmPelsWidth = mode.width;
        dm.dmPelsHeight = mode.height;
        dm.dmDisplayFrequency = mode.refresh_hz;
        dm.dmFields = DM_PELSWIDTH | DM_PELSHEIGHT | DM_DISPLAYFREQUENCY | DM_DISPLAYFIXEDOUTPUT;
        dm.Anonymous1.Anonymous2.dmDisplayFixedOutput = DMDFO_STRETCH;

        let code = change(device, Some(&dm), CDS_FULLSCREEN);
        if code == DISP_CHANGE_SUCCESSFUL.0 {
            return Ok(());
        }
        // Some drivers reject the fixed-output hint: retry without it.
        dm.dmFields = DM_PELSWIDTH | DM_PELSHEIGHT | DM_DISPLAYFREQUENCY;
        let code = change(device, Some(&dm), CDS_FULLSCREEN);
        if code == DISP_CHANGE_SUCCESSFUL.0 { Ok(()) } else { Err(DisplayError::Rejected(code)) }
    }

    fn restore(&self, device: &str) -> Result<(), DisplayError> {
        let code = change(device, None, CDS_TYPE(0));
        if code == DISP_CHANGE_SUCCESSFUL.0 { Ok(()) } else { Err(DisplayError::Rejected(code)) }
    }

    fn save_topology(&self) -> Result<(), DisplayError> {
        let topology = query_topology()?;
        *self.saved_topology.lock().map_err(|_| DisplayError::Win32("poisoned".into()))? = Some(topology);
        Ok(())
    }

    fn restore_topology(&self) -> Result<(), DisplayError> {
        let saved = self.saved_topology.lock().map_err(|_| DisplayError::Win32("poisoned".into()))?.clone();
        let Some((paths, modes)) = saved else {
            return Err(DisplayError::Win32("aucune configuration d'écrans enregistrée".into()));
        };
        let code = unsafe {
            SetDisplayConfig(Some(&paths), Some(&modes), SDC_APPLY | SDC_USE_SUPPLIED_DISPLAY_CONFIG | SDC_ALLOW_CHANGES)
        };
        if code == 0 {
            return Ok(());
        }
        // No blind fallback (e.g. forcing "extend"): it could enable a screen the user keeps off.
        log::warn!("SetDisplayConfig(saved) failed with {code}");
        Err(DisplayError::Rejected(code))
    }
}
