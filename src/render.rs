use chrono::Local;
use std::{ffi::OsStr, os::windows::ffi::OsStrExt};
use windows::{
    Win32::{
        Foundation::{COLORREF, RECT},
        Graphics::Gdi::{
            CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS, CreateFontW, CreateSolidBrush, DEFAULT_CHARSET,
            DEFAULT_PITCH, DT_CENTER, DT_SINGLELINE, DT_VCENTER, DrawTextW, FW_BOLD, FW_MEDIUM,
            FW_NORMAL, FillRect, FrameRect, GRADIENT_FILL_RECT_V, GRADIENT_RECT, GradientFill, HDC,
            OUT_DEFAULT_PRECIS, SelectObject, SetBkMode, SetTextColor, TRANSPARENT, TRIVERTEX,
        },
    },
    core::PCWSTR,
};

use crate::{
    config::WARNING_MESSAGE,
    state::{AppState, warning_active},
};

const PRIMARY_FONT: &str = "Segoe UI Variable Display";
const MONO_FONT: &str = "JetBrains Mono";
const MIN_PANEL_WIDTH: i32 = 320;
const MIN_PANEL_HEIGHT: i32 = 280;

pub unsafe fn draw_overlay(hdc: HDC, state: &AppState) {
    let rect = responsive_panel_rect(state);

    let warning = warning_active(state);
    draw_panel_background(hdc, &rect, warning);

    SetBkMode(hdc, TRANSPARENT);

    if warning {
        draw_warning_content(hdc, rect, state);
    } else {
        draw_normal_content(hdc, rect, state);
    }
}

fn responsive_panel_rect(state: &AppState) -> RECT {
    let horizontal_margin = ((state.width as f32) * 0.08).clamp(40.0, 180.0).round() as i32;
    let vertical_margin = ((state.height as f32) * 0.1).clamp(50.0, 200.0).round() as i32;
    let available_width = (state.width - 2 * horizontal_margin).max(MIN_PANEL_WIDTH);
    let available_height = (state.height - 2 * vertical_margin).max(MIN_PANEL_HEIGHT);
    let preferred_width = ((state.width as f32) * 0.45).round() as i32;
    let preferred_height = ((state.height as f32) * 0.42).round() as i32;
    let width = choose_dimension(preferred_width, available_width, MIN_PANEL_WIDTH);
    let height = choose_dimension(preferred_height, available_height, MIN_PANEL_HEIGHT);
    let left = ((state.width - width) / 2)
        .max(horizontal_margin)
        .min(state.width - horizontal_margin - width);
    let top = ((state.height - height) / 2)
        .max(vertical_margin)
        .min(state.height - vertical_margin - height);
    RECT {
        left,
        top,
        right: left + width,
        bottom: top + height,
    }
}

fn choose_dimension(preferred: i32, available: i32, min_size: i32) -> i32 {
    let limited = preferred.min(available);
    if available > min_size {
        limited.max(min_size)
    } else {
        available.max(0)
    }
}

fn overlay_scale(rect: &RECT) -> f32 {
    let base = 320.0;
    ((rect.bottom - rect.top) as f32 / base).clamp(0.85, 1.35)
}

fn scaled(value: i32, scale: f32) -> i32 {
    ((value as f32) * scale).round() as i32
}

unsafe fn draw_normal_content(hdc: HDC, rect: RECT, state: &AppState) {
    let scale = overlay_scale(&rect);
    let spacing = scaled(24, scale);
    let now = Local::now();
    let time_text = now.format("%H:%M:%S").to_string();
    let date_text = now.format("%A, %B %d %Y").to_string();
    let tagline = "Windows input is locked; type the password and press Enter.";
    let hint_text = "Use Backspace to correct mistakes. Ctrl+Alt+Delete is suppressed.";

    let mut tag_rect = rect;
    tag_rect.left += spacing;
    tag_rect.top += spacing / 2;
    tag_rect.right = tag_rect.left + scaled(140, scale);
    tag_rect.bottom = tag_rect.top + scaled(34, scale);
    draw_tag(hdc, &tag_rect, "LOCKED", COLORREF(0x00E06C75));

    let mut time_rect = rect;
    time_rect.left += spacing;
    time_rect.right -= spacing;
    time_rect.top = tag_rect.bottom + spacing;
    time_rect.bottom = time_rect.top + scaled(110, scale);
    draw_text_with_font(
        hdc,
        &time_rect,
        &time_text,
        scaled(96, scale),
        FW_BOLD.0 as i32,
        COLORREF(0x00F5F5F5),
        PRIMARY_FONT,
    );

    let mut date_rect = time_rect;
    date_rect.top = time_rect.bottom - scaled(10, scale);
    date_rect.bottom = date_rect.top + scaled(42, scale);
    draw_text_with_font(
        hdc,
        &date_rect,
        &date_text,
        scaled(28, scale),
        FW_NORMAL.0 as i32,
        COLORREF(0x00B4C7F5),
        PRIMARY_FONT,
    );

    draw_divider(hdc, rect, date_rect.bottom + spacing / 2);

    let mut password_rect = rect;
    password_rect.left += spacing;
    password_rect.right -= spacing;
    password_rect.top = date_rect.bottom + spacing;
    password_rect.bottom = password_rect.top + scaled(60, scale);
    draw_password_field(hdc, &password_rect, state, scaled(28, scale));

    let mut tag_line_rect = password_rect;
    tag_line_rect.top = password_rect.bottom + spacing / 2;
    tag_line_rect.bottom = tag_line_rect.top + scaled(28, scale);
    draw_text_with_font(
        hdc,
        &tag_line_rect,
        tagline,
        scaled(20, scale),
        FW_NORMAL.0 as i32,
        COLORREF(0x0095A5C1),
        PRIMARY_FONT,
    );

    let mut hint_rect = tag_line_rect;
    hint_rect.top = tag_line_rect.bottom + spacing / 2;
    hint_rect.bottom = hint_rect.top + scaled(24, scale);
    draw_text_with_font(
        hdc,
        &hint_rect,
        hint_text,
        scaled(18, scale),
        FW_MEDIUM.0 as i32,
        COLORREF(0x00C7D2EE),
        PRIMARY_FONT,
    );
}

unsafe fn draw_warning_content(hdc: HDC, rect: RECT, state: &AppState) {
    let scale = overlay_scale(&rect);
    let spacing = scaled(22, scale);
    let now = Local::now();
    let top_message = format!("{}!", WARNING_MESSAGE);
    let time_text = now.format("%H:%M:%S").to_string();
    let hint_text = "Hands off the keyboard and mouse until the warning clears.";

    let mut alert_rect = rect;
    alert_rect.left += spacing;
    alert_rect.right -= spacing;
    alert_rect.top += spacing;
    alert_rect.bottom = alert_rect.top + scaled(90, scale);
    draw_text_with_font(
        hdc,
        &alert_rect,
        &top_message,
        scaled(54, scale),
        FW_BOLD.0 as i32,
        COLORREF(0x00FF8585),
        PRIMARY_FONT,
    );

    let mut time_rect = alert_rect;
    time_rect.top = alert_rect.bottom;
    time_rect.bottom = time_rect.top + scaled(60, scale);
    draw_text_with_font(
        hdc,
        &time_rect,
        &time_text,
        scaled(40, scale),
        FW_MEDIUM.0 as i32,
        COLORREF(0x00FFD6A5),
        PRIMARY_FONT,
    );

    let mut password_rect = time_rect;
    password_rect.left += spacing;
    password_rect.right -= spacing;
    password_rect.top = time_rect.bottom + spacing;
    password_rect.bottom = password_rect.top + scaled(60, scale);
    draw_password_field(hdc, &password_rect, state, scaled(28, scale));

    let mut hint_rect = password_rect;
    hint_rect.left = password_rect.left;
    hint_rect.right = password_rect.right;
    hint_rect.top = password_rect.bottom + spacing / 2;
    hint_rect.bottom = hint_rect.top + scaled(30, scale);
    draw_text_with_font(
        hdc,
        &hint_rect,
        hint_text,
        scaled(22, scale),
        FW_NORMAL.0 as i32,
        COLORREF(0x00F0C674),
        PRIMARY_FONT,
    );
}

unsafe fn draw_password_field(hdc: HDC, rect: &RECT, state: &AppState, font_size: i32) {
    let masked = if state.input.is_empty() {
        "…".to_string()
    } else {
        "*".repeat(state.input.chars().count())
    };
    let password_text = format!("Password: {masked}");
    draw_text_with_font(
        hdc,
        rect,
        &password_text,
        font_size,
        FW_NORMAL.0 as i32,
        COLORREF(0x00FFFFFF),
        MONO_FONT,
    );
}

unsafe fn draw_tag(hdc: HDC, rect: &RECT, text: &str, color: COLORREF) {
    let brush = CreateSolidBrush(color);
    FillRect(hdc, rect, brush);
    let _ = windows::Win32::Graphics::Gdi::DeleteObject(brush);

    draw_text_with_font(
        hdc,
        rect,
        text,
        18,
        FW_BOLD.0 as i32,
        COLORREF(0x00000000),
        PRIMARY_FONT,
    );
}

unsafe fn draw_divider(hdc: HDC, rect: RECT, y: i32) {
    let mut line_rect = rect;
    line_rect.top = y;
    line_rect.bottom = y + 2;
    let brush = CreateSolidBrush(COLORREF(0x00454545));
    FillRect(hdc, &line_rect, brush);
    let _ = windows::Win32::Graphics::Gdi::DeleteObject(brush);
}

unsafe fn draw_panel_background(hdc: HDC, rect: &RECT, warning: bool) {
    let (top, bottom, accent) = if warning {
        (
            COLORREF(0x00300030),
            COLORREF(0x00100010),
            COLORREF(0x007F1D1D),
        )
    } else {
        (
            COLORREF(0x0020294A),
            COLORREF(0x000B1323),
            COLORREF(0x004357B7),
        )
    };

    fill_gradient(hdc, rect, top, bottom);

    let mut accent_rect = *rect;
    accent_rect.bottom = accent_rect.top + 5;
    let accent_brush = CreateSolidBrush(accent);
    FillRect(hdc, &accent_rect, accent_brush);
    let _ = windows::Win32::Graphics::Gdi::DeleteObject(accent_brush);

    let border_brush = CreateSolidBrush(COLORREF(0x00353C4A));
    FrameRect(hdc, rect, border_brush);
    let _ = windows::Win32::Graphics::Gdi::DeleteObject(border_brush);
}

unsafe fn fill_gradient(hdc: HDC, rect: &RECT, start: COLORREF, end: COLORREF) {
    let vertices = [
        color_vertex(rect.left, rect.top, start),
        color_vertex(rect.right, rect.bottom, end),
    ];
    let mesh = GRADIENT_RECT {
        UpperLeft: 0,
        LowerRight: 1,
    };
    let _ = GradientFill(
        hdc,
        &vertices,
        &mesh as *const _ as *const _,
        1,
        GRADIENT_FILL_RECT_V,
    );
}

fn color_vertex(x: i32, y: i32, color: COLORREF) -> TRIVERTEX {
    fn component(value: u32) -> u16 {
        ((value & 0xFF) as u16) << 8
    }

    TRIVERTEX {
        x,
        y,
        Red: component(color.0),
        Green: component(color.0 >> 8),
        Blue: component(color.0 >> 16),
        Alpha: 0,
    }
}

unsafe fn draw_text_with_font(
    hdc: HDC,
    rect: &RECT,
    text: &str,
    height: i32,
    weight: i32,
    color: COLORREF,
    font_face: &str,
) {
    let mut rect_copy = *rect;
    let mut text_wide = to_wstring(text);
    let face_wide = to_wstring(font_face);
    let font = CreateFontW(
        height,
        0,
        0,
        0,
        weight,
        0,
        0,
        0,
        DEFAULT_CHARSET.0 as u32,
        OUT_DEFAULT_PRECIS.0 as u32,
        CLIP_DEFAULT_PRECIS.0 as u32,
        CLEARTYPE_QUALITY.0 as u32,
        DEFAULT_PITCH.0 as u32,
        PCWSTR(face_wide.as_ptr()),
    );
    let old_font = SelectObject(hdc, font);
    SetTextColor(hdc, color);
    DrawTextW(
        hdc,
        &mut text_wide,
        &mut rect_copy,
        DT_CENTER | DT_VCENTER | DT_SINGLELINE,
    );
    let _ = SelectObject(hdc, old_font);
    let _ = windows::Win32::Graphics::Gdi::DeleteObject(font);
}

fn to_wstring(value: &str) -> Vec<u16> {
    let mut wide: Vec<u16> = OsStr::new(value).encode_wide().collect();
    wide.push(0);
    wide
}
