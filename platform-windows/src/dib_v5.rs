use std::slice;

use klip_core::{Frame, PlatformError};
use windows::Win32::Graphics::Gdi::{BI_BITFIELDS, BITMAPV5HEADER, LCS_GM_IMAGES};
use windows::Win32::UI::ColorSystem::LCS_sRGB;

const HEADER_LEN: usize = size_of::<BITMAPV5HEADER>();
const BYTES_PER_PIXEL: usize = 4;

pub(crate) fn byte_len(image: &Frame) -> Result<usize, PlatformError> {
    let bounds = image.bounds();
    if bounds.w == 0 || bounds.h == 0 {
        return Err(PlatformError::new(
            "build clipboard bitmap",
            "the image is empty",
        ));
    }
    Ok(HEADER_LEN + image.rgba().len())
}

/// Fills `dib`, which must be `byte_len(image)` bytes long, with the layout
/// CF_DIBV5 consumers expect: bottom-up rows of BGRA pixels described by channel masks.
pub(crate) fn write(image: &Frame, dib: &mut [u8]) {
    let bounds = image.bounds();
    let header = BITMAPV5HEADER {
        bV5Size: HEADER_LEN as u32,
        bV5Width: bounds.w as i32,
        bV5Height: bounds.h as i32,
        bV5Planes: 1,
        bV5BitCount: 32,
        bV5Compression: BI_BITFIELDS,
        bV5SizeImage: image.rgba().len() as u32,
        bV5RedMask: 0x00FF_0000,
        bV5GreenMask: 0x0000_FF00,
        bV5BlueMask: 0x0000_00FF,
        bV5AlphaMask: 0xFF00_0000,
        bV5CSType: LCS_sRGB.0 as u32,
        bV5Intent: LCS_GM_IMAGES as u32,
        ..Default::default()
    };
    let (header_bytes, pixels) = dib.split_at_mut(HEADER_LEN);
    // SAFETY: BITMAPV5HEADER is a C struct made only of integers with no padding,
    // so every one of its bytes is initialized.
    header_bytes.copy_from_slice(unsafe {
        slice::from_raw_parts((&raw const header).cast::<u8>(), HEADER_LEN)
    });

    let stride = bounds.w as usize * BYTES_PER_PIXEL;
    let rows_from_the_bottom = image.rgba().chunks_exact(stride).rev();
    for (target, source) in pixels.chunks_exact_mut(stride).zip(rows_from_the_bottom) {
        let bgra = target.as_chunks_mut::<BYTES_PER_PIXEL>().0;
        let rgba = source.as_chunks::<BYTES_PER_PIXEL>().0;
        for (bgra, rgba) in bgra.iter_mut().zip(rgba) {
            *bgra = [rgba[2], rgba[1], rgba[0], rgba[3]];
        }
    }
}

#[cfg(test)]
mod tests {
    use klip_core::Rect;

    use super::*;

    const TOP: [u8; 4] = [10, 20, 30, 255];
    const BOTTOM: [u8; 4] = [40, 50, 60, 128];

    fn one_pixel_wide_frame() -> Frame {
        let bounds = Rect {
            x: 0,
            y: 0,
            w: 1,
            h: 2,
        };
        Frame::new(bounds, [TOP, BOTTOM].concat()).unwrap()
    }

    fn u32_at(bytes: &[u8], offset: usize) -> u32 {
        u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
    }

    #[test]
    fn dib_is_a_v5_header_followed_by_bottom_up_bgra_rows() {
        let frame = one_pixel_wide_frame();
        let mut dib = vec![0; byte_len(&frame).unwrap()];

        write(&frame, &mut dib);

        assert_eq!(dib.len(), 124 + 8);
        assert_eq!(u32_at(&dib, 0), 124, "header size");
        assert_eq!(u32_at(&dib, 4), 1, "width");
        assert_eq!(u32_at(&dib, 8), 2, "positive height means bottom-up rows");
        assert_eq!(
            u32_at(&dib, 12),
            1 | (32 << 16),
            "one plane, 32 bits per pixel"
        );
        assert_eq!(u32_at(&dib, 16), 3, "BI_BITFIELDS");
        assert_eq!(u32_at(&dib, 20), 8, "image size");
        assert_eq!(
            [
                u32_at(&dib, 40),
                u32_at(&dib, 44),
                u32_at(&dib, 48),
                u32_at(&dib, 52)
            ],
            [0x00FF_0000, 0x0000_FF00, 0x0000_00FF, 0xFF00_0000],
            "red, green, blue and alpha masks"
        );
        assert_eq!(
            &dib[124..128],
            &[60, 50, 40, 128],
            "bottom row comes first, as BGRA"
        );
        assert_eq!(
            &dib[128..132],
            &[30, 20, 10, 255],
            "top row comes last, as BGRA"
        );
    }

    #[test]
    fn empty_frame_has_no_dib_representation() {
        let bounds = Rect {
            x: 0,
            y: 0,
            w: 0,
            h: 3,
        };
        let frame = Frame::new(bounds, Vec::new()).unwrap();

        assert!(byte_len(&frame).is_err());
    }
}
