use windows::{
    core::*,
    Win32::{
        Foundation::*,
        System::LibraryLoader::*,
        UI::WindowsAndMessaging::*,
    }
};
use std::sync::*;
#[macro_use]
extern  crate lazy_static;

const W_APP_NAME: PCWSTR = w!("dwmr-win32");
const S_APP_NAME: PCSTR = s!("dwmr-win32");

#[derive(Default)]
struct DwmrApp {
    hwnd: RwLock<Option<HWND>>,
}

lazy_static! {
    static ref DWMR_APP: Arc<DwmrApp> = Arc::new(DwmrApp::default());
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT
{
    LRESULT::default()
}

unsafe fn setup(hinstance: &HINSTANCE) -> Result<()> {
    let wnd_class = WNDCLASSW {
        lpfnWndProc: Some(wnd_proc),
        hInstance: *hinstance,
        lpszClassName: W_APP_NAME,
        ..Default::default()
    };

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
    let mut hwnd = DWMR_APP.hwnd.write().unwrap();
    DestroyWindow((*hwnd).unwrap())?;
    *hwnd = None;

    UnregisterClassW(W_APP_NAME, *hinstance)?;

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
