use std::io::{BufRead, Seek};

use klip_core::{Frame, PlatformError, Rect};
use png::{BitDepth, ColorType, Decoder, Transformations};

const OPERATION: &str = "decode the portal screenshot";

/// Decodes any 8-bit PNG into an opaque-or-not RGBA frame placed at the origin.
pub(crate) fn decode_png(source: impl BufRead + Seek) -> Result<Frame, PlatformError> {
    let fail = |error: png::DecodingError| PlatformError::new(OPERATION, error.to_string());
    let mut decoder = Decoder::new(source);
    decoder.set_transformations(Transformations::EXPAND | Transformations::STRIP_16);
    let mut reader = decoder.read_info().map_err(fail)?;
    let mut pixels = vec![
        0;
        reader.output_buffer_size().ok_or_else(|| {
            PlatformError::new(OPERATION, "the image is too large to decode")
        })?
    ];
    let info = reader.next_frame(&mut pixels).map_err(fail)?;
    pixels.truncate(info.buffer_size());

    if info.bit_depth != BitDepth::Eight {
        return Err(PlatformError::new(OPERATION, "unsupported bit depth"));
    }
    let rgba = match info.color_type {
        ColorType::Rgba => pixels,
        ColorType::Rgb => pixels
            .as_chunks::<3>()
            .0
            .iter()
            .flat_map(|pixel| [pixel[0], pixel[1], pixel[2], u8::MAX])
            .collect(),
        ColorType::Grayscale => pixels
            .iter()
            .flat_map(|&gray| [gray, gray, gray, u8::MAX])
            .collect(),
        ColorType::GrayscaleAlpha => pixels
            .as_chunks::<2>()
            .0
            .iter()
            .flat_map(|pixel| [pixel[0], pixel[0], pixel[0], pixel[1]])
            .collect(),
        ColorType::Indexed => {
            return Err(PlatformError::new(
                OPERATION,
                "indexed color was not expanded",
            ));
        }
    };
    let bounds = Rect {
        x: 0,
        y: 0,
        w: info.width,
        h: info.height,
    };
    Frame::new(bounds, rgba).map_err(|error| PlatformError::new(OPERATION, error.to_string()))
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use png::Encoder;

    use super::*;

    fn encode(color: ColorType, width: u32, height: u32, data: &[u8]) -> Vec<u8> {
        let mut bytes = Vec::new();
        let mut encoder = Encoder::new(&mut bytes, width, height);
        encoder.set_color(color);
        encoder.set_depth(BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(data).unwrap();
        writer.finish().unwrap();
        bytes
    }

    #[test]
    fn rgba_png_keeps_its_pixels() {
        let png = encode(ColorType::Rgba, 2, 1, &[1, 2, 3, 4, 5, 6, 7, 8]);

        let frame = decode_png(Cursor::new(png)).unwrap();

        assert_eq!(
            frame.bounds(),
            Rect {
                x: 0,
                y: 0,
                w: 2,
                h: 1
            }
        );
        assert_eq!(frame.rgba(), [1, 2, 3, 4, 5, 6, 7, 8]);
    }

    #[test]
    fn rgb_png_gains_an_opaque_alpha_channel() {
        let png = encode(ColorType::Rgb, 1, 2, &[1, 2, 3, 4, 5, 6]);

        let frame = decode_png(Cursor::new(png)).unwrap();

        assert_eq!(frame.rgba(), [1, 2, 3, 255, 4, 5, 6, 255]);
    }

    #[test]
    fn grayscale_png_is_expanded_to_rgba() {
        let png = encode(ColorType::Grayscale, 2, 1, &[9, 200]);

        let frame = decode_png(Cursor::new(png)).unwrap();

        assert_eq!(frame.rgba(), [9, 9, 9, 255, 200, 200, 200, 255]);
    }

    #[test]
    fn garbage_is_reported_instead_of_panicking() {
        assert!(decode_png(Cursor::new(vec![1, 2, 3])).is_err());
    }
}
