use windows::{
    core::*,
    Foundation::Numerics::*,
    Win32::{
        UI::HiDpi::*,
        System::{
            Diagnostics::Debug::*, 
            Threading::*, 
            ProcessStatus::*,
        },
        Foundation::*,
        UI::{
            WindowsAndMessaging::*,
            Input::KeyboardAndMouse::*,
            Accessibility::*,
        },
        Graphics::{
            Dwm::*,
            Gdi::*,
            Direct2D::{*, Common::D2D1_ALPHA_MODE_PREMULTIPLIED},
            Dxgi::Common::*,
            DirectWrite::*,
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
pub mod graphic_utils;

use config::*;
use graphic_utils::*;

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
const W_BAR_NAME: PCWSTR = w!("dwmr-bar");

const W_WALLPAPER_CLASS_NAME: PCWSTR = w!("Progman");
const BAR_HEIGHT: i32 = 20;
const TAGMASK: u32 = (1 << TAGS.len()) - 1;
const DEFAULT_DPI : f32 = 96.0;

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

impl PartialEq for Rect {
    fn eq(&self, other: &Self) -> bool {
        (self.x == other.x) && (self.y == other.y) && (self.width == other.width) && (self.height == other.height)
    }
}

impl Eq for Rect {}

#[derive(Default, Debug)]
struct Bar {
    hwnd: HWND,
    rect: Rect,
    is_selected_monitor: bool,
    render_target: Option<ID2D1HwndRenderTarget>,
    unselected_text_brush: Option<ID2D1SolidColorBrush>,
    selected_text_brush: Option<ID2D1SolidColorBrush>,
    text_box_brush: Option<ID2D1SolidColorBrush>,
    background_brush: Option<ID2D1SolidColorBrush>,
    text_format: Option<IDWriteTextFormat>,
    write_factory: Option<IDWriteFactory>,
    dpi: f32,
    selected_tags: u32,
}

impl Bar {
    pub unsafe fn setup_bar(&mut self, display_rect: &Rect) -> Result<()> {
        let focus_hwnd = GetForegroundWindow();

        let hwnd_result = CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_LAYERED,
            W_BAR_NAME.clone(),
            W_BAR_NAME.clone(),
            WS_POPUP | WS_CLIPCHILDREN | WS_CLIPSIBLINGS,
            display_rect.x,
            display_rect.y,
            display_rect.width,
            BAR_HEIGHT as i32,
            None,
            None,
            None,
            Some(self as *const _ as _)
        );

        if hwnd_result.0 == 0 {
            GetLastError()?;
        }

        SetLayeredWindowAttributes(hwnd_result, COLORREF(0), (255 as f32 * BAR_TRANSPARENCY) as u8, LWA_ALPHA)?;

        self.hwnd = hwnd_result;
        self.rect = Rect {
            x: 0,
            y: 0,
            width: display_rect.width,
            height: BAR_HEIGHT
        };
        self.dpi = GetDpiForWindow(hwnd_result) as f32 / 96.0;

        let factory = D2D1CreateFactory::<ID2D1Factory>(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)?;
        let render_target_property = D2D1_RENDER_TARGET_PROPERTIES {
            r#type: D2D1_RENDER_TARGET_TYPE_DEFAULT,
            pixelFormat: Common::D2D1_PIXEL_FORMAT {
                format: DXGI_FORMAT_B8G8R8A8_UNORM,
                alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
            },
            dpiX: 0.0,
            dpiY: 0.0,
            usage: D2D1_RENDER_TARGET_USAGE_NONE,
            minLevel: D2D1_FEATURE_LEVEL_DEFAULT,
        };

        let hwnd_render_target_properties = D2D1_HWND_RENDER_TARGET_PROPERTIES {
            hwnd: hwnd_result,
            pixelSize: Common::D2D_SIZE_U {
                width: 1920,
                height: BAR_HEIGHT as u32,
            },
            presentOptions: D2D1_PRESENT_OPTIONS_NONE,
        };
        let render_target = factory.CreateHwndRenderTarget(&render_target_property, &hwnd_render_target_properties)?;

        let brush_property = D2D1_BRUSH_PROPERTIES { 
            opacity: 1.0, 
            transform: Matrix3x2::identity()
        };

        let background_brush = render_target.CreateSolidColorBrush(&BAR_COLOR_BACKGROUND, Some(&brush_property as *const _))?;
        let selected_box_brush = render_target.CreateSolidColorBrush(&BAR_COLOR_SELECTED_BOX, Some(&brush_property as *const _))?;
        let selected_text_brush = render_target.CreateSolidColorBrush(&BAR_COLOR_SELECTED_TEXT, Some(&brush_property as *const _))?;
        let unselected_text_brush = render_target.CreateSolidColorBrush(&BAR_COLOR_UNSELECTED_TEXT, Some(&brush_property as *const _))?;
        self.render_target = Some(render_target);
        self.unselected_text_brush = Some(unselected_text_brush.clone());
        self.selected_text_brush = Some(selected_text_brush);
        self.text_box_brush = Some(selected_box_brush);
        self.background_brush = Some(background_brush);

        let write_factory = DWriteCreateFactory::<IDWriteFactory>(DWRITE_FACTORY_TYPE_ISOLATED)?;
        let text_format = write_factory.CreateTextFormat(
            w!("Arial"), 
            None, 
            DWRITE_FONT_WEIGHT_REGULAR, 
            DWRITE_FONT_STYLE_NORMAL, 
            DWRITE_FONT_STRETCH_NORMAL,
            20.0, 
            w!("ko-kr"))?;

        self.write_factory = Some(write_factory);
        self.text_format = Some(text_format);

        ShowWindow(hwnd_result, SW_SHOW);
        UpdateWindow(hwnd_result);
        SetForegroundWindow(focus_hwnd);
        Ok(())
    }

    unsafe extern "system" fn bar_wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        match msg {
            WM_CREATE => {
                let create_struct = lparam.0 as *const CREATESTRUCTW;
                let this = (*create_struct).lpCreateParams as *mut Bar;
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, this as isize);
                LRESULT::default()
            }
            WM_DESTROY => {
                PostQuitMessage(0);
                LRESULT::default()
            }
            WM_PAINT => {
                let this = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Bar;
                if this.is_null() {
                    return LRESULT::default();
                }

                let mut ps = PAINTSTRUCT::default();
                let _hdc = BeginPaint(hwnd, &mut ps);
                    (*this).draw().unwrap();
                EndPaint(hwnd, &ps);
                LRESULT::default()
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam)
        }
    }

    unsafe fn draw(&self) -> Result<()> {
        if self.hwnd.0 == 0 {
            return Ok(());
        }

        if self.render_target.is_none() || self.text_box_brush.is_none() || self.text_format.is_none(){
            return Ok(());
        }

        let render_target_ref = self.render_target.as_ref().unwrap();
        render_target_ref.BeginDraw();

        render_target_ref.Clear(Some(&BAR_COLOR_BACKGROUND));

        let mut x_pos = 0.0;
        for i in 0..TAGS.len() {
            x_pos = match (has_flag!(self.selected_tags, 1 << i), self.is_selected_monitor) {
                (true, true ) => self.draw_selected_monitor_selected_text_box(TAGS[i].as_wide(), 15.0, x_pos)?,
                (true, false) => self.draw_unselected_monitor_selected_text_box(TAGS[i].as_wide(), 15.0, x_pos)?,
                (false, _   ) => self.draw_unselected_text_box(TAGS[i].as_wide(), 15.0, x_pos)?

            };
            x_pos += 5.0;
        }

        render_target_ref.EndDraw(None, None)?;

        Ok(())
    }

    unsafe fn draw_unselected_text_box(&self, text: &[u16], font_size: f32, origin_x: f32) -> Result<f32> 
    {
        let next_width = implement_draw_text_box(
            text, 
            font_size, 
            self.rect.width as f32, 
            self.rect.height as f32, 
            origin_x, 
            self.rect.y as f32,
            self.dpi, 
            self.text_format.as_ref().unwrap(), 
            self.write_factory.as_ref().unwrap(), 
            self.render_target.as_ref().unwrap(), 
            self.background_brush.as_ref().unwrap(),
            self.unselected_text_brush.as_ref().unwrap())?;
        Ok(next_width)
    }

    unsafe fn draw_selected_monitor_selected_text_box(&self, text: &[u16], font_size: f32, origin_x: f32) -> Result<f32> 
    {
        let next_width = implement_draw_text_box(
            text, 
            font_size, 
            self.rect.width as f32, 
            self.rect.height as f32, 
            origin_x, 
            self.rect.y as f32,
            self.dpi, 
            self.text_format.as_ref().unwrap(), 
            self.write_factory.as_ref().unwrap(), 
            self.render_target.as_ref().unwrap(), 
            self.text_box_brush.as_ref().unwrap(),
            self.selected_text_brush.as_ref().unwrap())?;
        Ok(next_width)
    }

    unsafe fn draw_unselected_monitor_selected_text_box(&self, text: &[u16], font_size: f32, origin_x: f32) -> Result<f32> 
    {
        let next_width = implement_draw_text_box(
            text, 
            font_size, 
            self.rect.width as f32, 
            self.rect.height as f32, 
            origin_x, 
            self.rect.y as f32,
            self.dpi, 
            self.text_format.as_ref().unwrap(), 
            self.write_factory.as_ref().unwrap(), 
            self.render_target.as_ref().unwrap(), 
            self.unselected_text_brush.as_ref().unwrap(),
            self.selected_text_brush.as_ref().unwrap())?;
        Ok(next_width)
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
    clients: Vec<Client>, // Reversed order
    tagset: [u32; 2],
    selected_tag_index: usize,
    bar: Bar,
    layout: Layout,
}

impl Monitor {
    unsafe fn arrangemon(&mut self) -> Result<()> {
        self.show_hide()?;
        let layout = self.layout.clone();
        layout.unwrap().arrange_layout(self)?;
        Ok(())
    }

    unsafe fn show_hide(&mut self) -> Result<()> {
        for client in self.clients.iter_mut() {
            let is_visible = Self::is_visible(client, self.tagset[self.selected_tag_index]);
            let is_window_visible = IsWindowVisible(client.hwnd) == TRUE;
            if is_visible && !is_window_visible {
                client.is_hide = false;
                ShowWindow(client.hwnd, SW_NORMAL);
            }

            if !is_visible && is_window_visible {
                client.is_hide = true;
                ShowWindow(client.hwnd, SW_HIDE);
            }
        }
        Ok(())
    }

    pub fn find_client_index(&self, hwnd: &HWND) -> Option<usize> {
        if hwnd.0 == 0 {
            return None;
        }
        self.clients.iter().position(|client| client.hwnd == *hwnd)
    }

    fn get_selected_client_index(&self) -> Option<usize> {
        let selected_hwnd = self.selected_hwnd;
        if selected_hwnd.0 == 0 {
            return None;
        }

        return self.find_client_index(&selected_hwnd);
    }

    pub fn is_visible(client: &Client, visible_tags: u32) -> bool {
        return (visible_tags & client.tags) != 0
    }

    pub fn visible_clinets_count(&self) -> i32 {
        let mut count = 0;
        for client in self.clients.iter() {
            if Self::is_visible(client, self.tagset[self.selected_tag_index]) {
                count += 1;
            }
        }
        return count;
    }

    unsafe fn is_tiled(client: &Client, visible_tags: u32) -> bool {
        (!client.is_floating) && Self::is_visible(client, visible_tags)
    }

    pub unsafe fn sanitize_clients(&mut self) -> Result<()> {
        self.clients.retain(|client| IsWindow(client.hwnd) == TRUE);
        Ok(())
    }
}

trait LayoutTrait {
    unsafe fn arrange_layout(&self, monitor: &mut Monitor) -> Result<()>;
    fn is_in_master_area(&self, monitor: &Monitor, x: i32, y: i32) -> bool;
    unsafe fn resize(&self, hwnd: &HWND, rect: &Rect) -> Result<()> {
        ShowWindow(*hwnd, SW_NORMAL);
        SetWindowPos(
            *hwnd,
            None,
            rect.x,
            rect.y,
            rect.width,
            rect.height,
            SET_WINDOW_POS_FLAGS(0)
        )?;

        let mut result_rect = RECT::default();
        GetWindowRect(*hwnd, &mut result_rect)?;
        let window_pos_result_rect = Rect::from_win_rect(&result_rect);
        if window_pos_result_rect != *rect {
            SetWindowPos(
                *hwnd,
                None,
                rect.x,
                rect.y,
                rect.width,
                rect.height,
                SET_WINDOW_POS_FLAGS(0)
            )?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
enum Layout {
    Tile(TileLayout),
    Stack(StackLayout)
}

impl Layout {
    fn unwrap(&self) -> &dyn LayoutTrait {
        match self {
            Layout::Tile(tile) => tile,
            Layout::Stack(stack) => stack
        }
    }
}

impl Default for Layout {
    fn default() -> Self {
        Layout::Tile(TileLayout::default())
    }
}

#[derive(Default, Debug, Clone, Copy)]
struct TileLayout;

impl LayoutTrait for TileLayout {
    unsafe fn arrange_layout(&self, monitor: &mut Monitor) -> Result<()> {
        let visible_tags = monitor.tagset[monitor.selected_tag_index];
        let clients = &mut monitor.clients;

        let mut tiled_count: u32 = 0;
        for client in clients.iter() {
            tiled_count += Monitor::is_tiled(client, visible_tags) as u32;
        }

        if tiled_count <= 0 {
            return Ok(());
        }

        let mut master_y: u32 = 0;
        let mut stack_y: u32 = 0;

        let master_width = if tiled_count > monitor.master_count {
            if monitor.master_count > 0 {
                ((monitor.client_area.width as f32) * monitor.master_factor) as i32
            } else {
                0
            }
        } else {
            monitor.rect.width
        };

        let mut index = 0;
        for client in clients.iter_mut().rev() {
            if !Monitor::is_tiled(client, visible_tags) {
                continue;
            }

            let is_master = index < monitor.master_count as usize;
            let rect = if is_master {
                let height: u32 = (monitor.client_area.height as u32 - master_y) / (min(tiled_count, monitor.master_count) - (index as u32));
                Rect {
                    x: monitor.client_area.x,
                    y: monitor.client_area.y + master_y as i32,
                    width: master_width,
                    height: height as i32
                }
            } else {
                let height: u32 = (monitor.client_area.height as u32 - stack_y) / (tiled_count - (index as u32));
                Rect {
                    x: monitor.client_area.x + master_width as i32,
                    y: monitor.client_area.y + stack_y as i32,
                    width: monitor.client_area.width - master_width,
                    height: height as i32
                }
            };
            index += 1;
            self.resize(&client.hwnd, &rect)?;
            client.rect = rect.clone();

            let next_y = (is_master as u32) * master_y + (!is_master as u32) * stack_y + rect.height as u32;
            if next_y >= monitor.client_area.height as u32 {
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

    fn is_in_master_area(&self, monitor: &Monitor, x: i32, _y: i32) -> bool {
        let threshold = monitor.rect.x + ((monitor.rect.width as f32 * monitor.master_factor) as i32);
        x < threshold
    }
}

#[derive(Default, Debug, Clone, Copy)]
struct StackLayout;

impl LayoutTrait for StackLayout {
    unsafe fn arrange_layout(&self, monitor: &mut Monitor) -> Result<()> {
        let visible_tags = monitor.tagset[monitor.selected_tag_index];
        let clients = &mut monitor.clients;

        let mut tiled_count: u32 = 0;
        for client in clients.iter() {
            tiled_count += Monitor::is_tiled(client, visible_tags) as u32;
        }

        if tiled_count <= 0 {
            return Ok(());
        }

        let mut stack_y: u32 = 0;

        let master_height = match (tiled_count > monitor.master_count, monitor.master_count > 0) {
            (true, true) => ((monitor.client_area.height as f32) * monitor.master_factor) as i32,
            (true, false) => 0,
            (false, _) => monitor.client_area.height
        };

        //let stack_height = monitor.client_area.height - master_height;

        let mut index = 0;
        for client in clients.iter_mut().rev() {
            if !Monitor::is_tiled(client, visible_tags) {
                continue;
            }

            let is_master = index < monitor.master_count as usize;

            let height = if is_master {
                (master_height as u32 - stack_y) / (min(tiled_count, monitor.master_count) - (index as u32))
            } else {
                (monitor.client_area.height as u32 - stack_y) / (tiled_count - (index as u32))
            };

            let rect = Rect {
                x: monitor.client_area.x,
                y: monitor.client_area.y + stack_y as i32,
                width: monitor.client_area.width,
                height: height as i32
            };

            index += 1;
            self.resize(&client.hwnd, &rect)?;
            client.rect = rect.clone();

            stack_y += rect.height as u32;
        }
        Ok(())
    }

    fn is_in_master_area(&self, monitor: &Monitor, _x: i32, y: i32) -> bool {
        let threshold = monitor.client_area.y + ((monitor.client_area.height as f32 * monitor.master_factor) as i32);
        y < threshold
    }
}

pub union Arg {
    i: i32,
    ui: u32,
    f: f32,
    l: Layout
}

pub struct Key {
    pub mod_key: HOT_KEY_MODIFIERS,
    pub key: char,
    pub func: unsafe fn(&mut DwmrApp, &Option<Arg>)->Result<()>,
    pub arg: Option<Arg>
}


#[derive(Default, Clone, Debug)]
pub struct Client {
    hwnd: HWND,
    title: String,
    class: String,
    process_filename: String,
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
    is_fixed: bool,
    is_urgent: bool,
    is_cloaked: bool,
    is_hide: bool,
    monitor: usize,
}

pub struct Rule {
    title: Option<String>,
    class: Option<String>,
    process_filename: Option<String>,
    is_floating: bool,
    tags: u32
}

impl Rule {
    pub fn is_match(&self, client: &Client) -> bool {
        if self.title.is_some() && self.title.as_ref().unwrap() != &client.title {
            return false;
        }

        if self.class.is_some() && self.class.as_ref().unwrap() != &client.class {
            return false;
        }

        if self.process_filename.is_some() && !client.process_filename.contains(self.process_filename.as_ref().unwrap()) {
            return false;
        }
        true
    }
}

#[derive(Default, Debug)]
pub struct DwmrApp {
    hwnd: HWND,
    wallpaper_hwnd: HWND,
    monitors: Vec<Monitor>,
    selected_monitor_index: Option<usize>,
    event_hook: Vec<HWINEVENTHOOK>,
    bar: Bar,
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
        "OleMainThreadWndClass".to_string(),
        "Xaml_WindowedPopupClass".to_string(),
        "LivePreview".to_string(),
        "TaskListOverlayWnd".to_string(),
        "Shell_TrayWnd".to_string(),
        "TopLevelWindowForOverflowXamlIsland".to_string(),
    ]);

}

impl DwmrApp {
    pub unsafe fn setup(&mut self, hinstance: &HINSTANCE) -> Result<()> {
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

        let bar_wnd_class = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(Bar::bar_wnd_proc),
            hInstance: *hinstance,
            lpszClassName: W_BAR_NAME.clone(),
            hbrBackground: HBRUSH((COLOR_WINDOW.0 + 1) as isize),
            ..Default::default()
        };

        let bar_class_atom = RegisterClassExW(&bar_wnd_class);
        if bar_class_atom == 0{
            GetLastError()?;
        }

        self.request_update_geom()?;

        let wallpaper_hwnd = FindWindowW(W_WALLPAPER_CLASS_NAME, None);
        if wallpaper_hwnd.0 == 0 {
            GetLastError()?;
        }
        self.wallpaper_hwnd = wallpaper_hwnd;

        self.event_hook.push(SetWinEventHook(EVENT_SYSTEM_FOREGROUND, EVENT_SYSTEM_FOREGROUND, None, Some(Self::window_event_hook_proc), 0, 0, WINEVENT_OUTOFCONTEXT));
        self.event_hook.push(SetWinEventHook(EVENT_OBJECT_SHOW, EVENT_OBJECT_HIDE, None, Some(Self::window_event_hook_proc), 0, 0, WINEVENT_OUTOFCONTEXT));
        self.event_hook.push(SetWinEventHook(EVENT_OBJECT_DESTROY, EVENT_OBJECT_DESTROY, None, Some(Self::window_event_hook_proc), 0, 0, WINEVENT_OUTOFCONTEXT));
        self.event_hook.push(SetWinEventHook(EVENT_SYSTEM_MOVESIZEEND, EVENT_SYSTEM_MOVESIZEEND, None, Some(Self::window_event_hook_proc), 0, 0, WINEVENT_OUTOFCONTEXT));
        self.event_hook.push(SetWinEventHook(EVENT_OBJECT_CLOAKED, EVENT_OBJECT_UNCLOAKED, None, Some(Self::window_event_hook_proc), 0, 0, WINEVENT_OUTOFCONTEXT));

        self.grab_keys()?;

        Ok(())
    }


    unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
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

    unsafe fn handle_message(&mut self, hwnd:HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        match msg {
            WM_CLOSE => {
                DestroyWindow(self.hwnd).unwrap();
                LRESULT::default()
            }
            WM_DESTROY => {
                self.cleanup().unwrap();
                PostQuitMessage(0);
                LRESULT::default()
            }
            WM_HOTKEY => {
                self.sanitize_monitors().unwrap();
                let tag_keys_sub_len = TAG_KEYS.first().unwrap().len();
                let tag_keys_len = TAG_KEYS.len() * tag_keys_sub_len;
                if wparam.0 < KEYS.len(){
                    let key = &KEYS[wparam.0];
                    (key.func)(self, &key.arg).unwrap();
                } else if wparam.0 < KEYS.len() + tag_keys_len {
                    let tag_key_index = wparam.0 - KEYS.len();
                    let tag_key_first_index = tag_key_index / tag_keys_sub_len;
                    let tag_key_second_index = tag_key_index % tag_keys_sub_len;
                    let key = &TAG_KEYS[tag_key_first_index][tag_key_second_index];
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
        _hwin_event_hook: HWINEVENTHOOK,
        event: u32,
        hwnd: HWND,
        id_object: i32,
        id_child: i32,
        _id_event_thread: u32,
        _dwms_event_time: u32
    ) {
        let is_target = (id_object == OBJID_WINDOW.0) && (id_child == CHILDID_SELF as i32);
        if !is_target {
            return;
        }

        if IsWindow(hwnd) == FALSE {
            return;
        }

        let mut client_name_buf = [0u16; 256];
        GetWindowTextW(hwnd, client_name_buf.as_mut());
        let client_name = PCWSTR::from_raw(client_name_buf.as_ptr()).to_string().unwrap();

        let mut class_name_buf = [0u16; 256];
        if GetClassNameW(hwnd, class_name_buf.as_mut()) == 0 {
            SetLastError(WIN32_ERROR(0));
            return;
        }
        let class_name = PCWSTR::from_raw(class_name_buf.as_ptr()).to_string().unwrap();
        SetLastError(WIN32_ERROR(0));

        let is_disallowed_title = DISALLOWED_TITLE.contains(&client_name);
        let is_disallowed_class = DISALLOWED_CLASS.contains(&class_name);

        if is_disallowed_title || is_disallowed_class {
            return;
        }

        self.sanitize_monitors().unwrap();

        match event {
            EVENT_SYSTEM_FOREGROUND => {
                let is_new_clinet = !self.monitors.iter().any(|monitor| -> bool {monitor.clients.iter().any(|client| -> bool {client.hwnd == hwnd})});
                if is_new_clinet {
                    if !Self::is_manageable(&hwnd).unwrap() {
                        return;
                    }
                    let client = self.manage(&hwnd).unwrap();
                    self.monitors[client.monitor].arrangemon().unwrap();
                }
                self.set_focus(hwnd);
                self.refresh_bar().unwrap();
            }
            EVENT_OBJECT_UNCLOAKED | EVENT_OBJECT_SHOW => {
                let is_new_clinet = !self.monitors.iter().any(|monitor| -> bool {monitor.clients.iter().any(|client| -> bool {client.hwnd == hwnd})});
                if is_new_clinet {
                    if !Self::is_manageable(&hwnd).unwrap() {
                        return;
                    }
                    let client = self.manage(&hwnd).unwrap();
                    self.monitors[client.monitor].arrangemon().unwrap();
                }
                self.set_focus(hwnd);
                self.refresh_bar().unwrap();
            }
            EVENT_OBJECT_CLOAKED | EVENT_OBJECT_DESTROY => {
                self.unmanage(&hwnd).unwrap();
            }
            EVENT_OBJECT_HIDE => {
                if self.monitors.iter().any(|monitor| -> bool {monitor.clients.iter().any(|client| -> bool {client.hwnd == hwnd && client.is_hide})}) {
                    return;
                }
                self.unmanage(&hwnd).unwrap();
            }
            EVENT_SYSTEM_MOVESIZEEND => {
                let is_new_clinet = !self.monitors.iter().any(|monitor| -> bool {monitor.clients.iter().any(|client| -> bool {client.hwnd == hwnd})});
                if is_new_clinet {
                    if !Self::is_manageable(&hwnd).unwrap() {
                        return;
                    }
                    let client = self.manage(&hwnd).unwrap();
                    self.monitors[client.monitor].arrangemon().unwrap();
                }
                self.reallocate_window(&hwnd).unwrap();
            }
            _ => ()
        }
    }

    unsafe fn sanitize_monitors(&mut self) -> Result<()>
    {
        for monitor in self.monitors.iter_mut() {
            monitor.sanitize_clients()?;
        }
        Ok(())
    }

    unsafe fn reallocate_window(&mut self, hwnd: &HWND) -> Result<()>
    {
        let mut original_window_rect = RECT::default();
        let mut mouse_point: POINT = POINT::default();
        GetCursorPos(&mut mouse_point)?;
        GetWindowRect(hwnd.clone(), &mut original_window_rect)?;
        let original_rect = Rect::from_win_rect(&original_window_rect);
        
        let mut contained_monitor_index: Option<usize> = None;
        let mut found_monitor_index: Option<usize> = None;
        let mut found_client_index: Option<usize> = None;
        for (monitor_index, monitor) in self.monitors.iter().enumerate() {
            let monitor_rect = &monitor.rect;
            let left_check = monitor_rect.x <= mouse_point.x;
            let right_check = mouse_point.x <= monitor_rect.x + monitor_rect.width;
            let top_check = monitor_rect.y <= mouse_point.y;
            let bottom_check = mouse_point.y <= monitor_rect.y + monitor_rect.height;

            if left_check && right_check && top_check && bottom_check {
                contained_monitor_index = Some(monitor_index);
            }

            let client_index = monitor.find_client_index(hwnd);
            if client_index.is_none() {
                continue;
            }

            let client_index = client_index.unwrap();

            if monitor.clients[client_index].rect == original_rect {
                return Ok(());
            }

            found_monitor_index = Some(monitor_index);
            found_client_index = Some(client_index);

            if  contained_monitor_index.is_some() && 
                found_monitor_index.is_some() &&
                found_client_index.is_some() {
                break;
            }
                    
        }

        if  found_monitor_index.is_none() || 
            found_client_index.is_none()  ||
            contained_monitor_index.is_none() {
            return Ok(());
        }

        let contained_monitor_index = contained_monitor_index.unwrap();
        let found_monitor_index = found_monitor_index.unwrap();
        let found_client_index = found_client_index.unwrap();

        let found_monitor = &self.monitors[found_monitor_index];
        let previous_master_threshold = (found_monitor.clients.len() as i32) - (found_monitor.master_count as i32);
        let previous_is_in_master = (found_client_index as i32) >= previous_master_threshold ;
        let is_in_master = found_monitor.layout.unwrap().is_in_master_area(found_monitor, mouse_point.x, mouse_point.y);
        let is_same_monitor = contained_monitor_index == found_monitor_index;
        let is_in_same_area = previous_is_in_master == is_in_master;

        if is_same_monitor && is_in_same_area {
            self.monitors[found_monitor_index].arrangemon()?;
            self.set_focus(*hwnd);
            return Ok(());
        }

        let current_monitor = &self.monitors[found_monitor_index];
        let current_monitor_visible_tags = current_monitor.tagset[current_monitor.selected_tag_index];
        let mut next_focus_index = (found_client_index + 1) % current_monitor.clients.len();
        while !Monitor::is_visible(&self.monitors[found_monitor_index].clients[next_focus_index], current_monitor_visible_tags)
        {
            next_focus_index += 1;
            next_focus_index %= current_monitor.clients.len();
        }

        if found_client_index == next_focus_index {
            self.monitors[found_monitor_index].selected_hwnd = HWND(0);
        } else {
            self.monitors[found_monitor_index].selected_hwnd = current_monitor.clients[next_focus_index].hwnd;
        }

        let mut client = self.monitors[found_monitor_index].clients[found_client_index].clone();
        self.monitors[found_monitor_index].clients.remove(found_client_index);


        let clients_count = self.monitors[contained_monitor_index].clients.len();
        let master_count = self.monitors[contained_monitor_index].master_count as usize;
        client.monitor = contained_monitor_index;
        if !is_in_master && (master_count <= clients_count) {
            self.monitors[contained_monitor_index].clients.insert(clients_count - master_count, client);
        } else {
            self.monitors[contained_monitor_index].clients.push(client);
        }

        self.arrange()?;
        self.set_focus(*hwnd);

        for monitor in self.monitors.iter_mut() {
            let _result = RedrawWindow(monitor.bar.hwnd, None, None, RDW_INVALIDATE);
        }
        Ok(())
    }

    fn set_focus(&mut self, hwnd: HWND)
    {
        for monitor in self.monitors.iter_mut() {
            monitor.bar.is_selected_monitor = false;
        }

        if let Some(selected_monitor_index) = self.selected_monitor_index {
            if hwnd == self.monitors[selected_monitor_index].selected_hwnd {
                self.monitors[selected_monitor_index].bar.is_selected_monitor = true;
                return;
            }
        }

        for monitor in self.monitors.iter_mut() {
            if monitor.find_client_index(&hwnd).is_none() {
                continue;
            }

            self.selected_monitor_index = Some(monitor.index);
            monitor.bar.is_selected_monitor = true;
            monitor.selected_hwnd = hwnd;
            return;
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

        let mut key_index = 0;
        for key in KEYS.iter() {
            RegisterHotKey(self.hwnd, key_index, key.mod_key, key.key as u32)?;
            key_index += 1;
        }

        for tag_keys in TAG_KEYS.iter() {
            for key in tag_keys.iter() {
                RegisterHotKey(self.hwnd, key_index, key.mod_key, key.key as u32)?;
                key_index += 1;
            }
        }
        Ok(())
    }

    unsafe extern "system" fn update_geom(hmonitor: HMONITOR, _: HDC, _: *mut RECT, lparam: LPARAM) -> BOOL {
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

        let mut monitor = Monitor{
            name: monitor_info.szDevice,
            index: (*this).monitors.len(),
            rect: Rect::from_win_rect(&monitor_info.monitorInfo.rcMonitor),
            client_area: Rect::from_win_rect(&monitor_info.monitorInfo.rcWork),
            master_count: 1,
            master_factor: 0.5,
            tagset: [1, 1],
            ..Default::default()
        };

        monitor.client_area.y += BAR_HEIGHT as i32;
        monitor.client_area.height -= BAR_HEIGHT as i32;
        monitor.bar.selected_tags = 1;

        let display_rect = monitor.rect.clone();
        (*this).monitors.push(monitor);
        (*this).monitors.last_mut().as_mut().unwrap().bar.setup_bar(&display_rect).unwrap();
        TRUE
    }

    unsafe fn sendmon(&mut self, client: Client, target_monitor_index: usize) -> Result<()> {
        if client.monitor == target_monitor_index {
            return Ok(());
        }

        let found_index: Option<usize> = self.monitors[client.monitor].find_client_index(&client.hwnd);

        if found_index.is_none() {
            return Ok(());
        }

        let found_index = found_index.unwrap();

        let current_monitor = &self.monitors[client.monitor];
        let current_monitor_visible_tags = current_monitor.tagset[current_monitor.selected_tag_index];
        let mut next_focus_index = (found_index + 1) % current_monitor.clients.len();
        while !Monitor::is_visible(&self.monitors[client.monitor].clients[next_focus_index], current_monitor_visible_tags)
        {
            next_focus_index += 1;
            next_focus_index %= current_monitor.clients.len();
        }

        if found_index == next_focus_index {
            self.monitors[client.monitor].selected_hwnd = HWND(0);
        } else {
            self.monitors[client.monitor].selected_hwnd = current_monitor.clients[next_focus_index].hwnd;
        }

        self.monitors[client.monitor].clients.remove(found_index);

        let mut new_client = client;
        new_client.monitor = target_monitor_index;
        self.monitors[target_monitor_index].clients.push(new_client);

        self.arrange()?;
        self.refresh_focus()?;
        Ok(())
    }

    pub unsafe fn set_monitor_factor(&mut self, arg: &Option<Arg>) -> Result<()> {
        if arg.is_none() {
            return Ok(());
        }

        let factor_offset = arg.as_ref().unwrap().f;

        self.monitors[self.selected_monitor_index.unwrap()].master_factor += factor_offset;
        self.monitors[self.selected_monitor_index.unwrap()].arrangemon()?;
        Ok(())
    }

    pub unsafe fn tag_monitor(&mut self, arg: &Option<Arg>) -> Result<()> {
        if arg.is_none() {
            return Ok(());
        }

        if self.selected_monitor_index.is_none() {
            return Ok(());
        }

        let index_offset = arg.as_ref().unwrap().i;
        let new_index = (((self.selected_monitor_index.unwrap() as i32) + index_offset) % self.monitors.len() as i32) as usize;

        let selected_monitor = &self.monitors[self.selected_monitor_index.unwrap()];
        let selected_client_index = selected_monitor.get_selected_client_index();

        if selected_client_index.is_none() {
            return Ok(());
        }

        let selected_client = selected_monitor.clients[selected_client_index.unwrap()].clone();
        self.sendmon(selected_client, new_index)?;
        Ok(())
    }

    unsafe fn refresh_bar(&mut self) -> Result<()> {
        let selected_monitor_index = self.selected_monitor_index;
        for monitor in self.monitors.iter_mut() {
            monitor.bar.selected_tags = monitor.tagset[monitor.selected_tag_index];
            if selected_monitor_index.is_some() && monitor.index == selected_monitor_index.unwrap() {
                monitor.bar.is_selected_monitor = true;
            } else {
                monitor.bar.is_selected_monitor = false;
            }
            let _result = RedrawWindow(monitor.bar.hwnd, None, None, RDW_INVALIDATE);
        }
        Ok(())
    }

    unsafe fn refresh_current_focus(&mut self) -> Result<()> {
        let focus_hwnd = GetForegroundWindow();
        self.selected_monitor_index = Some(0);
        for (monitor_index, monitor) in self.monitors.iter_mut().enumerate() {
            if monitor.find_client_index(&focus_hwnd).is_none() {
                continue;
            }

            self.selected_monitor_index = Some(monitor_index);
            monitor.selected_hwnd = focus_hwnd;
            break;
        }
        self.refresh_bar()?;
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

        let selected_client_index = selected_client_index.unwrap();
        let selected_client = selected_monitor.clients[selected_client_index].clone();
        selected_monitor.clients.remove(selected_client_index);
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
        if IsWindow(*hwnd) == FALSE {
            return Ok(false);
        }

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
            if let Err(e) = GetLastError() {
                println!("Error: failed to get window title - {e}");
            }
        }
        let client_name = PCWSTR::from_raw(client_name_buf.as_ptr()).to_string().unwrap();
        if DISALLOWED_TITLE.contains(&client_name) {
            return Ok(false);
        }

        let mut class_name_buf = [0u16; 256];
        SetLastError(WIN32_ERROR(0));
        if GetClassNameW(*hwnd, class_name_buf.as_mut()) == 0 {
            if let Err(e) = GetLastError() {
                println!("Error: failed to get class name - {e}");
            }
            // class name should not be empty
            return Ok(false); 
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
        for monitor in self.monitors.iter() {
            if let Some(client_index) = monitor.find_client_index(hwnd) {
                return Ok(monitor.clients[client_index].clone());
            }
        }

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

        let mut client_name_buf = [0u16; 256];
        SetLastError(WIN32_ERROR(0));
        if GetWindowTextW(*hwnd, client_name_buf.as_mut()) == 0 {
            if let Err(e) = GetLastError() {
                println!("Error: failed to get window title - {e}");
            }
        }
        let title = PCWSTR::from_raw(client_name_buf.as_ptr()).to_string().unwrap();

        let mut class_name_buf = [0u16; 256];
        SetLastError(WIN32_ERROR(0));
        if GetClassNameW(*hwnd, class_name_buf.as_mut()) == 0 {
            if let Err(e) = GetLastError() {
                println!("Error: failed to get class name - {e}");
            }
        }
        let class = PCWSTR::from_raw(class_name_buf.as_ptr()).to_string().unwrap();

        let get_processname = || -> Result<String> {
            let mut process_id: u32 = 0;
            if GetWindowThreadProcessId(*hwnd, Some(&mut process_id as *mut _)) == 0 {
                if let Err(e) = GetLastError() {
                    println!("Error: Failed to get process id - {}", e);
                    return Err(e);
                }
                return Ok(String::default());
            }

            let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, FALSE, process_id);
            if let Err(e) = handle {
                println!("Error: Failed to open process handle - {}", e);
                return Err(e);
            }

            let handle = handle.unwrap();
            let mut file_name_buf = [0u16; 256];
            if GetProcessImageFileNameW(handle, &mut file_name_buf) == 0 {
                if let Err(e) = GetLastError() {
                    println!("Error: Failed to get file name - {}", e);
                }
            }
            let file_name = PCWSTR::from_raw(file_name_buf.as_ptr()).to_string().unwrap();
            CloseHandle(handle)?;
            return Ok(file_name);
        };

        let process_filename = get_processname().unwrap_or_default();

        let mut client = Client {
            hwnd: *hwnd,
            title,
            class,
            process_filename,
            parent,
            root,
            rect: rect.into(),
            bw: 0,
            is_minimized,
            is_cloaked,
            monitor: monitor_index,
            tags: 1,
            ..Default::default()
        };

        for rule in RULES.iter() {
            if rule.is_match(&client) {
                client.is_floating = rule.is_floating;
                client.tags = rule.tags;
                break;
            }
        }

        self.monitors[monitor_index].clients.push(client.clone());

        Ok(client)
    }

    unsafe fn unmanage(&mut self, hwnd: &HWND) -> Result<()> {
        for monitor in self.monitors.iter_mut() {
            let found_index = monitor.find_client_index(hwnd);

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

        let monitors = &self.monitors;
        for monitor in monitors.iter() {
            for client in monitor.clients.iter() {
                ShowWindow(client.hwnd, SW_RESTORE);
            }
        }

        if self.hwnd.0 == 0 {
            return Ok(());
        }

        let tag_keys_len = TAG_KEYS.len() * TAG_KEYS.first().unwrap().len();
        for key_index in 0..(KEYS.len() + tag_keys_len) {
            UnregisterHotKey(self.hwnd, key_index as i32)?;
        }

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

    pub unsafe fn view(&mut self, arg: &Option<Arg>) -> Result<()> {
        if arg.is_none() {
            return Ok(());
        }

        let selected_tag = arg.as_ref().unwrap().ui;

        let monitor_index = self.selected_monitor_index.unwrap();
        let monitor = &mut self.monitors[monitor_index];
        if (selected_tag & TAGMASK) == monitor.tagset[monitor.selected_tag_index] {
            return Ok(());
        }

        monitor.selected_tag_index ^= 1;
        if (selected_tag & TAGMASK) != 0 {
            monitor.tagset[monitor.selected_tag_index] = selected_tag & TAGMASK;
        }
        monitor.bar.selected_tags = monitor.tagset[monitor.selected_tag_index];
        let _result = RedrawWindow(monitor.bar.hwnd, None, None, RDW_INVALIDATE);
        self.refresh_focus()?;
        self.arrange()?;
        Ok(())
    }

    pub unsafe fn toggle_view(&mut self, arg: &Option<Arg>) -> Result<()> {
        if arg.is_none() {
            return Ok(());
        }

        let selected_tag = arg.as_ref().unwrap().ui;

        let monitor_index = self.selected_monitor_index.unwrap();
        let monitor = &mut self.monitors[monitor_index];
        let new_tag_set = (selected_tag & TAGMASK) ^ monitor.tagset[monitor.selected_tag_index];

        if new_tag_set == 0 {
            return Ok(());
        }
        monitor.tagset[monitor.selected_tag_index] = new_tag_set;
        monitor.bar.selected_tags = new_tag_set;
        let _result = RedrawWindow(monitor.bar.hwnd, None, None, RDW_INVALIDATE);
        self.refresh_focus()?;
        self.arrange()?;
        Ok(())
    }

    pub unsafe fn tag(&mut self, arg: &Option<Arg>) -> Result<()> {
        if arg.is_none() {
            return Ok(());
        }

        let selected_tag = arg.as_ref().unwrap().ui & TAGMASK;
        if selected_tag == 0 {
            return Ok(());
        }

        let monitor_index = self.selected_monitor_index.unwrap();
        let monitor = &mut self.monitors[monitor_index];
        let selected_client_index = monitor.get_selected_client_index();
        if selected_client_index.is_none() {
            return Ok(());
        }

        monitor.clients[selected_client_index.unwrap()].tags = selected_tag;
        self.refresh_focus()?;
        self.arrange()?;
        Ok(())
    }

    pub unsafe fn toggle_tag(&mut self, arg: &Option<Arg>) -> Result<()> {
        if arg.is_none() {
            return Ok(());
        }

        let monitor_index = self.selected_monitor_index.unwrap();
        let monitor = &mut self.monitors[monitor_index];
        let selected_client_index = monitor.get_selected_client_index();
        if selected_client_index.is_none() {
            return Ok(());
        }

        let selected_tag = arg.as_ref().unwrap().ui & TAGMASK;
        let new_tags = monitor.clients[selected_client_index.unwrap()].tags ^ selected_tag;
        if new_tags == 0 {
            return Ok(());
        }

        monitor.clients[selected_client_index.unwrap()].tags = new_tags;
        self.refresh_focus()?;
        self.arrange()?;
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
        let clients_count = selected_monitor.visible_clinets_count();
        if clients_count == 0 {
            return Ok(());
        }

        let mut new_focus_index = selected_client_index as i32;
        let mut left_offset = -offset;
        let clients_count = selected_monitor.clients.len() as i32;
        let step = if offset < 0 { 1 } else { -1 } ;
        let selected_tag = selected_monitor.tagset[selected_monitor.selected_tag_index];
        while left_offset != 0 {
            left_offset -= step;
            new_focus_index += step;
            new_focus_index %= clients_count;
            new_focus_index += clients_count * (new_focus_index < 0) as i32;

            while !Monitor::is_visible(&selected_monitor.clients[new_focus_index as usize], selected_tag) {
                new_focus_index += step;
                new_focus_index %= clients_count as i32;
                new_focus_index += clients_count * (new_focus_index < 0) as i32;
            }
        }

        if new_focus_index == (selected_client_index as i32) {
            return Ok(());
        }

        let new_focus_hwnd = selected_monitor.clients[new_focus_index as usize].hwnd;
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

        let mut selected_client_option = selected_monitor.get_selected_client_index();
        if selected_client_option.is_none() && selected_monitor.clients.len() > 0 {
            selected_client_option = Some(selected_monitor.clients.len() - 1);
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
        self.refresh_bar()?;
        Ok(())
    }

    pub unsafe fn set_layout(&mut self, arg: &Option<Arg>) -> Result<()> {
        if arg.is_none() {
            return Ok(());
        }

        let monitor = &mut self.monitors[self.selected_monitor_index.unwrap()];
        monitor.layout = arg.as_ref().unwrap().l;
        monitor.arrangemon()?;
        self.refresh_focus()?;
        Ok(())
    }

    pub unsafe fn toggle_float(&mut self, _arg: &Option<Arg>) -> Result<()> {
        let selected_monitor = &self.monitors[self.selected_monitor_index.unwrap()];
        let selected_index = selected_monitor.get_selected_client_index();
        if selected_index.is_none() {
            return Ok(());
        }

        let selected_client = &mut self.monitors[self.selected_monitor_index.unwrap()].clients[selected_index.unwrap()];
        selected_client.is_floating = !selected_client.is_floating;
        self.arrange()?;
        self.refresh_focus()?;
        Ok(())
    }
}

