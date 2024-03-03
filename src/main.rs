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
        setup(&hinstance)?;
        scan()?;
        arrange()?;
        cleanup()?; 
    }
    Ok(())
}
