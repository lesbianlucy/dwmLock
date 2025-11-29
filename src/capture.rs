use windows::{
    Win32::{
        Graphics::Gdi::{
            BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BitBlt, CreateCompatibleBitmap,
            CreateCompatibleDC, DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDC, GetDIBits, RGBQUAD,
            ReleaseDC, SRCCOPY, SelectObject,
        },
        UI::WindowsAndMessaging::{GetDesktopWindow, GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN},
    },
    core::Result,
};

pub struct ScreenImage {
    pub width: i32,
    pub height: i32,
    pub pixels: Vec<u8>,
}

pub unsafe fn capture_screen() -> Result<ScreenImage> {
    let width = GetSystemMetrics(SM_CXSCREEN);
    let height = GetSystemMetrics(SM_CYSCREEN);

    let desktop = GetDesktopWindow();
    let screen_dc = GetDC(desktop);
    if screen_dc.0 == 0 {
        return Err(windows::core::Error::from_win32());
    }

    let memory_dc = CreateCompatibleDC(screen_dc);
    if memory_dc.0 == 0 {
        let _ = ReleaseDC(desktop, screen_dc);
        return Err(windows::core::Error::from_win32());
    }

    let bitmap = CreateCompatibleBitmap(screen_dc, width, height);
    if bitmap.0 == 0 {
        let _ = DeleteDC(memory_dc);
        let _ = ReleaseDC(desktop, screen_dc);
        return Err(windows::core::Error::from_win32());
    }

    let previous = SelectObject(memory_dc, bitmap);
    let _ = BitBlt(memory_dc, 0, 0, width, height, screen_dc, 0, 0, SRCCOPY);

    let mut bitmap_info = build_bitmap_info(width, height);
    let mut pixels = vec![0u8; (width * height * 4) as usize];
    let result = GetDIBits(
        memory_dc,
        bitmap,
        0,
        height as u32,
        Some(pixels.as_mut_ptr() as *mut _),
        &mut bitmap_info,
        DIB_RGB_COLORS,
    );

    let _ = SelectObject(memory_dc, previous);
    let _ = DeleteObject(bitmap);
    let _ = DeleteDC(memory_dc);
    let _ = ReleaseDC(desktop, screen_dc);

    if result == 0 {
        return Err(windows::core::Error::from_win32());
    }

    Ok(ScreenImage {
        width,
        height,
        pixels,
    })
}

pub fn build_bitmap_info(width: i32, height: i32) -> BITMAPINFO {
    BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width,
            biHeight: -height,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        bmiColors: [RGBQUAD::default(); 1],
    }
}
