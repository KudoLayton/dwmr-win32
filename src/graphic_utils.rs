use windows::{
    core::*,
    Foundation::Numerics::*,
    Win32::{
        UI::HiDpi::*,
        System::{
            Diagnostics::Debug::*, 
            Threading::*,
            LibraryLoader::*,
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
            Direct2D::{*, Common::{D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_COLOR_F}},
            Dxgi::Common::*,
            DirectWrite::*,
        }
    }
};

use super::Rect;

pub unsafe fn implement_draw_text_box(
    text: &[u16],
    font_size: f32,
    max_width: f32,
    max_height: f32,
    origin_x: f32,
    origin_y: f32,
    dpi: f32,
    text_format: &IDWriteTextFormat,
    write_factory: &IDWriteFactory,
    render_target_ref: &ID2D1HwndRenderTarget,
    box_brush: &ID2D1SolidColorBrush,
    text_brush: &ID2D1SolidColorBrush,
) -> Result<f32> {
    let text_range = DWRITE_TEXT_RANGE{startPosition: 0, length: text.len() as u32};
    let mut text_metrics = DWRITE_TEXT_METRICS::default();

    let text_layout = write_factory.CreateTextLayout(
        text, 
        text_format, 
        max_width / dpi, 
        max_height / dpi)?;
    text_layout.SetFontSize(font_size, text_range.clone())?;
    text_layout.SetFontFamilyName(w!("Arial"), text_range.clone())?;
    text_layout.SetLocaleName(w!("ko_kr"), text_range.clone())?;
    text_layout.GetMetrics(&mut text_metrics)?;

    let rect = Common::D2D_RECT_F {
        left: origin_x, 
        top: max_height, 
        right: origin_x + &text_metrics.width, 
        bottom: 0.0
    };
    render_target_ref.FillRectangle(&rect, box_brush);
    render_target_ref.DrawTextLayout(
        //Common::D2D_POINT_2F{x: rect.left, y: (max_height / 2.0) - (&text_metrics.width / 2.0)}, 
        Common::D2D_POINT_2F{x: rect.left, y: 0.0}, 
        &text_layout, 
        text_brush, 
        D2D1_DRAW_TEXT_OPTIONS_NONE);
    Ok(origin_x + text_metrics.width)
}