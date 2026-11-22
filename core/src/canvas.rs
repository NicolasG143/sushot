use crate::glyphs::{GLYPH_HEIGHT, GLYPH_WIDTH, glyph};
use crate::{Frame, Rect};

const DIM_NUMERATOR: u32 = 3;
const DIM_DENOMINATOR: u32 = 5;
const BORDER_COLOR: u32 = 0x003C_9BFF;
const LABEL_BACKGROUND: u32 = 0x0014_1414;
const LABEL_FOREGROUND: u32 = 0x00FF_FFFF;
const LABEL_SCALE: u32 = 2;
const LABEL_PADDING: u32 = 4;
const LABEL_GAP: u32 = 4;
const GLYPH_ADVANCE: u32 = GLYPH_WIDTH + 1;

/// XRGB pixels of the overlay in frame-local coordinates, with the origin at the
/// top-left corner of the frozen frame.
pub struct Canvas {
    width: u32,
    height: u32,
    pixels: Vec<u32>,
}

impl Canvas {
    pub fn new(frame: &Frame) -> Self {
        let bounds = frame.bounds();
        let mut canvas = Canvas {
            width: bounds.w,
            height: bounds.h,
            pixels: vec![0; bounds.w as usize * bounds.h as usize],
        };
        canvas.paint(frame, None, canvas.area());
        canvas
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn pixels(&self) -> &[u32] {
        &self.pixels
    }

    pub fn area(&self) -> Rect {
        Rect {
            x: 0,
            y: 0,
            w: self.width,
            h: self.height,
        }
    }

    /// Everything `paint` can draw for `selection`: the selection and its label.
    pub fn footprint(&self, selection: Rect) -> Rect {
        selection.union(self.label_box(selection))
    }

    /// Redraws `area`: dimmed frame outside `selection`, untouched frame inside it,
    /// then the selection border and its size label, clipped to `area`.
    pub fn paint(&mut self, frame: &Frame, selection: Option<Rect>, area: Rect) {
        let Some(area) = self.area().intersection(area) else {
            return;
        };
        self.restore(frame, selection, area);
        let Some(selection) = selection else {
            return;
        };
        self.draw_border(selection, area);
        self.draw_label(selection, area);
    }

    fn restore(&mut self, frame: &Frame, selection: Option<Rect>, area: Rect) {
        let source = frame.rgba();
        for y in area.y..area.y + area.h as i32 {
            for x in area.x..area.x + area.w as i32 {
                let index = y as usize * self.width as usize + x as usize;
                let pixel = &source[index * 4..index * 4 + 3];
                let (r, g, b) = (
                    u32::from(pixel[0]),
                    u32::from(pixel[1]),
                    u32::from(pixel[2]),
                );
                self.pixels[index] = if selection.is_some_and(|inside| inside.contains(x, y)) {
                    r << 16 | g << 8 | b
                } else {
                    dim(r) << 16 | dim(g) << 8 | dim(b)
                };
            }
        }
    }

    fn draw_border(&mut self, selection: Rect, clip: Rect) {
        if selection.w == 0 || selection.h == 0 {
            return;
        }
        let last_x = selection.x + selection.w as i32 - 1;
        let last_y = selection.y + selection.h as i32 - 1;
        for x in selection.x..=last_x {
            self.set(x, selection.y, BORDER_COLOR, clip);
            self.set(x, last_y, BORDER_COLOR, clip);
        }
        for y in selection.y..=last_y {
            self.set(selection.x, y, BORDER_COLOR, clip);
            self.set(last_x, y, BORDER_COLOR, clip);
        }
    }

    fn draw_label(&mut self, selection: Rect, clip: Rect) {
        let label = self.label_box(selection);
        self.fill(label, LABEL_BACKGROUND, clip);

        let mut pen_x = label.x + LABEL_PADDING as i32;
        let pen_y = label.y + LABEL_PADDING as i32;
        for c in dimension_text(selection).chars() {
            for (row, bits) in glyph(c).iter().enumerate() {
                for column in 0..GLYPH_WIDTH {
                    if bits & (1 << (GLYPH_WIDTH - 1 - column)) != 0 {
                        let cell = Rect {
                            x: pen_x + (column * LABEL_SCALE) as i32,
                            y: pen_y + (row as u32 * LABEL_SCALE) as i32,
                            w: LABEL_SCALE,
                            h: LABEL_SCALE,
                        };
                        self.fill(cell, LABEL_FOREGROUND, clip);
                    }
                }
            }
            pen_x += (GLYPH_ADVANCE * LABEL_SCALE) as i32;
        }
    }

    /// Sits above the selection when there is room, otherwise inside its top-left corner.
    fn label_box(&self, selection: Rect) -> Rect {
        let text_width = dimension_text(selection).chars().count() as u32 * GLYPH_ADVANCE;
        let w = (text_width - 1) * LABEL_SCALE + 2 * LABEL_PADDING;
        let h = GLYPH_HEIGHT * LABEL_SCALE + 2 * LABEL_PADDING;
        let above = selection.y - (h + LABEL_GAP) as i32;
        let y = if above >= 0 {
            above
        } else {
            selection.y + LABEL_GAP as i32
        };
        let x = selection.x.min(self.width as i32 - w as i32).max(0);
        Rect { x, y, w, h }
    }

    fn fill(&mut self, area: Rect, color: u32, clip: Rect) {
        let Some(visible) = clip.intersection(area) else {
            return;
        };
        for y in visible.y..visible.y + visible.h as i32 {
            for x in visible.x..visible.x + visible.w as i32 {
                self.pixels[y as usize * self.width as usize + x as usize] = color;
            }
        }
    }

    fn set(&mut self, x: i32, y: i32, color: u32, clip: Rect) {
        if clip.contains(x, y) {
            self.pixels[y as usize * self.width as usize + x as usize] = color;
        }
    }
}

fn dim(channel: u32) -> u32 {
    channel * DIM_NUMERATOR / DIM_DENOMINATOR
}

fn dimension_text(selection: Rect) -> String {
    format!("{} × {}", selection.w, selection.h)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SOURCE: u32 = 200 << 16 | 100 << 8 | 50;
    const DIMMED: u32 = 120 << 16 | 60 << 8 | 30;

    fn frame(w: u32, h: u32, rgb: [u8; 3]) -> Frame {
        let pixel = [rgb[0], rgb[1], rgb[2], 255];
        let rgba = pixel.repeat(w as usize * h as usize);
        Frame::new(Rect { x: -5, y: -5, w, h }, rgba).unwrap()
    }

    fn at(canvas: &Canvas, x: u32, y: u32) -> u32 {
        canvas.pixels()[(y * canvas.width() + x) as usize]
    }

    fn selection() -> Rect {
        Rect {
            x: 10,
            y: 40,
            w: 20,
            h: 10,
        }
    }

    #[test]
    fn new_canvas_dims_every_pixel_to_sixty_percent() {
        let canvas = Canvas::new(&frame(4, 3, [200, 100, 50]));

        assert_eq!((canvas.width(), canvas.height()), (4, 3));
        assert!(canvas.pixels().iter().all(|&pixel| pixel == DIMMED));
    }

    #[test]
    fn selection_interior_keeps_the_original_colors_and_the_rest_stays_dim() {
        let frame = frame(60, 80, [200, 100, 50]);
        let mut canvas = Canvas::new(&frame);

        canvas.paint(&frame, Some(selection()), canvas.area());

        assert_eq!(at(&canvas, 15, 45), SOURCE);
        assert_eq!(at(&canvas, 5, 45), DIMMED);
        assert_eq!(at(&canvas, 40, 45), DIMMED);
    }

    #[test]
    fn selection_border_is_one_pixel_wide_on_its_own_edge() {
        let frame = frame(60, 80, [200, 100, 50]);
        let mut canvas = Canvas::new(&frame);

        canvas.paint(&frame, Some(selection()), canvas.area());

        assert_eq!(at(&canvas, 10, 45), BORDER_COLOR);
        assert_eq!(at(&canvas, 29, 45), BORDER_COLOR);
        assert_eq!(at(&canvas, 20, 40), BORDER_COLOR);
        assert_eq!(at(&canvas, 20, 49), BORDER_COLOR);
        assert_eq!(at(&canvas, 11, 41), SOURCE);
        assert_eq!(at(&canvas, 9, 45), DIMMED);
    }

    #[test]
    fn label_sits_above_the_selection_when_there_is_room() {
        let canvas = Canvas::new(&frame(200, 100, [0, 0, 0]));
        let selection = Rect {
            x: 20,
            y: 60,
            w: 50,
            h: 30,
        };

        let label = canvas.label_box(selection);

        assert_eq!(label.x, 20);
        assert!(label.y + label.h as i32 + LABEL_GAP as i32 <= selection.y);
    }

    #[test]
    fn label_moves_inside_the_selection_at_the_top_edge() {
        let canvas = Canvas::new(&frame(200, 100, [0, 0, 0]));
        let selection = Rect {
            x: 20,
            y: 2,
            w: 100,
            h: 60,
        };

        let label = canvas.label_box(selection);

        assert_eq!(label.y, selection.y + LABEL_GAP as i32);
    }

    #[test]
    fn label_stays_inside_the_canvas_at_the_right_edge() {
        let canvas = Canvas::new(&frame(200, 100, [0, 0, 0]));
        let selection = Rect {
            x: 190,
            y: 60,
            w: 10,
            h: 30,
        };

        let label = canvas.label_box(selection);

        assert!(label.x >= 0);
        assert!(label.x + label.w as i32 <= 200);
    }

    #[test]
    fn label_draws_the_size_in_light_pixels_on_a_dark_box() {
        let frame = frame(200, 100, [0, 0, 0]);
        let mut canvas = Canvas::new(&frame);
        let selection = Rect {
            x: 20,
            y: 60,
            w: 50,
            h: 30,
        };

        canvas.paint(&frame, Some(selection), canvas.area());

        let label = canvas.label_box(selection);
        let mut box_pixels = Vec::new();
        for y in label.y..label.y + label.h as i32 {
            for x in label.x..label.x + label.w as i32 {
                box_pixels.push(at(&canvas, x as u32, y as u32));
            }
        }
        assert!(box_pixels.contains(&LABEL_FOREGROUND));
        assert!(box_pixels.contains(&LABEL_BACKGROUND));
        assert!(
            box_pixels
                .iter()
                .all(|&pixel| pixel == LABEL_FOREGROUND || pixel == LABEL_BACKGROUND)
        );
    }

    #[test]
    fn repainting_the_union_of_old_and_new_footprints_equals_a_full_repaint() {
        let frame = frame(120, 80, [90, 160, 30]);
        let first = Rect {
            x: 10,
            y: 30,
            w: 40,
            h: 20,
        };
        let second = Rect {
            x: 30,
            y: 10,
            w: 60,
            h: 50,
        };

        let mut incremental = Canvas::new(&frame);
        incremental.paint(&frame, Some(first), incremental.area());
        let dirty = incremental
            .footprint(first)
            .union(incremental.footprint(second));
        incremental.paint(&frame, Some(second), dirty);

        let mut full = Canvas::new(&frame);
        full.paint(&frame, Some(second), full.area());

        assert_eq!(incremental.pixels(), full.pixels());
    }

    #[test]
    fn painting_outside_the_canvas_changes_nothing() {
        let frame = frame(10, 10, [10, 20, 30]);
        let mut canvas = Canvas::new(&frame);
        let before = canvas.pixels().to_vec();

        canvas.paint(
            &frame,
            None,
            Rect {
                x: 50,
                y: 50,
                w: 5,
                h: 5,
            },
        );

        assert_eq!(canvas.pixels(), before);
    }
}
