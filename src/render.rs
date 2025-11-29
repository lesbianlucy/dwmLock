use chrono::Local;
use std::{ffi::OsStr, os::windows::ffi::OsStrExt};
use windows::{
    core::PCWSTR,
    Win32::{
        Foundation::{COLORREF, RECT},
        Graphics::Gdi::{
            CreateFontW, CreateSolidBrush, DrawTextW, FillRect, FrameRect, GradientFill, SelectObject, SetBkMode,
            SetTextColor, CLIP_DEFAULT_PRECIS, CLEARTYPE_QUALITY, DEFAULT_CHARSET, DEFAULT_PITCH, DT_CENTER,
            DT_SINGLELINE, DT_VCENTER, FW_BOLD, FW_MEDIUM, FW_NORMAL, GRADIENT_FILL_RECT_V, GRADIENT_RECT, HDC,
            OUT_DEFAULT_PRECIS, TRANSPARENT, TRIVERTEX,
        },
    },
};

use crate::{
    config::WARNING_MESSAGE,
    state::{warning_active, AppState},
};

const PRIMARY_FONT: &str = "Segoe UI Variable Display";
const MONO_FONT: &str = "JetBrains Mono";

pub unsafe fn draw_overlay(hdc: HDC, state: &AppState) {
    let overlay_width = (state.width as f32 * 0.55) as i32;
    let overlay_height = 320;
    let left = (state.width - overlay_width) / 2;
    let top = (state.height - overlay_height) / 2;
    let rect = RECT {
        left,
        top,
        right: left + overlay_width,
        bottom: top + overlay_height,
    };

    let warning = warning_active(state);
    draw_panel_background(hdc, &rect, warning);

    SetBkMode(hdc, TRANSPARENT);

    if warning {
        draw_warning_content(hdc, rect, state);
    } else {
        draw_normal_content(hdc, rect, state);
    }
}

unsafe fn draw_normal_content(hdc: HDC, rect: RECT, state: &AppState) {
    let now = Local::now();
    let time_text = now.format("%H:%M:%S").to_string();
    let date_text = now.format("%A, %B %d %Y").to_string();
    let tagline = "Sleek blur lock inspired by unixporn setups";

    let mut tag_rect = rect;
    tag_rect.left += 30;
    tag_rect.top += 20;
    tag_rect.right = tag_rect.left + 120;
    tag_rect.bottom = tag_rect.top + 32;
    draw_tag(hdc, &tag_rect, "LOCKED", COLORREF(0x00E06C75));

    let mut time_rect = rect;
    time_rect.top = tag_rect.bottom + 10;
    time_rect.bottom = time_rect.top + 110;
    draw_text_with_font(
        hdc,
        &time_rect,
        &time_text,
        96,
        FW_BOLD.0 as i32,
        COLORREF(0x00F5F5F5),
        PRIMARY_FONT,
    );

    let mut date_rect = time_rect;
    date_rect.top = time_rect.bottom - 10;
    date_rect.bottom = date_rect.top + 40;
    draw_text_with_font(
        hdc,
        &date_rect,
        &date_text,
        28,
        FW_NORMAL.0 as i32,
        COLORREF(0x00B4C7F5),
        PRIMARY_FONT,
    );

    draw_divider(hdc, rect, date_rect.bottom + 10);

    let mut password_rect = rect;
    password_rect.top = date_rect.bottom + 30;
    password_rect.bottom = password_rect.top + 50;
    draw_password_field(hdc, &password_rect, state);

    let mut tag_line_rect = password_rect;
    tag_line_rect.top = password_rect.bottom + 20;
    tag_line_rect.bottom = tag_line_rect.top + 30;
    draw_text_with_font(
        hdc,
        &tag_line_rect,
        tagline,
        20,
        FW_NORMAL.0 as i32,
        COLORREF(0x0095A5C1),
        PRIMARY_FONT,
    );
}

unsafe fn draw_warning_content(hdc: HDC, rect: RECT, state: &AppState) {
    let now = Local::now();
    let top_message = format!("{}!", WARNING_MESSAGE);
    let time_text = now.format("%H:%M:%S").to_string();
    let hint_text = "Hands off the keyboard and mouse.";

    let mut alert_rect = rect;
    alert_rect.top += 25;
    alert_rect.bottom = alert_rect.top + 90;
    draw_text_with_font(
        hdc,
        &alert_rect,
        &top_message,
        54,
        FW_BOLD.0 as i32,
        COLORREF(0x00FF8585),
        PRIMARY_FONT,
    );

    let mut time_rect = alert_rect;
    time_rect.top = alert_rect.bottom;
    time_rect.bottom = time_rect.top + 60;
    draw_text_with_font(
        hdc,
        &time_rect,
        &time_text,
        40,
        FW_MEDIUM.0 as i32,
        COLORREF(0x00FFD6A5),
        PRIMARY_FONT,
    );

    let mut password_rect = time_rect;
    password_rect.top = time_rect.bottom + 15;
    password_rect.bottom = password_rect.top + 50;
    draw_password_field(hdc, &password_rect, state);

    let mut hint_rect = password_rect;
    hint_rect.top = password_rect.bottom + 20;
    hint_rect.bottom = hint_rect.top + 30;
    draw_text_with_font(
        hdc,
        &hint_rect,
        hint_text,
        22,
        FW_NORMAL.0 as i32,
        COLORREF(0x00F0C674),
        PRIMARY_FONT,
    );
}

unsafe fn draw_password_field(hdc: HDC, rect: &RECT, state: &AppState) {
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
        28,
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
        (COLORREF(0x00300030), COLORREF(0x00100010), COLORREF(0x007F1D1D))
    } else {
        (COLORREF(0x0020294A), COLORREF(0x000B1323), COLORREF(0x004357B7))
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
