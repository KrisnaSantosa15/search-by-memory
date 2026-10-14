//! Thumbnails of files, as the system file manager shows them.

use std::path::Path;

/// A PNG thumbnail of `path` at most `size` pixels on its longer side, from
/// the system's thumbnail handlers; `None` when the system has no picture of
/// the file (only an icon), or the file cannot be read. With `cached_only`,
/// only a thumbnail the system already made is used, so the file itself is
/// never opened (an Online-only File is never downloaded).
pub fn thumbnail(path: &Path, size: u32, cached_only: bool) -> Option<Vec<u8>> {
    let (width, height, rgba) = shell::thumbnail(path, size, cached_only)?;
    encode_png(width, height, &rgba)
}

fn encode_png(width: u32, height: u32, rgba: &[u8]) -> Option<Vec<u8>> {
    let mut bytes = Vec::new();
    let mut encoder = png::Encoder::new(&mut bytes, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().ok()?;
    writer.write_image_data(rgba).ok()?;
    writer.finish().ok()?;
    Some(bytes)
}

#[cfg(windows)]
mod shell {
    use std::path::Path;

    use windows::Win32::Foundation::SIZE;
    use windows::Win32::Graphics::Gdi::{
        BI_RGB, BITMAP, BITMAPINFO, BITMAPINFOHEADER, CreateCompatibleDC, DIB_RGB_COLORS, DeleteDC,
        DeleteObject, GetDIBits, GetObjectW, HBITMAP, HGDIOBJ,
    };
    use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};
    use windows::Win32::UI::Shell::{
        IShellItemImageFactory, SHCreateItemFromParsingName, SIIGBF_INCACHEONLY,
        SIIGBF_THUMBNAILONLY,
    };
    use windows::core::HSTRING;

    /// Width, height and straight RGBA pixels, top row first.
    pub fn thumbnail(path: &Path, size: u32, cached_only: bool) -> Option<(u32, u32, Vec<u8>)> {
        // Already initialised on this thread is fine; the call only fails then.
        let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
        let factory: IShellItemImageFactory =
            unsafe { SHCreateItemFromParsingName(&HSTRING::from(path.as_os_str()), None) }.ok()?;
        let mut flags = SIIGBF_THUMBNAILONLY;
        if cached_only {
            flags |= SIIGBF_INCACHEONLY;
        }
        let side = size as i32;
        let bitmap = unsafe { factory.GetImage(SIZE { cx: side, cy: side }, flags) }.ok()?;
        let pixels = pixels_of(bitmap);
        unsafe {
            let _ = DeleteObject(HGDIOBJ(bitmap.0));
        }
        pixels
    }

    fn pixels_of(bitmap: HBITMAP) -> Option<(u32, u32, Vec<u8>)> {
        let mut info = BITMAP::default();
        let read = unsafe {
            GetObjectW(
                HGDIOBJ(bitmap.0),
                size_of::<BITMAP>() as i32,
                Some(&mut info as *mut BITMAP as *mut _),
            )
        };
        if read == 0 || info.bmWidth <= 0 || info.bmHeight <= 0 {
            return None;
        }
        let (width, height) = (info.bmWidth as u32, info.bmHeight as u32);
        let mut header = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width as i32,
                // Negative: rows top first.
                biHeight: -(height as i32),
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bgra = vec![0u8; (width * height * 4) as usize];
        let dc = unsafe { CreateCompatibleDC(None) };
        let lines = unsafe {
            GetDIBits(
                dc,
                bitmap,
                0,
                height,
                Some(bgra.as_mut_ptr() as *mut _),
                &mut header,
                DIB_RGB_COLORS,
            )
        };
        unsafe {
            let _ = DeleteDC(dc);
        }
        if lines == 0 {
            return None;
        }
        Some((width, height, straight_rgba(bgra)))
    }

    /// The Shell's premultiplied BGRA as straight RGBA. A bitmap without any
    /// alpha (all zero) is opaque.
    fn straight_rgba(mut pixels: Vec<u8>) -> Vec<u8> {
        let has_alpha = pixels.chunks_exact(4).any(|pixel| pixel[3] != 0);
        for pixel in pixels.chunks_exact_mut(4) {
            pixel.swap(0, 2);
            if !has_alpha {
                pixel[3] = 255;
            } else if pixel[3] != 0 && pixel[3] != 255 {
                let alpha = pixel[3] as u32;
                for channel in &mut pixel[..3] {
                    *channel = ((*channel as u32 * 255 + alpha / 2) / alpha).min(255) as u8;
                }
            }
        }
        pixels
    }
}

#[cfg(not(windows))]
mod shell {
    use std::path::Path;

    /// No system thumbnails here yet.
    pub fn thumbnail(_path: &Path, _size: u32, _cached_only: bool) -> Option<(u32, u32, Vec<u8>)> {
        None
    }
}
