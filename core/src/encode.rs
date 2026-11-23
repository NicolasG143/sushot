use std::io::Write;

use png::{BitDepth, ColorType, Encoder, EncodingError};

use crate::Frame;

pub fn encode_png(frame: &Frame) -> Result<Vec<u8>, EncodingError> {
    let mut encoded = Vec::new();
    write_png(&mut encoded, frame)?;
    Ok(encoded)
}

/// Streams the PNG into `sink` as it is compressed, so no encoded copy of the image
/// is held in memory.
pub fn write_png(sink: impl Write, frame: &Frame) -> Result<(), EncodingError> {
    let bounds = frame.bounds();
    let mut encoder = Encoder::new(sink, bounds.w, bounds.h);
    encoder.set_color(ColorType::Rgba);
    encoder.set_depth(BitDepth::Eight);
    let mut writer = encoder.write_header()?;
    writer.write_image_data(frame.rgba())?;
    writer.finish()
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use png::Decoder;

    use super::*;
    use crate::Rect;

    #[test]
    fn encoded_png_decodes_back_to_the_same_pixels() {
        let bounds = Rect {
            x: -1,
            y: 0,
            w: 2,
            h: 1,
        };
        let rgba = vec![10, 20, 30, 255, 40, 50, 60, 255];
        let frame = Frame::new(bounds, rgba.clone()).unwrap();

        let png = encode_png(&frame).unwrap();

        let mut reader = Decoder::new(Cursor::new(png)).read_info().unwrap();
        let mut decoded = vec![0; reader.output_buffer_size().unwrap()];
        let info = reader.next_frame(&mut decoded).unwrap();
        assert_eq!((info.width, info.height), (2, 1));
        assert_eq!(decoded, rgba);
    }

    #[test]
    fn written_png_matches_the_encoded_bytes() {
        let bounds = Rect {
            x: 0,
            y: 0,
            w: 1,
            h: 1,
        };
        let frame = Frame::new(bounds, vec![1, 2, 3, 255]).unwrap();
        let mut streamed = Vec::new();

        write_png(&mut streamed, &frame).unwrap();

        assert_eq!(streamed, encode_png(&frame).unwrap());
    }
}
