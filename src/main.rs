use windows::{
    core::*,
    Win32::{
        Foundation::*,
        System::LibraryLoader::*,
        UI::WindowsAndMessaging::*,
        Graphics::{
            Dwm::*,
            Gdi::*
        }
    }
};
use std::{
    sync::*, 
    collections::*,
    mem::size_of
};

mod test;

// a macro to check bit flags for u32
macro_rules! has_flag {
    ($flags:expr, $flag:expr) => {
        $flags & $flag == $flag
    };
}

#[macro_use]
extern  crate lazy_static;

const W_APP_NAME: PCWSTR = w!("dwmr-win32");
const S_APP_NAME: PCSTR = s!("dwmr-win32");

const BAR_HEIGHT: i32 = 20;


#[derive(Default)]
struct Rect {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

impl Rect {
    fn from_win_rect(rect: &RECT) -> Rect {
        Rect {
            x: rect.left,
            y: rect.top,
            width: rect.right - rect.left,
            height: rect.bottom - rect.top
        }
    }
}

#[derive(Default)]
struct Monitor {
    //LPCWSTR type
    name: [u16; 32],
    master_index: u32,
    index: u32,
    bar_y: i32,
    rect: Rect,
    client_area: Rect,
}

#[derive(Default)]
struct Client {
    hwnd: HWND,
    parent: HWND,
    root: HWND,
    rect: Rect,
    bw: i32,
    tags: u32,
    is_minimized: bool,
    is_floating: bool,
    is_ignored: bool,
    ignore_borders: bool,
    border: bool,
    was_visible: bool,
    is_fixed: bool,
    is_urgent: bool,
    is_cloaked: bool,
}

#[derive(Default)]
struct DwmrApp {
    hwnd: RwLock<Option<HWND>>,
    monitors: RwLock<Vec<Monitor>>,
    clients: RwLock<LinkedList<Client>>,
}

lazy_static! {
    static ref DWMR_APP: DwmrApp = DwmrApp::default();
    static ref DISALLOWED_TITLE: HashSet<String> = HashSet::from([
        "Windows Shell Experience Host".to_string(),
        "Microsoft Text Input Application".to_string(),
        "Action center".to_string(),
        "New Notification".to_string(),
        "Date and Time Information".to_string(),
        "Volume Control".to_string(),
        "Network Connections".to_string(),
        "Cortana".to_string(),
        "Start".to_string(),
        "Windows Default Lock Screen".to_string(),
        "Search".to_string(),
        "WinUI Desktop".to_string()
    ]);

    static ref DISALLOWED_CLASS: HashSet<String> = HashSet::from([
        "Windows.UI.Core.CoreWindow".to_string(),
        "ForegroundStaging".to_string(),
        "ApplicationManager_DesktopShellWindow".to_string(),
        "Static".to_string(),
        "Scrollbar".to_string(),
        "Progman".to_string(),
    ]);
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT
{
    LRESULT::default()
}

unsafe extern "system" fn update_geom(hmonitor: HMONITOR, _: HDC, rect: *mut RECT, _: LPARAM) -> BOOL {
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
    let _monitor_name = PCWSTR::from_raw(monitor_info.szDevice.as_ptr()).to_string().unwrap();

    let monitor = Monitor{
        name: monitor_info.szDevice,
        index: DWMR_APP.monitors.read().unwrap().len() as u32,
        rect: Rect::from_win_rect(&monitor_info.monitorInfo.rcMonitor),
        client_area: Rect::from_win_rect(&monitor_info.monitorInfo.rcWork),
        ..Default::default()
    };

    DWMR_APP.monitors.write().unwrap().push(monitor);
    TRUE
}

unsafe fn request_update_geom() -> Result<()> {
    let monitors = GetSystemMetrics(SM_CMONITORS) as usize;
    DWMR_APP.monitors.write().unwrap().reserve(monitors);


    if EnumDisplayMonitors(None, None, Some(update_geom), None) == FALSE {
        return Ok(());
    }
    Ok(())
}

unsafe fn is_cloaked(hwnd: &HWND) -> Result<bool> {
    let mut cloaked_val = 0;
    DwmGetWindowAttribute(*hwnd, DWMWA_CLOAKED, (&mut cloaked_val) as *const _ as *mut _, size_of::<u32>() as u32)?;
    let is_cloaked = cloaked_val > 0;

    Ok(is_cloaked)
}

pub unsafe fn is_manageable(hwnd: &HWND) -> Result<bool>
{
    let style = GetWindowLongW(*hwnd, GWL_STYLE) as u32;
    if has_flag!(style, WS_DISABLED.0) {
        return Ok(false);
    }

    let exstyle = GetWindowLongW(*hwnd, GWL_EXSTYLE) as u32;
    if has_flag!(exstyle, WS_EX_NOACTIVATE.0) {
        return Ok(false);
    }

    SetLastError(WIN32_ERROR(0));
    let name_length = GetWindowTextLengthW(*hwnd);
    if name_length == 0 {
        GetLastError()?;
        return Ok(false);
    }

    if is_cloaked(hwnd)? {
        return Ok(false);
    }

    let mut client_name_buf = [0u16; 256];
    SetLastError(WIN32_ERROR(0));
    if GetWindowTextW(*hwnd, client_name_buf.as_mut()) == 0 {
        GetLastError()?;
    }
    let client_name = PCWSTR::from_raw(client_name_buf.as_ptr()).to_string().unwrap();
    if DISALLOWED_TITLE.contains(&client_name) {
        return Ok(false);
    }

    let mut class_name_buf = [0u16; 256];
    SetLastError(WIN32_ERROR(0));
    if GetClassNameW(*hwnd, class_name_buf.as_mut()) == 0 {
        GetLastError()?;
    }
    let class_name = PCWSTR::from_raw(class_name_buf.as_ptr()).to_string().unwrap();
    if DISALLOWED_CLASS.contains(&class_name) {
        return Ok(false);
    }

    let parent = GetParent(*hwnd);
    let parent_exist = parent.0 != 0;
    let is_tool = has_flag!(exstyle, WS_EX_TOOLWINDOW.0);

    if !parent_exist {
        if is_tool {
            return Ok(false);
        } else {
            let result = IsWindowVisible(*hwnd) == TRUE;
            return Ok(result);
        }
    }

    if is_manageable(&parent)? == false {
        return Ok(false);
    }

    let is_app = has_flag!(exstyle, WS_EX_APPWINDOW.0);
    if is_tool || is_app {
        return Ok(true);
    }

    Ok(false)
}

unsafe extern "system" fn scan(hwnd: HWND, _: LPARAM) -> BOOL {
    if !is_manageable(&hwnd).unwrap() {
        return TRUE;
    }

    TRUE
}

unsafe fn setup(hinstance: &HINSTANCE) -> Result<()> {
    let wnd_class = WNDCLASSW {
        lpfnWndProc: Some(wnd_proc),
        hInstance: *hinstance,
        lpszClassName: W_APP_NAME,
        ..Default::default()
    };

    request_update_geom()?;

    EnumWindows(Some(scan), None)?;

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
