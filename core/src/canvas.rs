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
const LOUPE_REACH: i32 = 7;
const LOUPE_ZOOM: u32 = 8;
const LOUPE_RING: u32 = 2;
const LOUPE_SIZE: u32 = (2 * LOUPE_REACH as u32 + 1) * LOUPE_ZOOM + 2 * LOUPE_RING;
const LOUPE_DARK: u32 = 0x0000_0000;
const LOUPE_MARKER: u32 = 0x00FF_FFFF;

/// XRGB pixels of the overlay in frame-local coordinates, with the origin at the
/// top-left corner of the frozen frame.
pub struct Canvas {
    width: u32,
    height: u32,
    pixels: Vec<u32>,
    dimmed: Vec<u32>,
}

impl Canvas {
    pub fn new(frame: &Frame) -> Self {
        let bounds = frame.bounds();
        let dimmed: Vec<u32> = frame
            .rgba()
            .as_chunks::<4>()
            .0
            .iter()
            .map(|pixel| xrgb(dim(pixel[0]), dim(pixel[1]), dim(pixel[2])))
            .collect();
        Canvas {
            width: bounds.w,
            height: bounds.h,
            pixels: dimmed.clone(),
            dimmed,
        }
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

    /// The square around the round magnifier, centred on `cursor`. It may reach past
    /// the canvas edges; painting clips it.
    pub fn loupe_box(&self, cursor: (i32, i32)) -> Rect {
        let half = (LOUPE_SIZE / 2) as i32;
        Rect {
            x: cursor.0 - half,
            y: cursor.1 - half,
            w: LOUPE_SIZE,
            h: LOUPE_SIZE,
        }
    }

    /// Redraws `area`: dimmed frame outside `selection`, untouched frame inside it,
    /// the selection border and its size label, then the magnifier around `loupe`,
    /// clipped to `area`.
    pub fn paint(
        &mut self,
        frame: &Frame,
        selection: Option<Rect>,
        loupe: Option<(i32, i32)>,
        area: Rect,
    ) {
        let Some(area) = self.area().intersection(area) else {
            return;
        };
        self.restore(frame, selection, area);
        if let Some(selection) = selection {
            self.draw_border(selection, BORDER_COLOR, area);
            self.draw_label(selection, area);
        }
        if let Some(cursor) = loupe {
            self.draw_loupe(frame, cursor, area);
        }
    }

    fn restore(&mut self, frame: &Frame, selection: Option<Rect>, area: Rect) {
        let width = self.width as usize;
        let columns = area.x as usize..area.x as usize + area.w as usize;
        for row in area.y as usize..area.y as usize + area.h as usize {
            let start = row * width;
            self.pixels[start + columns.start..start + columns.end]
                .copy_from_slice(&self.dimmed[start + columns.start..start + columns.end]);
        }
        let Some(bright) = selection.and_then(|selection| selection.intersection(area)) else {
            return;
        };
        let source = frame.rgba();
        let columns = bright.x as usize..bright.x as usize + bright.w as usize;
        for row in bright.y as usize..bright.y as usize + bright.h as usize {
            let start = row * width;
            let target = &mut self.pixels[start + columns.start..start + columns.end];
            let source = &source[(start + columns.start) * 4..(start + columns.end) * 4];
            for (pixel, rgba) in target.iter_mut().zip(source.as_chunks::<4>().0) {
                *pixel = xrgb(rgba[0], rgba[1], rgba[2]);
            }
        }
    }

    fn draw_border(&mut self, rect: Rect, color: u32, clip: Rect) {
        if rect.w == 0 || rect.h == 0 {
            return;
        }
        let last_x = rect.x + rect.w as i32 - 1;
        let last_y = rect.y + rect.h as i32 - 1;
        for x in rect.x..=last_x {
            self.set(x, rect.y, color, clip);
            self.set(x, last_y, color, clip);
        }
        for y in rect.y..=last_y {
            self.set(rect.x, y, color, clip);
            self.set(last_x, y, color, clip);
        }
    }

    /// Draws the original pixels around `cursor` enlarged inside a ring centred on the
    /// cursor, with the pixel under the cursor outlined. Pixels beyond the frame are
    /// drawn dark.
    fn draw_loupe(&mut self, frame: &Frame, cursor: (i32, i32), clip: Rect) {
        let loupe = self.loupe_box(cursor);
        let Some(visible) = clip.intersection(loupe) else {
            return;
        };
        let source = frame.rgba();
        let outer = LOUPE_SIZE as i32;
        let lens = (LOUPE_SIZE - 2 * LOUPE_RING) as i32;
        let source_at =
            |offset: i32| (offset - LOUPE_RING as i32).div_euclid(LOUPE_ZOOM as i32) - LOUPE_REACH;
        for y in visible.y..visible.y + visible.h as i32 {
            for x in visible.x..visible.x + visible.w as i32 {
                let dx = 2 * (x - loupe.x) + 1 - outer;
                let dy = 2 * (y - loupe.y) + 1 - outer;
                let distance = dx * dx + dy * dy;
                if distance > outer * outer {
                    continue;
                }
                let color = if distance > lens * lens {
                    BORDER_COLOR
                } else {
                    let sx = cursor.0 + source_at(x - loupe.x);
                    let sy = cursor.1 + source_at(y - loupe.y);
                    if self.area().contains(sx, sy) {
                        let index = (sy as usize * self.width as usize + sx as usize) * 4;
                        xrgb(source[index], source[index + 1], source[index + 2])
                    } else {
                        LOUPE_DARK
                    }
                };
                self.pixels[y as usize * self.width as usize + x as usize] = color;
            }
        }
        let center = Rect {
            x: loupe.x + LOUPE_RING as i32 + LOUPE_REACH * LOUPE_ZOOM as i32,
            y: loupe.y + LOUPE_RING as i32 + LOUPE_REACH * LOUPE_ZOOM as i32,
            w: LOUPE_ZOOM,
            h: LOUPE_ZOOM,
        };
        self.draw_border(center, LOUPE_MARKER, clip);
        let shadow = Rect {
            x: center.x - 1,
            y: center.y - 1,
            w: center.w + 2,
            h: center.h + 2,
        };
        self.draw_border(shadow, LOUPE_DARK, clip);
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

fn dim(channel: u8) -> u8 {
    (u32::from(channel) * DIM_NUMERATOR / DIM_DENOMINATOR) as u8
}

fn xrgb(r: u8, g: u8, b: u8) -> u32 {
    u32::from(r) << 16 | u32::from(g) << 8 | u32::from(b)
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

        canvas.paint(&frame, Some(selection()), None, canvas.area());

        assert_eq!(at(&canvas, 15, 45), SOURCE);
        assert_eq!(at(&canvas, 5, 45), DIMMED);
        assert_eq!(at(&canvas, 40, 45), DIMMED);
    }

    #[test]
    fn selection_border_is_one_pixel_wide_on_its_own_edge() {
        let frame = frame(60, 80, [200, 100, 50]);
        let mut canvas = Canvas::new(&frame);

        canvas.paint(&frame, Some(selection()), None, canvas.area());

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

        canvas.paint(&frame, Some(selection), None, canvas.area());

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
        incremental.paint(&frame, Some(first), None, incremental.area());
        let dirty = incremental
            .footprint(first)
            .union(incremental.footprint(second));
        incremental.paint(&frame, Some(second), None, dirty);

        let mut full = Canvas::new(&frame);
        full.paint(&frame, Some(second), None, full.area());

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
    const DIMMED_SPOT_BACKGROUND: u32 = 6 << 16 | 12 << 8 | 18;

    fn spot_frame() -> Frame {
        let (w, h) = (400, 300);
        let mut rgba = [10, 20, 30, 255].repeat(w * h);
        let spot = (60 * w + 50) * 4;
        rgba[spot..spot + 4].copy_from_slice(&[250, 0, 0, 255]);
        Frame::new(
            Rect {
                x: 0,
                y: 0,
                w: w as u32,
                h: h as u32,
            },
            rgba,
        )
        .unwrap()
    }

    #[test]
    fn loupe_is_centred_on_the_cursor_and_outlines_the_pixel_under_it() {
        let frame = spot_frame();
        let mut canvas = Canvas::new(&frame);

        canvas.paint(&frame, None, Some((50, 60)), canvas.area());

        let loupe = canvas.loupe_box((50, 60));
        assert_eq!(
            (loupe.x + loupe.w as i32 / 2, loupe.y + loupe.h as i32 / 2),
            (50, 60)
        );
        let center_x = (loupe.x + LOUPE_RING as i32 + LOUPE_REACH * LOUPE_ZOOM as i32) as u32;
        let center_y = (loupe.y + LOUPE_RING as i32 + LOUPE_REACH * LOUPE_ZOOM as i32) as u32;
        assert_eq!(at(&canvas, center_x, center_y), LOUPE_MARKER);
        assert_eq!(at(&canvas, center_x - 1, center_y - 1), LOUPE_DARK);
        assert_eq!(at(&canvas, 50, 60), 250 << 16);
        assert_eq!(
            at(&canvas, center_x + LOUPE_ZOOM + 3, center_y + 3),
            10 << 16 | 20 << 8 | 30
        );
    }

    #[test]
    fn loupe_is_round_with_a_ring_and_leaves_its_corners_untouched() {
        let frame = spot_frame();
        let mut canvas = Canvas::new(&frame);

        canvas.paint(&frame, None, Some((150, 150)), canvas.area());

        let loupe = canvas.loupe_box((150, 150));
        let middle = (loupe.x + loupe.w as i32 / 2) as u32;
        assert_eq!(at(&canvas, middle, loupe.y as u32), BORDER_COLOR);
        assert_eq!(
            at(&canvas, loupe.x as u32, loupe.y as u32),
            DIMMED_SPOT_BACKGROUND
        );
        let last = (loupe.x + loupe.w as i32 - 1) as u32;
        assert_eq!(at(&canvas, last, last), DIMMED_SPOT_BACKGROUND);
    }

    #[test]
    fn loupe_at_a_desktop_corner_is_clipped_to_the_canvas() {
        let frame = spot_frame();
        let mut canvas = Canvas::new(&frame);

        canvas.paint(&frame, None, Some((0, 0)), canvas.area());

        assert_eq!(at(&canvas, 0, 0), 10 << 16 | 20 << 8 | 30);
        assert_eq!(at(&canvas, 3, 3), LOUPE_MARKER);
    }

    #[test]
    fn repainting_the_old_and_new_loupe_boxes_equals_a_full_repaint() {
        let frame = spot_frame();
        let mut incremental = Canvas::new(&frame);
        incremental.paint(&frame, None, Some((40, 50)), incremental.area());
        let dirty = incremental
            .loupe_box((40, 50))
            .union(incremental.loupe_box((90, 20)));
        incremental.paint(&frame, None, Some((90, 20)), dirty);

        let mut full = Canvas::new(&frame);
        full.paint(&frame, None, Some((90, 20)), full.area());

        assert_eq!(incremental.pixels(), full.pixels());
    }
}
