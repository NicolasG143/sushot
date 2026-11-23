use png::{BitDepth, ColorType, Encoder, EncodingError};

use crate::Frame;

pub fn encode_png(frame: &Frame) -> Result<Vec<u8>, EncodingError> {
    let bounds = frame.bounds();
    let mut encoded = Vec::new();

    let mut encoder = Encoder::new(&mut encoded, bounds.w, bounds.h);
    encoder.set_color(ColorType::Rgba);
    encoder.set_depth(BitDepth::Eight);
    let mut writer = encoder.write_header()?;
    writer.write_image_data(frame.rgba())?;
    writer.finish()?;

    Ok(encoded)
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
            y: 5,
            w: 2,
            h: 2,
        };
        let rgba = vec![
            255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 10, 20, 30, 128,
        ];
        let frame = Frame::new(bounds, rgba.clone()).unwrap();

        let encoded = encode_png(&frame).unwrap();

        let mut reader = Decoder::new(Cursor::new(encoded)).read_info().unwrap();
        let mut decoded = vec![0; reader.output_buffer_size().unwrap()];
        let info = reader.next_frame(&mut decoded).unwrap();
        assert_eq!(
            (info.width, info.height, info.color_type, info.bit_depth),
            (2, 2, ColorType::Rgba, BitDepth::Eight)
        );
        assert_eq!(decoded, rgba);
    }
}
