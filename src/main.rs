use windows::{
    core::*,
    Win32::{
        Foundation::*,
        System::LibraryLoader::*,
        UI::WindowsAndMessaging::*,
        Graphics::Gdi::*
    }
};
use std::sync::*;
#[macro_use]
extern  crate lazy_static;

const W_APP_NAME: PCWSTR = w!("dwmr-win32");
const S_APP_NAME: PCSTR = s!("dwmr-win32");

#[derive(Default)]
struct Rect {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

#[derive(Default)]
struct Monitor {
    name: [u16; 32],
    master_index: u32,
    index: u32,
    bar_y: i32,
    size: Rect,
    window_area: Rect,
}

#[derive(Default)]
struct DwmrApp {
    hwnd: RwLock<Option<HWND>>,
    monitors: RwLock<Vec<Monitor>>
}

lazy_static! {
    static ref DWMR_APP: DwmrApp = DwmrApp::default();
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT
{
    LRESULT::default()
}

unsafe extern "system" fn enum_monitor(hmonitor: HMONITOR, _: HDC, rect: *mut RECT, _: LPARAM) -> BOOL {
    let mut monitor_info = MONITORINFOEXW{
        monitorInfo: MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFOEXW>() as u32,
            ..Default::default()
        },
        ..Default::default()
    };
    if GetMonitorInfoW(hmonitor, &mut monitor_info.monitorInfo) == FALSE {
        return TRUE;
    }

    //unsigned shot to str
    let monitor_name = PCWSTR::from_raw(monitor_info.szDevice.as_ptr());

    let mut monitor = Monitor{
        name: monitor_info.szDevice,
        size: Rect {
            x: monitor_info.monitorInfo.rcMonitor.left,
            y: monitor_info.monitorInfo.rcMonitor.top,
            width: monitor_info.monitorInfo.rcMonitor.right - monitor_info.monitorInfo.rcMonitor.left,
            height: monitor_info.monitorInfo.rcMonitor.bottom - monitor_info.monitorInfo.rcMonitor.top
        },
        window_area: Rect {
            x: monitor_info.monitorInfo.rcWork.left,
            y: monitor_info.monitorInfo.rcWork.top,
            width: monitor_info.monitorInfo.rcWork.right - monitor_info.monitorInfo.rcWork.left,
            height: monitor_info.monitorInfo.rcWork.bottom - monitor_info.monitorInfo.rcWork.top
        },
        ..Default::default()
    };
    // monitor.index = DWMR_APP.monitors.read().unwrap().len() as u32;
    // DWMR_APP.monitors.write().unwrap().push(monitor);
    TRUE
}

unsafe fn request_update_geom() -> Result<()> {
    let mut wa: RECT = RECT::default();

    SystemParametersInfoW(
        SPI_GETWORKAREA,
        0,
        Some(&mut wa as *mut _ as *mut _),
        SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS{0:0}
    )?;

    if EnumDisplayMonitors(None, None, Some(enum_monitor), None) == FALSE {
        return Ok(());
    }
    Ok(())
}

unsafe fn setup(hinstance: &HINSTANCE) -> Result<()> {
    let wnd_class = WNDCLASSW {
        lpfnWndProc: Some(wnd_proc),
        hInstance: *hinstance,
        lpszClassName: W_APP_NAME,
        ..Default::default()
    };

    request_update_geom()?;

    if RegisterClassW(&wnd_class) == 0{
        GetLastError()?;
    }

    let hwnd_result = CreateWindowExW(
        WINDOW_EX_STYLE::default(),
        W_APP_NAME,
        W_APP_NAME,
        WINDOW_STYLE::default(),
        0,
        0,
        0,
        0,
        None,
        None,
        *hinstance,
        None,
    );

    if hwnd_result.0 == 0 {
        GetLastError()?;
    }

    let mut hwnd = DWMR_APP.hwnd.write().unwrap();
    *hwnd = Some(hwnd_result);
    Ok(())
}

unsafe fn cleanup(hinstance: &HINSTANCE) -> Result<()> {
    //let mut hwnd = DWMR_APP.hwnd.write().unwrap();
    //DestroyWindow((*hwnd).unwrap())?;
    //*hwnd = None;

    //UnregisterClassW(W_APP_NAME, *hinstance)?;

    Ok(())
}

fn main() -> Result<()> {
    unsafe{
        let hmodule = GetModuleHandleW(None)?;
        let hinstance: HINSTANCE = hmodule.into();
        setup(&hinstance)?;
        cleanup(&hinstance)?; 
    }
    Ok(())
}
