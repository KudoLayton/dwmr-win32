#![windows_subsystem = "windows"]
use dwmr_win32::*;
use windows::{
    core::*,
    Win32::{
        Foundation::*,
        System::LibraryLoader::*,
    }
};

fn main() -> Result<()> {
    unsafe{
        let hmodule = GetModuleHandleW(None)?;
        let hinstance: HINSTANCE = hmodule.into();
        let mut app = DwmrApp::default();
        app.setup(&hinstance)?;
        app.scan()?;
        app.arrange()?;
        DwmrApp::run()?;
    }
    Ok(())
}
