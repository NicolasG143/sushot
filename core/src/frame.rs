use std::fmt;

use crate::Rect;

const BYTES_PER_PIXEL: usize = 4;

#[derive(Debug, PartialEq, Eq)]
pub enum FrameError {
    BufferSizeMismatch { expected: usize, actual: usize },
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::BufferSizeMismatch { expected, actual } => {
                write!(
                    f,
                    "pixel buffer holds {actual} bytes but the bounds need {expected}"
                )
            }
        }
    }
}

impl std::error::Error for FrameError {}

/// RGBA8 pixels in row-major order, positioned on the virtual desktop by `bounds`.
#[derive(Debug)]
pub struct Frame {
    bounds: Rect,
    rgba: Vec<u8>,
}

impl Frame {
    pub fn new(bounds: Rect, rgba: Vec<u8>) -> Result<Self, FrameError> {
        let expected = bounds.w as usize * bounds.h as usize * BYTES_PER_PIXEL;
        if rgba.len() != expected {
            return Err(FrameError::BufferSizeMismatch {
                expected,
                actual: rgba.len(),
            });
        }
        Ok(Self { bounds, rgba })
    }

    pub fn bounds(&self) -> Rect {
        self.bounds
    }

    pub fn rgba(&self) -> &[u8] {
        &self.rgba
    }

    /// Copies the part of `area` that lies inside the frame. `area` uses the same
    /// virtual desktop coordinates as `bounds`.
    pub fn crop(&self, area: Rect) -> Option<Frame> {
        let visible = self.bounds.intersection(area)?;
        let first_column = visible.x.abs_diff(self.bounds.x) as usize;
        let first_row = visible.y.abs_diff(self.bounds.y) as usize;
        let stride = self.bounds.w as usize * BYTES_PER_PIXEL;
        let row_start = first_column * BYTES_PER_PIXEL;
        let row_end = row_start + visible.w as usize * BYTES_PER_PIXEL;

        let mut rgba = Vec::with_capacity((row_end - row_start) * visible.h as usize);
        for row in self
            .rgba
            .chunks_exact(stride)
            .skip(first_row)
            .take(visible.h as usize)
        {
            rgba.extend_from_slice(&row[row_start..row_end]);
        }

        Some(Frame {
            bounds: visible,
            rgba,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BOUNDS: Rect = Rect {
        x: -4,
        y: -2,
        w: 6,
        h: 4,
    };

    fn pixel_at(x: i32, y: i32) -> [u8; 4] {
        [(x + 100) as u8, (y + 100) as u8, 7, 255]
    }

    fn frame_with_coordinates_as_pixels() -> Frame {
        let mut rgba = Vec::new();
        for y in BOUNDS.y..BOUNDS.y + BOUNDS.h as i32 {
            for x in BOUNDS.x..BOUNDS.x + BOUNDS.w as i32 {
                rgba.extend_from_slice(&pixel_at(x, y));
            }
        }
        Frame::new(BOUNDS, rgba).unwrap()
    }

    #[test]
    fn crop_with_negative_coordinates_returns_the_matching_pixels() {
        let area = Rect {
            x: -3,
            y: -1,
            w: 2,
            h: 2,
        };

        let cropped = frame_with_coordinates_as_pixels().crop(area).unwrap();

        assert_eq!(cropped.bounds(), area);
        let expected: Vec<u8> = [
            pixel_at(-3, -1),
            pixel_at(-2, -1),
            pixel_at(-3, 0),
            pixel_at(-2, 0),
        ]
        .concat();
        assert_eq!(cropped.rgba(), expected.as_slice());
    }

    #[test]
    fn crop_spanning_the_origin_keeps_both_sides() {
        let area = Rect {
            x: -1,
            y: 0,
            w: 3,
            h: 1,
        };

        let cropped = frame_with_coordinates_as_pixels().crop(area).unwrap();

        let expected: Vec<u8> = [pixel_at(-1, 0), pixel_at(0, 0), pixel_at(1, 0)].concat();
        assert_eq!(cropped.rgba(), expected.as_slice());
    }

    #[test]
    fn crop_is_clipped_to_the_frame_bounds() {
        let area = Rect {
            x: 0,
            y: 0,
            w: 50,
            h: 50,
        };

        let cropped = frame_with_coordinates_as_pixels().crop(area).unwrap();

        assert_eq!(
            cropped.bounds(),
            Rect {
                x: 0,
                y: 0,
                w: 2,
                h: 2
            }
        );
        assert_eq!(cropped.rgba().len(), 2 * 2 * 4);
    }

    #[test]
    fn crop_outside_the_frame_is_none() {
        let area = Rect {
            x: 2,
            y: 0,
            w: 5,
            h: 5,
        };

        assert!(frame_with_coordinates_as_pixels().crop(area).is_none());
    }

    #[test]
    fn new_rejects_a_buffer_that_does_not_match_the_bounds() {
        let result = Frame::new(BOUNDS, vec![0; 10]);

        assert_eq!(
            result.unwrap_err(),
            FrameError::BufferSizeMismatch {
                expected: 6 * 4 * 4,
                actual: 10
            }
        );
    }
}
