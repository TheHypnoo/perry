//! Screenshot capture for Windows (behind geisterhand feature).
//!
//! Uses PrintWindow + GDI to capture window, with inline minimal PNG encoder.

#[cfg(target_os = "windows")]
use windows::Win32::Foundation::*;
#[cfg(target_os = "windows")]
use windows::Win32::Graphics::Gdi::*;
#[cfg(target_os = "windows")]
use windows::Win32::Storage::Xps::{PrintWindow, PRINT_WINDOW_FLAGS};
#[cfg(target_os = "windows")]
use windows::Win32::UI::WindowsAndMessaging::GetWindowRect;

/// Capture the main application window as PNG bytes.
/// Returns a malloc'd buffer (caller frees with libc::free). Sets *out_len to byte count.
/// Returns null on failure.
#[no_mangle]
pub extern "C" fn perry_ui_screenshot_capture(out_len: *mut usize) -> *mut u8 {
    unsafe {
        *out_len = 0;
    }

    #[cfg(not(target_os = "windows"))]
    {
        return std::ptr::null_mut();
    }

    #[cfg(target_os = "windows")]
    {
        let hwnd = match crate::app::get_main_hwnd() {
            Some(h) => h,
            None => return std::ptr::null_mut(),
        };

        unsafe {
            // Get window dimensions
            let mut rect = RECT::default();
            if GetWindowRect(hwnd, &mut rect).is_err() {
                return std::ptr::null_mut();
            }
            let width = rect.right - rect.left;
            let height = rect.bottom - rect.top;
            if width <= 0 || height <= 0 {
                return std::ptr::null_mut();
            }

            // Create a memory DC and compatible bitmap
            let hdc_window = GetDC(hwnd);
            if hdc_window.is_invalid() {
                return std::ptr::null_mut();
            }
            let hdc_mem = CreateCompatibleDC(hdc_window);
            if hdc_mem.is_invalid() {
                ReleaseDC(hwnd, hdc_window);
                return std::ptr::null_mut();
            }
            let hbm = CreateCompatibleBitmap(hdc_window, width, height);
            if hbm.is_invalid() {
                DeleteDC(hdc_mem);
                ReleaseDC(hwnd, hdc_window);
                return std::ptr::null_mut();
            }
            let old_bm = SelectObject(hdc_mem, hbm);

            // Capture the window using PrintWindow (PW_RENDERFULLCONTENT = 2)
            let _ = PrintWindow(hwnd, hdc_mem, PRINT_WINDOW_FLAGS(2));

            // Set up BITMAPINFOHEADER for 32-bit BGRA
            let mut bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: width,
                    biHeight: -height, // negative = top-down
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };

            let row_bytes = (width as usize) * 4;
            let pixel_data_len = row_bytes * (height as usize);
            let mut pixels = vec![0u8; pixel_data_len];

            let lines = GetDIBits(
                hdc_mem,
                hbm,
                0,
                height as u32,
                Some(pixels.as_mut_ptr() as *mut _),
                &mut bmi,
                DIB_RGB_COLORS,
            );

            // Cleanup GDI
            SelectObject(hdc_mem, old_bm);
            let _ = DeleteObject(hbm);
            DeleteDC(hdc_mem);
            ReleaseDC(hwnd, hdc_window);

            if lines == 0 {
                return std::ptr::null_mut();
            }

            // Convert BGRA -> RGBA
            for i in (0..pixel_data_len).step_by(4) {
                pixels.swap(i, i + 2); // B <-> R
            }

            // Encode as PNG using minimal inline encoder
            let png = encode_png_rgba(width as u32, height as u32, &pixels);

            let len = png.len();
            let buf = libc::malloc(len) as *mut u8;
            if buf.is_null() {
                return std::ptr::null_mut();
            }
            std::ptr::copy_nonoverlapping(png.as_ptr(), buf, len);
            *out_len = len;
            buf
        }
    }
}

// ---------------------------------------------------------------------------
// Minimal PNG encoder — uses zlib stored blocks (no compression)
// ---------------------------------------------------------------------------

/// Encode RGBA pixel data as a valid PNG file.
#[cfg(target_os = "windows")]
fn encode_png_rgba(width: u32, height: u32, rgba: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();

    // PNG signature
    out.extend_from_slice(&[137, 80, 78, 71, 13, 10, 26, 10]);

    // IHDR chunk
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.push(8); // bit depth
    ihdr.push(6); // color type: RGBA
    ihdr.push(0); // compression method
    ihdr.push(0); // filter method
    ihdr.push(0); // interlace method
    write_chunk(&mut out, b"IHDR", &ihdr);

    // IDAT chunk — zlib wrapper around uncompressed deflate blocks.
    // Each scanline: filter_byte(0 = None) followed by width*4 RGBA bytes.
    let row_len = 1 + (width as usize) * 4;
    let total_raw = row_len * (height as usize);

    let mut raw = Vec::with_capacity(total_raw);
    for y in 0..(height as usize) {
        raw.push(0); // filter: None
        let start = y * (width as usize) * 4;
        let end = start + (width as usize) * 4;
        raw.extend_from_slice(&rgba[start..end]);
    }

    // Wrap in zlib format: CMF + FLG, then stored deflate blocks, then Adler-32
    let mut zlib = Vec::new();
    zlib.push(0x78); // CMF: deflate, window size 32K
    zlib.push(0x01); // FLG: check bits (0x7801 % 31 == 0)

    // Write as stored deflate blocks (max 65535 bytes each)
    let mut offset = 0;
    while offset < raw.len() {
        let remaining = raw.len() - offset;
        let block_size = remaining.min(65535);
        let is_last = offset + block_size >= raw.len();
        zlib.push(if is_last { 1 } else { 0 }); // BFINAL + BTYPE=00 (stored)
        let len16 = block_size as u16;
        zlib.extend_from_slice(&len16.to_le_bytes());
        zlib.extend_from_slice(&(!len16).to_le_bytes()); // NLEN (one's complement)
        zlib.extend_from_slice(&raw[offset..offset + block_size]);
        offset += block_size;
    }

    // Adler-32 checksum of raw data
    let adler = adler32(&raw);
    zlib.extend_from_slice(&adler.to_be_bytes());

    write_chunk(&mut out, b"IDAT", &zlib);

    // IEND chunk
    write_chunk(&mut out, b"IEND", &[]);

    out
}

/// Write a single PNG chunk: length (4 BE) + type (4) + data + CRC32 (4 BE).
#[cfg(target_os = "windows")]
fn write_chunk(out: &mut Vec<u8>, chunk_type: &[u8; 4], data: &[u8]) {
    let len = data.len() as u32;
    out.extend_from_slice(&len.to_be_bytes());
    out.extend_from_slice(chunk_type);
    out.extend_from_slice(data);
    // CRC32 covers type + data
    let mut crc_data = Vec::with_capacity(4 + data.len());
    crc_data.extend_from_slice(chunk_type);
    crc_data.extend_from_slice(data);
    let crc = crc32(&crc_data);
    out.extend_from_slice(&crc.to_be_bytes());
}

/// Standard CRC-32 (ISO 3309 / PNG spec).
#[cfg(target_os = "windows")]
fn crc32(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0xEDB8_8320;
            } else {
                crc >>= 1;
            }
        }
    }
    !crc
}

/// Adler-32 checksum (zlib trailer).
#[cfg(target_os = "windows")]
fn adler32(data: &[u8]) -> u32 {
    let mut a: u32 = 1;
    let mut b: u32 = 0;
    for &byte in data {
        a = (a + byte as u32) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}
