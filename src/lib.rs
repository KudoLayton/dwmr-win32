use windows::{
    core::*,
    Win32::{
        System::{
            Diagnostics::Debug::*, 
            Threading::*
        },
        Foundation::*,
        UI::{
            WindowsAndMessaging::*,
            Input::KeyboardAndMouse::*,
            Accessibility::*,
        },
        Graphics::{
            Dwm::*,
            Gdi::*
        }
    }
};

use std::{
    collections::*,
    mem::size_of,
    usize,
    cmp::*,
};

pub mod config;
use config::*;

#[cfg(test)]
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

const W_WALLPAPER_CLASS_NAME: PCWSTR = w!("Progman");
const BAR_HEIGHT: i32 = 20;

#[derive(Default, Clone, Debug)]
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

#[derive(Default, Debug)]
struct Monitor {
    name: [u16; 32], //LPCWSTR type
    master_count: u32,
    master_factor: f32,
    index: usize,
    bar_y: i32,
    rect: Rect,
    client_area: Rect,
    selected_hwnd: HWND,
    clients: Vec<Client> // Reversed order
}

impl Monitor {
    unsafe fn arrangemon(&mut self) -> Result<()> {
        self.tile()?;
        Ok(())
    }

    fn get_selected_client_index(&self) -> Option<usize> {
        let selected_hwnd = self.selected_hwnd;
        if selected_hwnd.0 == 0 {
            return None;
        }

        for (index, client) in self.clients.iter().enumerate() {
            if client.hwnd == selected_hwnd {
                return Some(index);
            }
        }
        None
    }

    unsafe fn is_tiled(client: &Client) -> bool {
        !client.is_floating
    }

    unsafe fn tile(&mut self) -> Result<()> {
        let clients = &mut self.clients;

        let mut tiled_count: u32 = 0;
        for client in clients.iter() {
            tiled_count += Self::is_tiled(client) as u32;
        }

        if tiled_count <= 0 {
            return Ok(());
        }

        //let mut master_width = 0;
        let mut master_y: u32 = 0;
        let mut stack_y: u32 = 0;

        let master_width = if tiled_count > self.master_count {
            if self.master_count > 0 {
                ((self.rect.width as f32) * self.master_factor) as i32
            } else {
                0
            }
        } else {
            self.rect.width
        };

        for (index, client) in clients.iter_mut().rev().enumerate() {
            if !Self::is_tiled(client) {
                continue;
            }

            let is_master = index < self.master_count as usize;
            let rect = if is_master {
                let height: u32 = (self.client_area.height as u32 - master_y) / (min(tiled_count, self.master_count) - (index as u32));
                Rect {
                    x: self.client_area.x,
                    y: self.client_area.y + master_y as i32,
                    width: master_width,
                    height: height as i32
                }
            } else {
                let height: u32 = (self.client_area.height as u32 - stack_y) / (tiled_count - (index as u32));
                Rect {
                    x: self.client_area.x + master_width as i32,
                    y: self.client_area.y + stack_y as i32,
                    width: self.client_area.width - master_width,
                    height: height as i32
                }
            };

            ShowWindow(client.hwnd, SW_NORMAL);
            SetWindowPos(
                client.hwnd,
                None,
                rect.x,
                rect.y,
                rect.width,
                rect.height,
                SET_WINDOW_POS_FLAGS(0)
            )?;

            client.rect = rect.clone();

            let next_y = (is_master as u32) * master_y + (!is_master as u32) * stack_y + rect.height as u32;
            if next_y >= self.client_area.height as u32 {
                continue;
            }

            if is_master  {
                master_y += rect.height as u32;
            } else{
                stack_y += rect.height as u32;
            }
        }

        Ok(())
    }
}

pub union Arg {
    i: i32,
    ui: u32,
    f: f32
}

pub struct Key {
    pub mod_key: HOT_KEY_MODIFIERS,
    pub key: char,
    pub func: unsafe fn(&mut DwmrApp, &Option<Arg>)->Result<()>,
    pub arg: Option<Arg>
}


#[derive(Default, Clone, Debug)]
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
    monitor: usize,
}

#[derive(Default, Debug)]
pub struct DwmrApp {
    hwnd: HWND,
    wallpaper_hwnd: HWND,
    monitors: Vec<Monitor>,
    selected_monitor_index: Option<usize>,
    event_hook: Vec<HWINEVENTHOOK>,
}

lazy_static! {
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

impl DwmrApp {
    pub unsafe fn setup(&mut self, hinstance: &HINSTANCE) -> Result<()> {
        self.request_update_geom()?;

        let wallpaper_hwnd = FindWindowW(W_WALLPAPER_CLASS_NAME, None);
        if wallpaper_hwnd.0 == 0 {
            GetLastError()?;
        }

        self.wallpaper_hwnd = wallpaper_hwnd;

        let wnd_class = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(Self::wnd_proc),
            hInstance: *hinstance,
            lpszClassName: W_APP_NAME.clone(),
            ..Default::default()
        };

        let class_atom = RegisterClassExW(&wnd_class);
        if class_atom == 0{
            GetLastError()?;
        }

        let hwnd_result = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            W_APP_NAME.clone(),
            W_APP_NAME.clone(),
            WINDOW_STYLE::default(),
            0,
            0,
            0,
            0,
            HWND_MESSAGE,
            None,
            None,
            Some(self as *mut _ as _)
        );

        if hwnd_result.0 == 0 {
            GetLastError()?;
        }

        self.event_hook.push(SetWinEventHook(EVENT_SYSTEM_FOREGROUND, EVENT_SYSTEM_FOREGROUND, None, Some(Self::window_event_hook_proc), 0, 0, WINEVENT_OUTOFCONTEXT));
        self.event_hook.push(SetWinEventHook(EVENT_OBJECT_SHOW, EVENT_OBJECT_SHOW, None, Some(Self::window_event_hook_proc), 0, 0, WINEVENT_OUTOFCONTEXT));
        self.event_hook.push(SetWinEventHook(EVENT_OBJECT_DESTROY, EVENT_OBJECT_DESTROY, None, Some(Self::window_event_hook_proc), 0, 0, WINEVENT_OUTOFCONTEXT));

        self.grab_keys()?;
        Ok(())
    }


    unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT
    {
        match msg {
            WM_CREATE => {
                let create_struct = lparam.0 as *const CREATESTRUCTW;
                let this = (*create_struct).lpCreateParams as *mut Self;
                (*this).hwnd = hwnd;
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, this as isize);
                LRESULT::default()
            }
            _ => {
                let this = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Self;
                match this.is_null() {
                    true => DefWindowProcW(hwnd, msg, wparam, lparam),
                    false => (*this).handle_message(hwnd, msg, wparam, lparam)
                }
            }
        }
    }

    unsafe fn handle_message(&mut self, hwnd:HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT
    {
        match msg {
            WM_CLOSE => {
                self.cleanup().unwrap();
                LRESULT::default()
            }
            WM_DESTROY => {
                PostQuitMessage(0);
                LRESULT::default()
            }
            WM_HOTKEY => {
                if wparam.0 < KEYS.len(){
                    let key = &KEYS[wparam.0];
                    (key.func)(self, &key.arg).unwrap();
                }
                LRESULT::default()
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam)
        }
    }

    unsafe extern "system" fn window_event_hook_proc(
        hwin_event_hook: HWINEVENTHOOK,
        event: u32,
        hwnd: HWND,
        id_object: i32,
        id_child: i32,
        id_event_thread: u32,
        dwms_event_time: u32
            
    ) {
        let self_hwnd = FindWindowW(W_APP_NAME, None);
        if self_hwnd.0 == 0 {
            GetLastError().unwrap();
        }
        let this = GetWindowLongPtrW(self_hwnd, GWLP_USERDATA) as *mut Self;
        if this.is_null() {
            return;
        }

        (*this).window_event_hook(hwin_event_hook, event, hwnd, id_object, id_child, id_event_thread, dwms_event_time);
    }

    unsafe fn window_event_hook (
        &mut self,
        hwin_event_hook: HWINEVENTHOOK,
        event: u32,
        hwnd: HWND,
        id_object: i32,
        id_child: i32,
        id_event_thread: u32,
        dwms_event_time: u32
    ) {
        match event {
            EVENT_SYSTEM_FOREGROUND => {
                self.set_focus(hwnd);
            }
            EVENT_OBJECT_SHOW => {
                if !Self::is_manageable(&hwnd).unwrap() {
                    return;
                }
                let client = self.manage(&hwnd).unwrap();
                self.monitors[client.monitor].arrangemon().unwrap();
            }
            EVENT_OBJECT_DESTROY => {
                self.unmanage(hwnd).unwrap();
            }
            _ => ()
        }
    }

    unsafe fn set_focus(&mut self, hwnd: HWND)
    {
        if let Some(selected_monitor_index) = self.selected_monitor_index {
            if hwnd == self.monitors[selected_monitor_index].selected_hwnd {
                return;
            }
        }

        for monitor in self.monitors.iter_mut() {
            for client in &monitor.clients {
                if client.hwnd == hwnd {
                    self.selected_monitor_index = Some(monitor.index);
                    monitor.selected_hwnd = hwnd;
                    return;
                }
            }
        } 
    }

    unsafe fn request_update_geom(&mut self) -> Result<()> {
        let monitors = GetSystemMetrics(SM_CMONITORS) as usize;
        self.monitors.reserve(monitors);

        let lparam = LPARAM(self as *mut _ as isize);
        if EnumDisplayMonitors(None, None, Some(Self::update_geom), lparam) == FALSE {
            return Ok(());
        }
        Ok(())
    }

    unsafe fn grab_keys(&self) -> Result<()> {
        if self.hwnd.0 == 0 {
            return Ok(());
        }

        for (index, key) in KEYS.iter().enumerate() {
            RegisterHotKey(self.hwnd, index as i32, key.mod_key, key.key as u32)?;
        }
        Ok(())
    }

    unsafe extern "system" fn update_geom(hmonitor: HMONITOR, _: HDC, rect: *mut RECT, lparam: LPARAM) -> BOOL {
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

        let _monitor_name = PCWSTR::from_raw(monitor_info.szDevice.as_ptr()).to_string().unwrap();

        let this = lparam.0 as *mut DwmrApp;

        let monitor = Monitor{
            name: monitor_info.szDevice,
            index: (*this).monitors.len(),
            rect: Rect::from_win_rect(&monitor_info.monitorInfo.rcMonitor),
            client_area: Rect::from_win_rect(&monitor_info.monitorInfo.rcWork),
            master_count: 1,
            master_factor: 0.5,
            ..Default::default()
        };

        (*this).monitors.push(monitor);
        TRUE
    }

    unsafe fn refresh_current_focus(&mut self) -> Result<()> {
        let focus_hwnd = GetForegroundWindow();
        let mut selected_index: Option<usize> = None;
        for (monitor_index, monitor) in self.monitors.iter_mut().enumerate() {
            for (index, client) in monitor.clients.iter().enumerate() {
                if client.hwnd != focus_hwnd {
                    continue;
                }

                self.selected_monitor_index = Some(monitor_index);
                monitor.selected_hwnd = focus_hwnd;
                selected_index = Some(index);
                break;
            }

            if selected_index.is_none() {
                continue;
            }

            break;
        }
        Ok(())
    }

    pub unsafe fn scan(&mut self) -> Result<()> {
        EnumWindows(Some(Self::scan_enum), LPARAM(self as *mut _ as isize))?;

        self.refresh_current_focus()?;
        let selected_monitor = &mut self.monitors[self.selected_monitor_index.unwrap()];

        let selected_client_index = selected_monitor.get_selected_client_index();
        if selected_client_index.is_none() {
            return Ok(());
        }

        let selected_client = selected_monitor.clients[selected_client_index.unwrap()].clone();
        selected_monitor.clients.remove(selected_client_index.unwrap());
        selected_monitor.clients.push(selected_client);

        Ok(())
    }

    unsafe extern "system" fn scan_enum(hwnd: HWND, lparam: LPARAM) -> BOOL {
        if !Self::is_manageable(&hwnd).unwrap() {
            return TRUE;
        }

        let this = lparam.0 as *mut Self;
        if this.is_null() {
            return TRUE;
        }

        (*this).manage(&hwnd).unwrap();
        TRUE
    }

    unsafe fn is_cloaked(hwnd: &HWND) -> Result<bool> {
        let mut cloaked_val = 0;
        DwmGetWindowAttribute(*hwnd, DWMWA_CLOAKED, (&mut cloaked_val) as *const _ as *mut _, size_of::<u32>() as u32)?;
        let is_cloaked = cloaked_val > 0;

        Ok(is_cloaked)
    }

    unsafe fn is_debugged(hwnd: &HWND) -> Result<bool> {
        let mut process_id: u32 = 0;
        if GetWindowThreadProcessId(*hwnd, Some(&mut process_id as *mut _)) == 0 {
            GetLastError()?;
        }

        let handle = OpenProcess(PROCESS_QUERY_INFORMATION, FALSE, process_id);
        if let Err(ref e) = handle {
            if e.code() != HRESULT::from(ERROR_ACCESS_DENIED) {
                return Err(e.clone());
            } else {
                return Ok(false);
            }
        }

        let mut is_debugged = FALSE;
        CheckRemoteDebuggerPresent(handle?, &mut is_debugged)?;
        if is_debugged == TRUE {
            return Ok(true);
        } else {
            return Ok(false);
        }
    }

    unsafe fn is_manageable(hwnd: &HWND) -> Result<bool> {
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

        if Self::is_cloaked(hwnd)? {
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

        if EXCLUDE_DEBUGGED_WINDOW {
            if Self::is_debugged(hwnd)? {
                return Ok(false);
            }
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

        if Self::is_manageable(&parent)? == false {
            return Ok(false);
        }

        let is_app = has_flag!(exstyle, WS_EX_APPWINDOW.0);
        if is_tool || is_app {
            return Ok(true);
        }

        Ok(false)
    }

    unsafe fn get_root(hwnd: &HWND) -> Result<HWND> {
        let desktop_window = GetDesktopWindow();
        let mut current = hwnd.clone();
        let mut parent = GetWindow(current, GW_OWNER);

        while (parent.0 != 0) && (parent != desktop_window) {
            current = parent;
            parent = GetWindow(current, GW_OWNER);
        }

        Ok(current)
    }

    unsafe fn manage(&mut self, hwnd: &HWND) -> Result<Client> {
        let mut window_info = WINDOWINFO {
            cbSize: size_of::<WINDOWINFO>() as u32,
            ..Default::default()
        };

        GetWindowInfo(*hwnd, &mut window_info)?;

        let parent = GetParent(*hwnd);
        let root = Self::get_root(hwnd)?;
        let is_cloaked = Self::is_cloaked(hwnd)?;
        let is_minimized = IsIconic(*hwnd) == TRUE;
        let rect = Rect::from_win_rect(&window_info.rcWindow);
        let center_x = rect.x + rect.width / 2;
        let center_y = rect.y + rect.height / 2;

        assert!(!self.monitors.is_empty());

        let mut monitor_index:usize = 0;
        for (index, monitor_iter) in self.monitors.iter().enumerate() {
            let monitor_rect = &monitor_iter.rect;

            let left_check = monitor_rect.x <= center_x;
            let right_check = center_x <= monitor_rect.x + monitor_rect.width;
            let top_check = monitor_rect.y <= center_y;
            let bottom_check = center_y <= monitor_rect.y + monitor_rect.height;

            if left_check && right_check && top_check && bottom_check {
                monitor_index = index;
            }
        }

        let client = Client {
            hwnd: *hwnd,
            parent,
            root,
            rect: rect.into(),
            bw: 0,
            is_minimized,
            is_cloaked,
            monitor: monitor_index,
            ..Default::default()
        };
        self.monitors[monitor_index].clients.push(client.clone());

        Ok(client)
    }

    unsafe fn unmanage(&mut self, hwnd: HWND) -> Result<()> {
        for monitor in self.monitors.iter_mut() {
            let mut found_index: Option<usize> = None;
            for (index, client) in monitor.clients.iter().enumerate() {
                if client.hwnd == hwnd {
                    found_index = Some(index);
                    break;
                }
            }

            if let Some(index) = found_index {
                monitor.clients.remove(index);
                monitor.arrangemon()?;
                return Ok(());
            }
        }

        Ok(())
    }

    pub unsafe fn arrange(&mut self) -> Result<()> {
        for monitor in self.monitors.iter_mut() {
            monitor.arrangemon()?;
        }

        Ok(())
    }

    pub unsafe fn cleanup(&mut self) -> Result<()> {
        for event_hook in self.event_hook.iter() {
            if event_hook.0 != 0 {
                UnhookWinEvent(*event_hook);
            }
        }
        self.event_hook.clear();

        if self.hwnd.0 == 0 {
            return Ok(());
        }

        for key_index in 0..KEYS.len() {
            UnregisterHotKey(self.hwnd, key_index as i32)?;
        }

        DestroyWindow(self.hwnd)?;
        self.hwnd = HWND::default();

        Ok(())
    }

    pub unsafe fn run() -> Result<()> {
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0) == TRUE {
            TranslateMessage(&mut msg);
            DispatchMessageW(&mut msg);
        }
        Ok(())
    }

    pub unsafe fn quit(&mut self, _: &Option<Arg>) -> Result<()> {
        if self.hwnd.0 == 0 {
            return Ok(());
        }

        PostMessageW(self.hwnd, WM_CLOSE, WPARAM(0), LPARAM(0))?;
        Ok(())
    }

    fn offset_to_new_index(length: usize, current_index: usize, offset_index: i32) -> usize {
        let is_underfloor = (current_index as i32 - offset_index) < 0;
        let is_overfloor = (current_index as i32 - offset_index) >= (length as i32);

        match (is_underfloor, is_overfloor) {
            (true, false) => (length - 1) as usize,
            (false, true) => 0 as usize,
            _ => (current_index as i32 - offset_index) as usize
        }
    }

    unsafe fn focus(hwnd: &HWND) -> Result<()> {
        let result = SetForegroundWindow(*hwnd);
        if result.0 == 0 {
            GetLastError()?;
        }
        Ok(())
    }

    pub unsafe fn focus_stack(&mut self, arg: &Option<Arg>) -> Result<()> {
        if arg.is_none() {
            return Ok(());
        }

        let offset = arg.as_ref().unwrap().i;

        if offset == 0 {
            return Ok(());
        }

        let selected_monitor = self.monitors.get_mut(self.selected_monitor_index.unwrap()).unwrap();
        let selected_client_index_option = selected_monitor.get_selected_client_index();

        if selected_client_index_option.is_none() {
            return Ok(());
        }

        let selected_client_index = selected_client_index_option.unwrap();
        let clients_count = selected_monitor.clients.len();
        if clients_count == 0 {
            return Ok(());
        }

        let new_focus_index = Self::offset_to_new_index(clients_count, selected_client_index, offset);
        if new_focus_index == selected_client_index {
            return Ok(());
        }

        let new_focus_hwnd = selected_monitor.clients[new_focus_index].hwnd;
        selected_monitor.selected_hwnd = new_focus_hwnd;
        Self::focus(&new_focus_hwnd)?;
        Ok(())
    }

    pub unsafe fn zoom(&mut self, _: &Option<Arg>) -> Result<()> {
        let selected_monitor = self.monitors.get_mut(self.selected_monitor_index.unwrap()).unwrap();
        let selected_client_index_option = selected_monitor.get_selected_client_index();

        if selected_client_index_option.is_none() {
            return Ok(());
        }

        let selected_client_idnex = selected_client_index_option.unwrap();

        let clients = &mut selected_monitor.clients;
        let selected_client = clients[selected_client_idnex].clone();
        clients.remove(selected_client_idnex);
        clients.push(selected_client);

        selected_monitor.arrangemon()?;

        Ok(())
    }

    unsafe fn unfocus() -> Result<()> {
        let desktop_hwnd = FindWindowW(W_WALLPAPER_CLASS_NAME, None);
        if desktop_hwnd.0 == 0 {
            GetLastError()?;
        }

        let result = SetForegroundWindow(desktop_hwnd);
        if result.0 == 0 {
            GetLastError()?;
        }

        Ok(())
    }

    unsafe fn refresh_focus(&self) -> Result<()> {
        let selected_monitor = &self.monitors[self.selected_monitor_index.unwrap()];
        if selected_monitor.clients.len() == 0 {
            Self::unfocus()?;
            return Ok(());
        }

        let selected_client_option = selected_monitor.get_selected_client_index();
        if selected_client_option.is_none() {
            Self::unfocus()?;
            return Ok(());
        }

        let selected_client_hwnd = selected_monitor.clients[selected_client_option.unwrap()].hwnd;
        Self::focus(&selected_client_hwnd)?;

        Ok(())
    }

    pub unsafe fn focus_monitor(&mut self, arg: &Option<Arg>) -> Result<()>
    {
        if self.monitors.len() == 0 {
            return Ok(());
        }

        if arg.is_none() {
            return Ok(());
        }

        let index_offset = arg.as_ref().unwrap().i;

        if index_offset == 0 {
            return Ok(());
        }

        if self.selected_monitor_index.is_none() {
            self.selected_monitor_index = Some(0);
        } else {
            let current_selected_index = self.selected_monitor_index.unwrap();
            let new_index = Self::offset_to_new_index(self.monitors.len(), current_selected_index, index_offset);
            if new_index == current_selected_index {
                return Ok(());
            }
            self.selected_monitor_index = Some(new_index);
        }

        let selected_monitor = &mut self.monitors[self.selected_monitor_index.unwrap()];
        let selected_hwnd = &mut selected_monitor.selected_hwnd;
        if selected_hwnd.0 == 0 {
            let clients = &selected_monitor.clients;
            *selected_hwnd = match clients.last() {
                Some(client) => client.hwnd,
                None => HWND::default()
            };
        }

        self.refresh_focus()?;

        Ok(())
    }
}

