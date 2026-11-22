/// Area in physical pixels. On the virtual desktop the origin can be negative.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
}

/// Smallest side, in pixels, a dragged region needs to count as a selection.
pub const MIN_SELECTION_SIDE: u32 = 2;

impl Rect {
    /// Rectangle spanned by two opposite corners, whichever direction was dragged.
    pub fn from_corners(first: (i32, i32), second: (i32, i32)) -> Rect {
        Rect {
            x: first.0.min(second.0),
            y: first.1.min(second.1),
            w: first.0.abs_diff(second.0),
            h: first.1.abs_diff(second.1),
        }
    }

    pub fn is_selection(self) -> bool {
        self.w >= MIN_SELECTION_SIDE && self.h >= MIN_SELECTION_SIDE
    }

    pub fn contains(self, x: i32, y: i32) -> bool {
        x >= self.x && y >= self.y && i64::from(x) < self.right() && i64::from(y) < self.bottom()
    }

    pub fn union(self, other: Rect) -> Rect {
        let left = self.x.min(other.x);
        let top = self.y.min(other.y);
        Rect {
            x: left,
            y: top,
            w: (self.right().max(other.right()) - i64::from(left)) as u32,
            h: (self.bottom().max(other.bottom()) - i64::from(top)) as u32,
        }
    }

    pub fn translate(self, dx: i32, dy: i32) -> Rect {
        Rect {
            x: self.x + dx,
            y: self.y + dy,
            ..self
        }
    }

    pub fn intersection(self, other: Rect) -> Option<Rect> {
        let left = self.x.max(other.x);
        let top = self.y.max(other.y);
        let right = self.right().min(other.right());
        let bottom = self.bottom().min(other.bottom());

        if right <= i64::from(left) || bottom <= i64::from(top) {
            return None;
        }

        Some(Rect {
            x: left,
            y: top,
            w: (right - i64::from(left)) as u32,
            h: (bottom - i64::from(top)) as u32,
        })
    }

    fn right(self) -> i64 {
        i64::from(self.x) + i64::from(self.w)
    }

    fn bottom(self) -> i64 {
        i64::from(self.y) + i64::from(self.h)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intersection_of_overlapping_rects_is_the_shared_area() {
        let left = Rect {
            x: -10,
            y: -10,
            w: 15,
            h: 15,
        };
        let right = Rect {
            x: 0,
            y: -5,
            w: 20,
            h: 5,
        };

        assert_eq!(
            left.intersection(right),
            Some(Rect {
                x: 0,
                y: -5,
                w: 5,
                h: 5
            })
        );
    }

    #[test]
    fn corners_dragged_in_any_direction_span_the_same_rect() {
        let expected = Rect {
            x: -4,
            y: 2,
            w: 10,
            h: 6,
        };

        assert_eq!(Rect::from_corners((-4, 2), (6, 8)), expected);
        assert_eq!(Rect::from_corners((6, 8), (-4, 2)), expected);
        assert_eq!(Rect::from_corners((-4, 8), (6, 2)), expected);
    }

    #[test]
    fn a_region_needs_two_pixels_on_both_sides_to_be_a_selection() {
        let rect = |w, h| Rect { x: 0, y: 0, w, h };

        assert!(rect(2, 2).is_selection());
        assert!(!rect(1, 50).is_selection());
        assert!(!rect(50, 1).is_selection());
        assert!(!rect(0, 0).is_selection());
    }

    #[test]
    fn contains_excludes_the_right_and_bottom_edges() {
        let rect = Rect {
            x: -2,
            y: 0,
            w: 4,
            h: 2,
        };

        assert!(rect.contains(-2, 0));
        assert!(rect.contains(1, 1));
        assert!(!rect.contains(2, 1));
        assert!(!rect.contains(0, 2));
        assert!(!rect.contains(-3, 0));
    }

    #[test]
    fn union_covers_both_rects() {
        let left = Rect {
            x: -10,
            y: 0,
            w: 5,
            h: 5,
        };
        let right = Rect {
            x: 3,
            y: -2,
            w: 4,
            h: 4,
        };

        assert_eq!(
            left.union(right),
            Rect {
                x: -10,
                y: -2,
                w: 17,
                h: 7
            }
        );
    }

    #[test]
    fn intersection_of_touching_rects_is_none() {
        let left = Rect {
            x: -10,
            y: 0,
            w: 10,
            h: 10,
        };
        let right = Rect {
            x: 0,
            y: 0,
            w: 10,
            h: 10,
        };

        assert_eq!(left.intersection(right), None);
    }
}
