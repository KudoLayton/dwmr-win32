use windows::{
    core::*,
    Foundation::Numerics::*,
    Win32::Graphics::{
        Direct2D::*,
        DirectWrite::*,
    }
};

use crate::BAR_FONT;

const FHD_WIDTH: f32 = 1920.0;

pub unsafe fn implement_draw_text_box(
    text: &[u16],
    super_text: Option<&[u16]>,
    font_size: f32,
    max_width: f32,
    max_height: f32,
    origin_x: f32,
    origin_y: f32,
    pad: f32,
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
    text_layout.SetFontFamilyName(BAR_FONT, text_range.clone())?;
    text_layout.SetLocaleName(w!("ko_kr"), text_range.clone())?;
    text_layout.GetMetrics(&mut text_metrics)?;

    let rect = Common::D2D_RECT_F {
        left: 0.0, 
        top: max_height, 
        right: 0.0 + &text_metrics.width + (pad * 2.0), 
        bottom: 0.0
    };

    let affine_matrix =  Matrix3x2 {
        M11: FHD_WIDTH / max_width, M12: 0.0,
        M21: 0.0,                   M22: 1.0,
        M31: origin_x,              M32: 0.0
    };
    render_target_ref.SetTransform(&affine_matrix);

    render_target_ref.FillRectangle(&rect, box_brush);
    render_target_ref.DrawTextLayout(
        Common::D2D_POINT_2F{x: rect.left + pad, y: 0.0}, 
        &text_layout, 
        text_brush, 
        D2D1_DRAW_TEXT_OPTIONS_NONE);

    if let Some(super_text_data) = super_text {
        let super_text_range = DWRITE_TEXT_RANGE{startPosition: 0, length: super_text_data.len() as u32};
        let mut super_text_metrics = DWRITE_TEXT_METRICS::default();

        let super_text_layout = write_factory.CreateTextLayout(
            super_text_data, 
            text_format, 
            max_width / dpi, 
            max_height / dpi)?;
        super_text_layout.SetFontSize(font_size * 0.8, super_text_range.clone())?;
        super_text_layout.SetFontFamilyName(BAR_FONT, super_text_range.clone())?;
        super_text_layout.SetLocaleName(w!("ko-kr"), super_text_range.clone())?;
        super_text_layout.GetMetrics(&mut super_text_metrics)?;

        render_target_ref.DrawTextLayout(
            Common::D2D_POINT_2F{x: rect.right - super_text_metrics.width, y: 0.0}, 
            &super_text_layout, 
            text_brush, 
            D2D1_DRAW_TEXT_OPTIONS_NONE);
    }
    Ok(origin_x + (rect.right) * FHD_WIDTH / max_width)
}
